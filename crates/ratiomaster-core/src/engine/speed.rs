/// Speed simulation with two-level random variation.
///
/// Level 1: A base speed is chosen randomly within a configured range at session start.
/// Level 2: Each announce cycle applies a small per-update variation on top of the base.
use rand::Rng;

/// Configuration for simulated transfer speed.
#[derive(Debug, Clone)]
pub struct SpeedConfig {
    /// Minimum base upload speed in bytes/sec.
    pub upload_min: u64,
    /// Maximum base upload speed in bytes/sec.
    pub upload_max: u64,
    /// Minimum base download speed in bytes/sec.
    pub download_min: u64,
    /// Maximum base download speed in bytes/sec.
    pub download_max: u64,
    /// Per-update variation range in bytes/sec (applied as +/- to the base).
    pub variation: u64,
}

impl Default for SpeedConfig {
    fn default() -> Self {
        Self {
            upload_min: 50 * 1024,   // 50 KB/s
            upload_max: 150 * 1024,  // 150 KB/s
            download_min: 50 * 1024, // 50 KB/s
            download_max: 150 * 1024,
            variation: 10 * 1024, // +/- 10 KB/s
        }
    }
}

/// Tracks the current simulated speeds for a session.
#[derive(Debug, Clone)]
pub struct SpeedState {
    /// Base upload speed selected at session start.
    pub base_upload: u64,
    /// Base download speed selected at session start.
    pub base_download: u64,
    /// Current effective upload speed (base + variation).
    pub current_upload: u64,
    /// Current effective download speed (base + variation).
    pub current_download: u64,
}

/// Selects random base speeds within the configured range (Level 1).
pub fn init_speed(config: &SpeedConfig) -> SpeedState {
    let mut rng = rand::thread_rng();
    let base_upload = rng.gen_range(config.upload_min..=config.upload_max);
    let base_download = rng.gen_range(config.download_min..=config.download_max);

    SpeedState {
        base_upload,
        base_download,
        current_upload: base_upload,
        current_download: base_download,
    }
}

/// Applies per-update random variation to the speeds (Level 2).
///
/// The effective speed is clamped to a minimum of 0.
pub fn vary_speed(state: &mut SpeedState, config: &SpeedConfig) {
    let mut rng = rand::thread_rng();

    if config.variation > 0 {
        let upload_delta = rng.gen_range(0..=config.variation * 2) as i64 - config.variation as i64;
        let download_delta =
            rng.gen_range(0..=config.variation * 2) as i64 - config.variation as i64;

        state.current_upload = (state.base_upload as i64 + upload_delta).max(0) as u64;
        state.current_download = (state.base_download as i64 + download_delta).max(0) as u64;
    }
}

/// Calculates bytes transferred during an interval at the current speed.
pub fn bytes_for_interval(speed_bytes_per_sec: u64, interval_secs: u64) -> u64 {
    speed_bytes_per_sec.saturating_mul(interval_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = SpeedConfig::default();
        assert_eq!(config.upload_min, 50 * 1024);
        assert_eq!(config.upload_max, 150 * 1024);
        assert_eq!(config.variation, 10 * 1024);
    }

    #[test]
    fn init_speed_within_range() {
        let config = SpeedConfig {
            upload_min: 100,
            upload_max: 200,
            download_min: 300,
            download_max: 400,
            variation: 0,
        };

        for _ in 0..100 {
            let state = init_speed(&config);
            assert!(state.base_upload >= 100 && state.base_upload <= 200);
            assert!(state.base_download >= 300 && state.base_download <= 400);
            assert_eq!(state.current_upload, state.base_upload);
            assert_eq!(state.current_download, state.base_download);
        }
    }

    #[test]
    fn init_speed_fixed_range() {
        let config = SpeedConfig {
            upload_min: 1000,
            upload_max: 1000,
            download_min: 2000,
            download_max: 2000,
            variation: 0,
        };

        let state = init_speed(&config);
        assert_eq!(state.base_upload, 1000);
        assert_eq!(state.base_download, 2000);
    }

    #[test]
    fn vary_speed_changes_current() {
        let config = SpeedConfig {
            upload_min: 1000,
            upload_max: 1000,
            download_min: 1000,
            download_max: 1000,
            variation: 500,
        };

        let mut state = init_speed(&config);
        let mut any_different = false;

        for _ in 0..100 {
            vary_speed(&mut state, &config);
            if state.current_upload != state.base_upload {
                any_different = true;
            }
            // Current should be within base +/- variation
            let diff = (state.current_upload as i64 - state.base_upload as i64).unsigned_abs();
            assert!(
                diff <= config.variation,
                "diff {diff} > variation {}",
                config.variation
            );
        }

        assert!(any_different, "variation should change current speed");
    }

    #[test]
    fn vary_speed_zero_variation() {
        let config = SpeedConfig {
            upload_min: 1000,
            upload_max: 1000,
            download_min: 1000,
            download_max: 1000,
            variation: 0,
        };

        let mut state = init_speed(&config);
        vary_speed(&mut state, &config);
        assert_eq!(state.current_upload, state.base_upload);
        assert_eq!(state.current_download, state.base_download);
    }

    #[test]
    fn vary_speed_no_underflow() {
        let config = SpeedConfig {
            upload_min: 0,
            upload_max: 0,
            download_min: 0,
            download_max: 0,
            variation: 1000,
        };

        let mut state = init_speed(&config);
        for _ in 0..100 {
            vary_speed(&mut state, &config);
            // Should never underflow — current_upload is u64
        }
    }

    #[test]
    fn bytes_for_interval_basic() {
        assert_eq!(bytes_for_interval(1024, 60), 61440);
    }

    #[test]
    fn bytes_for_interval_zero() {
        assert_eq!(bytes_for_interval(0, 100), 0);
        assert_eq!(bytes_for_interval(100, 0), 0);
    }

    #[test]
    fn bytes_for_interval_no_overflow() {
        // Should saturate instead of panicking
        let result = bytes_for_interval(u64::MAX, u64::MAX);
        assert_eq!(result, u64::MAX);
    }
}

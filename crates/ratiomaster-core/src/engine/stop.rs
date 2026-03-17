/// Stop conditions for the announce engine.
///
/// Determines when the engine should stop sending announces to the tracker,
/// based on upload/download amounts, time elapsed, swarm state, or ratio.
use std::time::Duration;

/// Condition under which the engine should stop.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum StopCondition {
    /// Never stop automatically.
    Never,
    /// Stop after uploading at least this many bytes.
    AfterUpload(u64),
    /// Stop after downloading at least this many bytes.
    AfterDownload(u64),
    /// Stop after this duration has elapsed.
    AfterTime(Duration),
    /// Stop when seeders reach or exceed this count.
    AfterSeeders(u32),
    /// Stop when leechers reach or exceed this count.
    AfterLeechers(u32),
    /// Stop when upload/download ratio reaches or exceeds this value.
    AfterRatio(f64),
}

/// Checks whether a stop condition has been met.
pub fn should_stop(condition: &StopCondition, state: &StopCheckState) -> bool {
    match condition {
        StopCondition::Never => false,
        StopCondition::AfterUpload(target) => state.uploaded >= *target,
        StopCondition::AfterDownload(target) => state.downloaded >= *target,
        StopCondition::AfterTime(duration) => state.elapsed >= *duration,
        StopCondition::AfterSeeders(target) => state.seeders >= *target,
        StopCondition::AfterLeechers(target) => state.leechers >= *target,
        StopCondition::AfterRatio(target) => {
            if state.downloaded == 0 {
                // If nothing downloaded, ratio is infinite (always met if target > 0)
                *target <= 0.0
            } else {
                (state.uploaded as f64 / state.downloaded as f64) >= *target
            }
        }
    }
}

/// Snapshot of engine state used for stop condition evaluation.
#[derive(Debug, Clone)]
pub struct StopCheckState {
    pub uploaded: u64,
    pub downloaded: u64,
    pub elapsed: Duration,
    pub seeders: u32,
    pub leechers: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_state() -> StopCheckState {
        StopCheckState {
            uploaded: 0,
            downloaded: 0,
            elapsed: Duration::ZERO,
            seeders: 0,
            leechers: 0,
        }
    }

    #[test]
    fn never_stops() {
        let state = StopCheckState {
            uploaded: u64::MAX,
            downloaded: u64::MAX,
            elapsed: Duration::from_secs(999999),
            seeders: u32::MAX,
            leechers: u32::MAX,
        };
        assert!(!should_stop(&StopCondition::Never, &state));
    }

    #[test]
    fn after_upload_not_met() {
        let state = StopCheckState {
            uploaded: 999,
            ..base_state()
        };
        assert!(!should_stop(&StopCondition::AfterUpload(1000), &state));
    }

    #[test]
    fn after_upload_met() {
        let state = StopCheckState {
            uploaded: 1000,
            ..base_state()
        };
        assert!(should_stop(&StopCondition::AfterUpload(1000), &state));
    }

    #[test]
    fn after_upload_exceeded() {
        let state = StopCheckState {
            uploaded: 2000,
            ..base_state()
        };
        assert!(should_stop(&StopCondition::AfterUpload(1000), &state));
    }

    #[test]
    fn after_download_met() {
        let state = StopCheckState {
            downloaded: 5000,
            ..base_state()
        };
        assert!(should_stop(&StopCondition::AfterDownload(5000), &state));
    }

    #[test]
    fn after_time_not_met() {
        let state = StopCheckState {
            elapsed: Duration::from_secs(59),
            ..base_state()
        };
        assert!(!should_stop(
            &StopCondition::AfterTime(Duration::from_secs(60)),
            &state
        ));
    }

    #[test]
    fn after_time_met() {
        let state = StopCheckState {
            elapsed: Duration::from_secs(60),
            ..base_state()
        };
        assert!(should_stop(
            &StopCondition::AfterTime(Duration::from_secs(60)),
            &state
        ));
    }

    #[test]
    fn after_seeders_met() {
        let state = StopCheckState {
            seeders: 10,
            ..base_state()
        };
        assert!(should_stop(&StopCondition::AfterSeeders(10), &state));
    }

    #[test]
    fn after_leechers_not_met() {
        let state = StopCheckState {
            leechers: 4,
            ..base_state()
        };
        assert!(!should_stop(&StopCondition::AfterLeechers(5), &state));
    }

    #[test]
    fn after_ratio_met() {
        let state = StopCheckState {
            uploaded: 2000,
            downloaded: 1000,
            ..base_state()
        };
        assert!(should_stop(&StopCondition::AfterRatio(2.0), &state));
    }

    #[test]
    fn after_ratio_not_met() {
        let state = StopCheckState {
            uploaded: 1500,
            downloaded: 1000,
            ..base_state()
        };
        assert!(!should_stop(&StopCondition::AfterRatio(2.0), &state));
    }

    #[test]
    fn after_ratio_zero_downloaded() {
        let state = StopCheckState {
            uploaded: 1000,
            downloaded: 0,
            ..base_state()
        };
        // With zero download, ratio is infinite; target > 0 is not met
        assert!(!should_stop(&StopCondition::AfterRatio(1.0), &state));
        // target <= 0 is met
        assert!(should_stop(&StopCondition::AfterRatio(0.0), &state));
    }
}

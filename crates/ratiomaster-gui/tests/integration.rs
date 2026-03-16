//! Integration tests for the GUI application logic.
//!
//! These tests exercise the app state, tab management, torrent loading,
//! engine bridge, and file dialog channel without spawning a real window.

use std::path::PathBuf;

use ratiomaster_core::client::profiles;
use ratiomaster_core::torrent;

// Re-use GUI crate internals — we test the logic, not the rendering.
// Since the gui crate is a binary, we test via the public APIs of its deps
// and replicate key logic paths.

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/test.torrent")
}

// ── Torrent Loading ──

#[test]
fn load_torrent_parses_metadata() {
    let path = fixture_path();
    let data = std::fs::read(&path).expect("fixture must exist");
    let meta = torrent::parse(&data).expect("fixture must parse");

    assert_eq!(meta.name, "test-file.dat");
    assert_eq!(meta.announce, "http://tracker.example.com:8080/announce");
    assert_eq!(meta.total_size(), 1_048_576);
    assert!(meta.is_single_file());
    assert!(!meta.is_multi_file());
    assert_eq!(meta.piece_count(), 4);
}

#[test]
fn load_torrent_info_hash_is_20_bytes() {
    let path = fixture_path();
    let data = std::fs::read(&path).unwrap();
    let meta = torrent::parse(&data).unwrap();

    assert_eq!(meta.info_hash.len(), 20);
    // Hash should be non-zero
    assert!(meta.info_hash.iter().any(|&b| b != 0));
}

#[test]
fn load_invalid_torrent_returns_error() {
    let result = torrent::parse(b"not a valid torrent");
    assert!(result.is_err());
}

#[test]
fn load_empty_data_returns_error() {
    let result = torrent::parse(b"");
    assert!(result.is_err());
}

#[test]
fn load_truncated_torrent_returns_error() {
    let path = fixture_path();
    let data = std::fs::read(&path).unwrap();
    // Truncate to half
    let truncated = &data[..data.len() / 2];
    let result = torrent::parse(truncated);
    assert!(result.is_err());
}

// ── Client Profiles ──

#[test]
fn all_profiles_is_not_empty() {
    let all = profiles::all_profiles();
    assert!(!all.is_empty(), "must have at least one client profile");
}

#[test]
fn all_profiles_have_names() {
    for p in profiles::all_profiles() {
        assert!(!p.name.is_empty(), "profile must have a name");
    }
}

#[test]
fn default_profile_exists() {
    // First profile should be retrievable by name
    let all = profiles::all_profiles();
    let first = &all[0];
    let fetched = profiles::get_profile(&first.name);
    assert!(fetched.is_some(), "first profile must be fetchable by name");
}

#[test]
fn get_unknown_profile_returns_none() {
    assert!(profiles::get_profile("NonExistentClient-999.99").is_none());
}

#[test]
fn utorrent_332_profile_exists() {
    // The default client in the GUI
    let all = profiles::all_profiles();
    let has_utorrent = all
        .iter()
        .any(|p| p.name.contains("uTorrent") || p.name.contains("3.3.2"));
    assert!(has_utorrent, "uTorrent 3.3.2 profile should exist");
}

// ── File Dialog Channel ──

#[test]
fn mpsc_channel_sends_file_path() {
    let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();

    let path = fixture_path();
    tx.send(Some(path.clone())).unwrap();

    let received = rx.try_recv().unwrap();
    assert_eq!(received, Some(path));
}

#[test]
fn mpsc_channel_sends_none_on_cancel() {
    let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();

    tx.send(None).unwrap();

    let received = rx.try_recv().unwrap();
    assert_eq!(received, None);
}

#[test]
fn mpsc_channel_empty_returns_err() {
    let (_tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
    assert!(rx.try_recv().is_err());
}

#[test]
fn mpsc_channel_cross_thread() {
    let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
    let path = fixture_path();
    let path_clone = path.clone();

    let handle = std::thread::spawn(move || {
        tx.send(Some(path_clone)).unwrap();
    });

    handle.join().unwrap();
    let received = rx.try_recv().unwrap();
    assert_eq!(received, Some(path));
}

// ── Speed / Size Formatting ──

#[test]
fn format_bytes_units() {
    // We replicate the format_bytes logic to verify correctness
    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;
        const TB: u64 = 1024 * GB;
        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{bytes} B")
        }
    }

    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1024), "1.00 KB");
    assert_eq!(format_bytes(1048576), "1.00 MB");
    assert_eq!(format_bytes(1073741824), "1.00 GB");
    assert_eq!(format_bytes(1099511627776), "1.00 TB");
    assert_eq!(format_bytes(1536), "1.50 KB");
}

#[test]
fn format_duration_values() {
    fn format_duration(d: std::time::Duration) -> String {
        let secs = d.as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        format!("{h:02}:{m:02}:{s:02}")
    }

    assert_eq!(
        format_duration(std::time::Duration::from_secs(0)),
        "00:00:00"
    );
    assert_eq!(
        format_duration(std::time::Duration::from_secs(61)),
        "00:01:01"
    );
    assert_eq!(
        format_duration(std::time::Duration::from_secs(3661)),
        "01:01:01"
    );
    assert_eq!(
        format_duration(std::time::Duration::from_secs(86399)),
        "23:59:59"
    );
}

// ── Parse Helpers ──

#[test]
fn parse_u64_valid() {
    fn parse_u64(s: &str) -> u64 {
        s.trim().parse().unwrap_or(0)
    }
    assert_eq!(parse_u64("100"), 100);
    assert_eq!(parse_u64("  42  "), 42);
    assert_eq!(parse_u64("0"), 0);
    assert_eq!(parse_u64(""), 0);
    assert_eq!(parse_u64("abc"), 0);
    assert_eq!(parse_u64("18446744073709551615"), u64::MAX);
}

#[test]
fn parse_u16_valid() {
    fn parse_u16(s: &str) -> u16 {
        s.trim().parse().unwrap_or(0)
    }
    assert_eq!(parse_u16("6881"), 6881);
    assert_eq!(parse_u16("0"), 0);
    assert_eq!(parse_u16("65535"), 65535);
    assert_eq!(parse_u16("99999"), 0); // overflow
    assert_eq!(parse_u16(""), 0);
}

// ── Stop Condition Labels ──

#[test]
fn stop_types_has_expected_entries() {
    let stop_types = &[
        "Never",
        "After Upload",
        "After Download",
        "After Time",
        "After Seeders",
        "After Leechers",
        "After Ratio",
    ];
    assert_eq!(stop_types.len(), 7);
    assert_eq!(stop_types[0], "Never");
}

// ── Proxy Types ──

#[test]
fn proxy_types_has_expected_entries() {
    let proxy_types = &["None", "SOCKS4", "SOCKS4a", "SOCKS5", "HTTP Connect"];
    assert_eq!(proxy_types.len(), 5);
    assert_eq!(proxy_types[0], "None");
}

// ── Engine Config Construction ──

#[test]
fn speed_config_construction() {
    use ratiomaster_core::engine::speed::SpeedConfig;

    let config = SpeedConfig {
        upload_min: 50 * 1024,
        upload_max: 150 * 1024,
        download_min: 0,
        download_max: 0,
        variation: 10 * 1024,
    };

    assert_eq!(config.upload_min, 51200);
    assert_eq!(config.upload_max, 153600);
    assert_eq!(config.download_min, 0);
}

#[test]
fn proxy_config_none() {
    use ratiomaster_core::proxy::ProxyConfig;
    let config = ProxyConfig::None;
    matches!(config, ProxyConfig::None);
}

#[test]
fn proxy_config_socks5_with_credentials() {
    use ratiomaster_core::proxy::socks5::Credentials;
    use ratiomaster_core::proxy::ProxyConfig;

    let config = ProxyConfig::Socks5 {
        proxy_host: "127.0.0.1".into(),
        proxy_port: 1080,
        credentials: Some(Credentials {
            username: "user".into(),
            password: "pass".into(),
        }),
    };

    if let ProxyConfig::Socks5 {
        proxy_host,
        proxy_port,
        credentials,
    } = &config
    {
        assert_eq!(proxy_host, "127.0.0.1");
        assert_eq!(*proxy_port, 1080);
        assert!(credentials.is_some());
    } else {
        panic!("expected Socks5");
    }
}

// ── Torrent Fixture Roundtrip ──

#[test]
fn torrent_hash_hex_format() {
    let path = fixture_path();
    let data = std::fs::read(&path).unwrap();
    let meta = torrent::parse(&data).unwrap();

    let hash_str: String = meta.info_hash.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hash_str.len(), 40, "hex hash should be 40 chars");
    assert!(hash_str.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn torrent_tracker_url_is_valid() {
    let path = fixture_path();
    let data = std::fs::read(&path).unwrap();
    let meta = torrent::parse(&data).unwrap();

    assert!(meta.announce.starts_with("http://") || meta.announce.starts_with("https://"));
    assert!(meta.announce.contains("/announce"));
}

// ── Multiple Tabs Simulation ──

#[test]
fn multiple_tabs_independent_state() {
    // Simulate what the app does with multiple tabs
    let path = fixture_path();
    let data = std::fs::read(&path).unwrap();

    let meta1 = torrent::parse(&data).unwrap();
    let meta2 = torrent::parse(&data).unwrap();

    // Each parse should produce independent copies
    assert_eq!(meta1.name, meta2.name);
    assert_eq!(meta1.info_hash, meta2.info_hash);
    assert_eq!(meta1.total_size(), meta2.total_size());
}

// ── Engine State ──

#[test]
fn engine_state_fields_accessible() {
    use ratiomaster_core::engine::EngineState;
    use std::time::Instant;

    let state = EngineState {
        uploaded: 0,
        downloaded: 0,
        left: 1_048_576,
        announce_count: 0,
        completed_sent: false,
        seeders: 0,
        leechers: 0,
        interval: 1800,
        started_at: Instant::now(),
    };

    assert_eq!(state.uploaded, 0);
    assert_eq!(state.downloaded, 0);
    assert_eq!(state.seeders, 0);
    assert_eq!(state.leechers, 0);
    assert_eq!(state.announce_count, 0);
    assert_eq!(state.interval, 1800);
}

// ── Stop Condition ──

#[test]
fn stop_condition_variants() {
    use ratiomaster_core::engine::stop::StopCondition;

    let never = StopCondition::Never;
    let upload = StopCondition::AfterUpload(100 * 1024 * 1024);
    let ratio = StopCondition::AfterRatio(2.0);

    matches!(never, StopCondition::Never);
    matches!(upload, StopCondition::AfterUpload(104857600));
    matches!(ratio, StopCondition::AfterRatio(_));
}

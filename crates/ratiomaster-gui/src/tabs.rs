#![allow(dead_code)]

use std::path::PathBuf;
use std::time::Instant;

use ratiomaster_core::client::profiles;
use ratiomaster_core::client::ClientFamily;
use ratiomaster_core::engine::EngineState;
use tokio::sync::{mpsc, watch};

/// Stop type options.
pub const STOP_TYPES: &[&str] = &[
    "Never",
    "After Upload",
    "After Download",
    "After Time",
    "After Seeders",
    "After Leechers",
    "After Ratio",
];

/// Proxy type options.
pub const PROXY_TYPES: &[&str] = &["None", "SOCKS4", "SOCKS4a", "SOCKS5", "HTTP Connect"];

/// Status of a torrent tab's engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabStatus {
    Idle,
    Running,
    Stopped,
    #[allow(dead_code)]
    Error(String),
}

/// Engine control handles for communicating with the async engine.
pub struct EngineHandles {
    pub shutdown_tx: watch::Sender<bool>,
    pub force_announce_tx: mpsc::Sender<()>,
    pub state_rx: watch::Receiver<EngineState>,
}

/// Log entry with timestamp.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
}

/// Client family display name and associated profile names.
pub struct ClientFamilyGroup {
    pub label: String,
    pub profiles: Vec<String>,
}

/// Build grouped client families from the profile registry.
pub fn build_client_families() -> Vec<ClientFamilyGroup> {
    let all = profiles::all_profiles();
    let mut groups: Vec<ClientFamilyGroup> = Vec::new();

    for p in all {
        let label = family_display_name(p.family);
        if let Some(g) = groups.iter_mut().find(|g| g.label == label) {
            g.profiles.push(p.name.clone());
        } else {
            groups.push(ClientFamilyGroup {
                label,
                profiles: vec![p.name.clone()],
            });
        }
    }

    groups
}

fn family_display_name(family: ClientFamily) -> String {
    match family {
        ClientFamily::UTorrent => "uTorrent",
        ClientFamily::BitComet => "BitComet",
        ClientFamily::Vuze => "Vuze",
        ClientFamily::Azureus => "Azureus",
        ClientFamily::BitTorrent => "BitTorrent",
        ClientFamily::Transmission => "Transmission",
        ClientFamily::ABC => "ABC",
        ClientFamily::BitLord => "BitLord",
        ClientFamily::BTuga => "BTuga",
        ClientFamily::BitTornado => "BitTornado",
        ClientFamily::Burst => "Burst",
        ClientFamily::BitTyrant => "BitTyrant",
        ClientFamily::BitSpirit => "BitSpirit",
        ClientFamily::KTorrent => "KTorrent",
        ClientFamily::Deluge => "Deluge",
        ClientFamily::GnomeBT => "Gnome BT",
    }
    .into()
}

/// A single torrent tab with all editable fields and engine state.
pub struct TorrentTab {
    pub name: String,
    pub torrent_path: Option<PathBuf>,
    pub torrent_data: Option<Vec<u8>>,
    pub torrent_name: String,
    pub tracker_url: String,
    pub info_hash: String,
    pub total_size: u64,
    pub torrent_size_override: String,

    // Client & version
    pub client_names: Vec<String>,
    pub selected_client: usize,
    pub port: String,
    pub interval: String,
    pub file_size_downloaded: String,

    // Stop condition
    pub stop_type: usize,
    pub stop_value: String,

    // Speed
    pub upload_speed: String,
    pub download_speed: String,
    pub upload_random: bool,
    pub upload_random_min: String,
    pub upload_random_max: String,
    pub download_random: bool,
    pub download_random_min: String,
    pub download_random_max: String,

    // Random on next update
    pub next_upload_random: bool,
    pub next_upload_random_min: String,
    pub next_upload_random_max: String,
    pub next_download_random: bool,
    pub next_download_random_min: String,
    pub next_download_random_max: String,

    // Custom values
    pub generate_new_values: bool,
    pub custom_port: String,
    pub custom_numwant: String,
    pub custom_peer_id: String,
    pub custom_key: String,

    // Proxy
    pub proxy_type: usize,
    pub proxy_host: String,
    pub proxy_port: String,
    pub proxy_user: String,
    pub proxy_pass: String,

    // Options
    pub tcp_listener: bool,
    pub scrape: bool,
    pub ignore_failure: bool,

    // Log
    pub log_enabled: bool,
    pub log_entries: Vec<LogEntry>,
    pub log_filter: String,
    pub log_auto_scroll: bool,

    // Engine state
    pub status: TabStatus,
    pub uploaded: u64,
    pub downloaded: u64,
    pub seeders: u32,
    pub leechers: u32,
    pub announce_count: u32,
    pub engine_interval: u64,
    pub started_at: Option<Instant>,
    pub handles: Option<EngineHandles>,
}

impl TorrentTab {
    pub fn new(name: String) -> Self {
        let client_names: Vec<String> = profiles::all_profiles()
            .iter()
            .map(|p| p.name.clone())
            .collect();

        Self {
            name,
            torrent_path: None,
            torrent_data: None,
            torrent_name: String::new(),
            tracker_url: String::new(),
            info_hash: String::new(),
            total_size: 0,
            torrent_size_override: String::new(),

            client_names,
            selected_client: 0,
            port: "6881".into(),
            interval: "1800".into(),
            file_size_downloaded: "0".into(),

            stop_type: 0,
            stop_value: String::new(),

            upload_speed: "100".into(),
            download_speed: "0".into(),
            upload_random: false,
            upload_random_min: "50".into(),
            upload_random_max: "150".into(),
            download_random: false,
            download_random_min: "0".into(),
            download_random_max: "0".into(),

            next_upload_random: false,
            next_upload_random_min: "50".into(),
            next_upload_random_max: "150".into(),
            next_download_random: false,
            next_download_random_min: "0".into(),
            next_download_random_max: "0".into(),

            generate_new_values: false,
            custom_port: String::new(),
            custom_numwant: String::new(),
            custom_peer_id: String::new(),
            custom_key: String::new(),

            proxy_type: 0,
            proxy_host: String::new(),
            proxy_port: String::new(),
            proxy_user: String::new(),
            proxy_pass: String::new(),

            tcp_listener: false,
            scrape: false,
            ignore_failure: false,

            log_enabled: true,
            log_entries: Vec::new(),
            log_filter: String::new(),
            log_auto_scroll: true,

            status: TabStatus::Idle,
            uploaded: 0,
            downloaded: 0,
            seeders: 0,
            leechers: 0,
            announce_count: 0,
            engine_interval: 1800,
            started_at: None,
            handles: None,
        }
    }

    pub fn set_defaults(&mut self) {
        self.upload_speed = "100".into();
        self.download_speed = "0".into();
        self.upload_random = false;
        self.upload_random_min = "50".into();
        self.upload_random_max = "150".into();
        self.download_random = false;
        self.download_random_min = "0".into();
        self.download_random_max = "0".into();
        self.interval = "1800".into();
        self.port = "6881".into();
        self.stop_type = 0;
        self.stop_value.clear();
        self.proxy_type = 0;
        self.proxy_host.clear();
        self.proxy_port.clear();
        self.proxy_user.clear();
        self.proxy_pass.clear();
        self.tcp_listener = false;
        self.scrape = false;
        self.ignore_failure = false;
        self.custom_port.clear();
        self.custom_numwant.clear();
        self.custom_peer_id.clear();
        self.custom_key.clear();
        self.generate_new_values = false;
        self.file_size_downloaded = "0".into();
        self.next_upload_random = false;
        self.next_download_random = false;
    }

    pub fn add_log(&mut self, message: String) {
        if !self.log_enabled {
            return;
        }
        let now = chrono_now();
        self.log_entries.push(LogEntry {
            timestamp: now,
            message,
        });
        if self.log_entries.len() > 5000 {
            self.log_entries.drain(0..self.log_entries.len() - 5000);
        }
    }

    pub fn poll_engine_state(&mut self) {
        if let Some(ref mut handles) = self.handles {
            if handles.state_rx.has_changed().unwrap_or(false) {
                let state = handles.state_rx.borrow_and_update().clone();
                self.uploaded = state.uploaded;
                self.downloaded = state.downloaded;
                self.seeders = state.seeders;
                self.leechers = state.leechers;
                self.announce_count = state.announce_count;
                self.engine_interval = state.interval;
            }
        }
    }

    pub fn ratio(&self) -> f64 {
        if self.total_size == 0 {
            return 0.0;
        }
        self.uploaded as f64 / self.total_size as f64
    }

    pub fn running_time(&self) -> Option<std::time::Duration> {
        self.started_at.map(|s| s.elapsed())
    }

    pub fn next_announce_secs(&self) -> Option<u64> {
        if self.status != TabStatus::Running {
            return None;
        }
        let started = self.started_at?;
        let elapsed = started.elapsed().as_secs();
        let total_intervals = self.announce_count as u64 * self.engine_interval;
        let next_at = total_intervals + self.engine_interval;
        Some(next_at.saturating_sub(elapsed))
    }

    pub fn is_running(&self) -> bool {
        self.status == TabStatus::Running
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now % 86400) / 3600;
    let minutes = (now % 3600) / 60;
    let seconds = now % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

/// Format bytes to human-readable string.
pub fn format_bytes(bytes: u64) -> String {
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

/// Format a duration as HH:MM:SS.
pub fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Parse a string as u64, defaulting to 0.
pub fn parse_u64(s: &str) -> u64 {
    s.trim().parse().unwrap_or(0)
}

/// Parse a string as u16, defaulting to 0.
pub fn parse_u16(s: &str) -> u16 {
    s.trim().parse().unwrap_or(0)
}

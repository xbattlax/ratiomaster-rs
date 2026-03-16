/// Application state and event handling for the TUI.
use std::path::PathBuf;
use std::time::Instant;

use tokio::sync::{mpsc, watch};

use ratiomaster_core::client::profiles;
use ratiomaster_core::config::{self, AppConfig};
use ratiomaster_core::engine::speed::SpeedConfig;
use ratiomaster_core::engine::stop::StopCondition;
use ratiomaster_core::engine::{Engine, EngineConfig, EngineState};
use ratiomaster_core::proxy::ProxyConfig;
use ratiomaster_core::torrent;

use crate::browser::FileBrowser;
use crate::dropdown::Dropdown;
use crate::input::TextInput;

/// Focusable fields in the settings area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusableField {
    Client,
    Port,
    UploadSpeed,
    UploadRandomEnabled,
    UploadRandomMin,
    UploadRandomMax,
    DownloadSpeed,
    DownloadRandomEnabled,
    DownloadRandomMin,
    DownloadRandomMax,
    Interval,
    TcpListener,
    Scrape,
    StopType,
    StopValue,
    ProxyType,
    ProxyHost,
    ProxyPort,
    ProxyUser,
    ProxyPass,
    CustomPeerId,
    CustomKey,
    IgnoreFailure,
}

/// Ordered list of all focusable fields for Tab/Shift+Tab cycling.
const FIELD_ORDER: &[FocusableField] = &[
    FocusableField::Client,
    FocusableField::Port,
    FocusableField::UploadSpeed,
    FocusableField::UploadRandomEnabled,
    FocusableField::UploadRandomMin,
    FocusableField::UploadRandomMax,
    FocusableField::DownloadSpeed,
    FocusableField::DownloadRandomEnabled,
    FocusableField::DownloadRandomMin,
    FocusableField::DownloadRandomMax,
    FocusableField::Interval,
    FocusableField::TcpListener,
    FocusableField::Scrape,
    FocusableField::StopType,
    FocusableField::StopValue,
    FocusableField::ProxyType,
    FocusableField::ProxyHost,
    FocusableField::ProxyPort,
    FocusableField::ProxyUser,
    FocusableField::ProxyPass,
    FocusableField::CustomPeerId,
    FocusableField::CustomKey,
    FocusableField::IgnoreFailure,
];

/// Application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    FileBrowser,
    LogFilter,
    Editing,
    DropdownOpen,
    HelpPopup,
    QuitConfirm,
    TabRename,
}

/// Stop type options for the dropdown.
pub const STOP_TYPES: &[&str] = &[
    "Never",
    "After Upload",
    "After Download",
    "After Time",
    "After Seeders",
    "After Leechers",
    "After Ratio",
];

/// Proxy type options for the dropdown.
pub const PROXY_TYPES: &[&str] = &["None", "SOCKS4", "SOCKS4a", "SOCKS5", "HTTP Connect"];

/// Status of a torrent tab's engine.
#[derive(Debug, Clone)]
pub enum TabStatus {
    Idle,
    Running,
    Stopped,
    #[allow(dead_code)]
    Error(String),
}

/// Engine control handles.
pub struct EngineHandles {
    pub shutdown_tx: watch::Sender<bool>,
    pub force_announce_tx: mpsc::Sender<()>,
    pub state_rx: watch::Receiver<EngineState>,
}

/// A single torrent tab with editable fields.
pub struct TorrentTab {
    pub name: String,
    pub torrent_path: Option<PathBuf>,
    pub torrent_name: String,
    pub tracker_url: String,
    pub info_hash: String,
    pub total_size: u64,
    pub piece_length: u64,
    pub piece_count: u64,

    // Editable fields
    pub client_dropdown: Dropdown,
    pub port: TextInput,
    pub upload_speed: TextInput,
    pub upload_random_enabled: bool,
    pub upload_random_min: TextInput,
    pub upload_random_max: TextInput,
    pub download_speed: TextInput,
    pub download_random_enabled: bool,
    pub download_random_min: TextInput,
    pub download_random_max: TextInput,
    pub interval: TextInput,
    pub tcp_listener: bool,
    pub scrape: bool,
    pub stop_dropdown: Dropdown,
    pub stop_value: TextInput,
    pub proxy_dropdown: Dropdown,
    pub proxy_host: TextInput,
    pub proxy_port: TextInput,
    pub proxy_user: TextInput,
    pub proxy_pass: TextInput,
    pub custom_peer_id: TextInput,
    pub custom_key: TextInput,
    pub ignore_failure: bool,

    // Engine state
    pub status: TabStatus,
    pub uploaded: u64,
    pub downloaded: u64,
    pub seeders: u32,
    pub leechers: u32,
    pub announce_count: u32,
    pub started_at: Option<Instant>,
    pub handles: Option<EngineHandles>,
}

/// Log entry with timestamp.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
}

/// Main application state.
pub struct App {
    pub tabs: Vec<TorrentTab>,
    pub active_tab: usize,
    pub mode: AppMode,
    pub focused_field: Option<FocusableField>,
    pub log_entries: Vec<LogEntry>,
    pub log_filter: String,
    pub log_scroll: usize,
    pub should_quit: bool,
    pub file_browser: FileBrowser,
    pub config: AppConfig,
    pub minimized: bool,
    pub tab_rename_input: TextInput,
}

/// Build the client name list from all_profiles().
fn client_names() -> Vec<String> {
    profiles::all_profiles()
        .iter()
        .map(|p| p.name.clone())
        .collect()
}

impl App {
    /// Creates a new application with default state.
    pub fn new() -> Self {
        let config = config::load_config();
        let mut app = Self {
            tabs: Vec::new(),
            active_tab: 0,
            mode: AppMode::Normal,
            focused_field: None,
            log_entries: Vec::new(),
            log_filter: String::new(),
            log_scroll: 0,
            should_quit: false,
            file_browser: FileBrowser::new(),
            config,
            minimized: false,
            tab_rename_input: TextInput::new(String::new()),
        };
        app.add_empty_tab();
        app
    }

    /// Adds an empty tab with default settings from config.
    pub fn add_empty_tab(&mut self) {
        let tab_num = self.tabs.len() + 1;
        let clients = client_names();

        self.tabs.push(TorrentTab {
            name: format!("Tab {tab_num}"),
            torrent_path: None,
            torrent_name: String::new(),
            tracker_url: String::new(),
            info_hash: String::new(),
            total_size: 0,
            piece_length: 0,
            piece_count: 0,
            client_dropdown: Dropdown::new(clients, &self.config.general.default_client),
            port: TextInput::new("6881".into()),
            upload_speed: TextInput::from_u64(self.config.general.default_upload_speed),
            upload_random_enabled: self.config.upload.random_enabled,
            upload_random_min: TextInput::from_u64(self.config.upload.random_min),
            upload_random_max: TextInput::from_u64(self.config.upload.random_max),
            download_speed: TextInput::from_u64(self.config.general.default_download_speed),
            download_random_enabled: self.config.download.random_enabled,
            download_random_min: TextInput::from_u64(self.config.download.random_min),
            download_random_max: TextInput::from_u64(self.config.download.random_max),
            interval: TextInput::from_u64(self.config.general.default_interval),
            tcp_listener: self.config.general.tcp_listener,
            scrape: self.config.general.scrape_enabled,
            stop_dropdown: Dropdown::new(
                STOP_TYPES.iter().map(|s| (*s).to_string()).collect(),
                "Never",
            ),
            stop_value: TextInput::new(String::new()),
            proxy_dropdown: Dropdown::new(
                PROXY_TYPES.iter().map(|s| (*s).to_string()).collect(),
                "None",
            ),
            proxy_host: TextInput::new(String::new()),
            proxy_port: TextInput::new(String::new()),
            proxy_user: TextInput::new(String::new()),
            proxy_pass: TextInput::new(String::new()),
            custom_peer_id: TextInput::new(String::new()),
            custom_key: TextInput::new(String::new()),
            ignore_failure: self.config.general.ignore_failure_reason,
            status: TabStatus::Idle,
            uploaded: 0,
            downloaded: 0,
            seeders: 0,
            leechers: 0,
            announce_count: 0,
            started_at: None,
            handles: None,
        });
        self.active_tab = self.tabs.len() - 1;
    }

    /// Loads a torrent file into the active tab.
    pub fn load_torrent(&mut self, path: PathBuf) {
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                self.add_log(format!("Failed to read {}: {e}", path.display()));
                return;
            }
        };

        let meta = match torrent::parse(&data) {
            Ok(m) => m,
            Err(e) => {
                self.add_log(format!("Failed to parse torrent: {e}"));
                return;
            }
        };

        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            let hash_str = meta
                .info_hash
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();

            let total = meta.total_size();
            let piece_count = if meta.piece_length > 0 {
                total.div_ceil(meta.piece_length)
            } else {
                0
            };

            tab.name = meta.name.clone();
            tab.torrent_path = Some(path);
            tab.torrent_name = meta.name.clone();
            tab.tracker_url = meta.announce.clone();
            tab.info_hash = hash_str;
            tab.total_size = total;
            tab.piece_length = meta.piece_length;
            tab.piece_count = piece_count;

            self.add_log(format!(
                "Loaded torrent: {} ({})",
                meta.name,
                format_bytes(meta.total_size())
            ));
        }
    }

    /// Starts the engine for the active tab.
    pub fn start_engine(&mut self) {
        let tab = match self.tabs.get_mut(self.active_tab) {
            Some(t) => t,
            None => return,
        };

        if matches!(tab.status, TabStatus::Running) {
            self.add_log("Engine already running".into());
            return;
        }

        let torrent_path = match tab.torrent_path.as_ref() {
            Some(p) => p.clone(),
            None => {
                self.add_log("No torrent loaded".into());
                return;
            }
        };

        let data = match std::fs::read(&torrent_path) {
            Ok(d) => d,
            Err(e) => {
                self.add_log(format!("Failed to read torrent: {e}"));
                return;
            }
        };

        let torrent_meta = match torrent::parse(&data) {
            Ok(m) => m,
            Err(e) => {
                self.add_log(format!("Failed to parse torrent: {e}"));
                return;
            }
        };

        let client_name = tab.client_dropdown.current().to_string();
        let profile = match profiles::get_profile(&client_name) {
            Some(p) => p.clone(),
            None => {
                self.add_log(format!("Unknown client: {client_name}"));
                return;
            }
        };

        let tab = self.tabs.get_mut(self.active_tab).unwrap();

        let stop_condition = match tab.stop_dropdown.current() {
            "After Upload" => StopCondition::AfterUpload(tab.stop_value.as_u64() * 1024 * 1024),
            "After Download" => StopCondition::AfterDownload(tab.stop_value.as_u64() * 1024 * 1024),
            "After Time" => {
                StopCondition::AfterTime(std::time::Duration::from_secs(tab.stop_value.as_u64()))
            }
            "After Seeders" => {
                StopCondition::AfterSeeders(tab.stop_value.value.parse().unwrap_or(0))
            }
            "After Leechers" => {
                StopCondition::AfterLeechers(tab.stop_value.value.parse().unwrap_or(0))
            }
            "After Ratio" => StopCondition::AfterRatio(tab.stop_value.value.parse().unwrap_or(0.0)),
            _ => StopCondition::Never,
        };

        let upload_speed = tab.upload_speed.as_u64();
        let download_speed = tab.download_speed.as_u64();
        let upload_max = if tab.upload_random_enabled {
            tab.upload_random_max.as_u64()
        } else {
            upload_speed
        };
        let download_max = if tab.download_random_enabled {
            tab.download_random_max.as_u64()
        } else {
            download_speed
        };

        let engine_config = EngineConfig {
            port: tab.port.as_u16(),
            speed: SpeedConfig {
                upload_min: upload_speed * 1024,
                upload_max: upload_max * 1024,
                download_min: download_speed * 1024,
                download_max: download_max * 1024,
                variation: 10 * 1024,
            },
            stop_condition,
            ignore_failure: tab.ignore_failure,
            max_retries: 5,
            initial_downloaded_percent: 0,
            http_timeout: std::time::Duration::from_secs(30),
        };

        let proxy = build_proxy_config(tab);

        let mut engine = Engine::new(torrent_meta, profile, proxy, engine_config);
        let state_rx = engine.subscribe_state();
        let shutdown_tx = engine.shutdown_handle();
        let force_announce_tx = engine.force_announce_handle();

        tab.handles = Some(EngineHandles {
            shutdown_tx,
            force_announce_tx,
            state_rx,
        });
        tab.status = TabStatus::Running;
        tab.started_at = Some(Instant::now());

        self.add_log("Engine started".into());

        let tab_idx = self.active_tab;
        tokio::spawn(async move {
            if let Err(e) = engine.run().await {
                tracing::error!("engine {tab_idx} error: {e}");
            }
        });
    }

    /// Stops the engine for the active tab.
    pub fn stop_engine(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            if let Some(ref handles) = tab.handles {
                let _ = handles.shutdown_tx.send(true);
                tab.status = TabStatus::Stopped;
                self.add_log("Engine stopped".into());
            }
        }
    }

    /// Forces an immediate announce on the active tab.
    pub fn force_announce(&self) {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            if let Some(ref handles) = tab.handles {
                let tx = handles.force_announce_tx.clone();
                tokio::spawn(async move {
                    let _ = tx.send(()).await;
                });
            }
        }
    }

    /// Updates tab stats from engine state watches.
    pub fn poll_engine_states(&mut self) {
        for tab in &mut self.tabs {
            if let Some(ref mut handles) = tab.handles {
                if handles.state_rx.has_changed().unwrap_or(false) {
                    let state = handles.state_rx.borrow_and_update().clone();
                    tab.uploaded = state.uploaded;
                    tab.downloaded = state.downloaded;
                    tab.seeders = state.seeders;
                    tab.leechers = state.leechers;
                    tab.announce_count = state.announce_count;
                    tab.interval.set(state.interval.to_string());
                }
            }
        }
    }

    /// Returns true if any engine is currently running.
    pub fn any_engine_running(&self) -> bool {
        self.tabs
            .iter()
            .any(|t| matches!(t.status, TabStatus::Running))
    }

    /// Adds a log entry.
    pub fn add_log(&mut self, message: String) {
        let now = chrono_now();
        self.log_entries.push(LogEntry {
            timestamp: now,
            message,
        });
        let max = self.config.ui.log_max_lines;
        if self.log_entries.len() > max {
            self.log_entries.drain(0..self.log_entries.len() - max);
        }
        self.log_scroll = self.filtered_log_len().saturating_sub(1);
    }

    /// Returns filtered log entries.
    pub fn filtered_logs(&self) -> Vec<&LogEntry> {
        if self.log_filter.is_empty() {
            self.log_entries.iter().collect()
        } else {
            let filter = self.log_filter.to_lowercase();
            self.log_entries
                .iter()
                .filter(|e| e.message.to_lowercase().contains(&filter))
                .collect()
        }
    }

    /// Returns the number of filtered log entries.
    pub fn filtered_log_len(&self) -> usize {
        self.filtered_logs().len()
    }

    // -- Focus management --

    /// Focus the next field in order.
    pub fn focus_next(&mut self) {
        let visible = self.visible_fields();
        if visible.is_empty() {
            return;
        }
        match self.focused_field {
            None => self.focused_field = Some(visible[0]),
            Some(current) => {
                if let Some(pos) = visible.iter().position(|f| *f == current) {
                    let next = (pos + 1) % visible.len();
                    self.focused_field = Some(visible[next]);
                } else {
                    self.focused_field = Some(visible[0]);
                }
            }
        }
    }

    /// Focus the previous field in order.
    pub fn focus_prev(&mut self) {
        let visible = self.visible_fields();
        if visible.is_empty() {
            return;
        }
        match self.focused_field {
            None => self.focused_field = Some(*visible.last().unwrap()),
            Some(current) => {
                if let Some(pos) = visible.iter().position(|f| *f == current) {
                    let prev = if pos == 0 { visible.len() - 1 } else { pos - 1 };
                    self.focused_field = Some(visible[prev]);
                } else {
                    self.focused_field = Some(*visible.last().unwrap());
                }
            }
        }
    }

    /// Unfocus all fields.
    pub fn unfocus(&mut self) {
        self.focused_field = None;
        self.mode = AppMode::Normal;
    }

    /// Returns the list of currently visible/applicable fields.
    fn visible_fields(&self) -> Vec<FocusableField> {
        let tab = match self.tabs.get(self.active_tab) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut fields: Vec<FocusableField> = Vec::new();
        for &field in FIELD_ORDER {
            match field {
                // Proxy detail fields only visible when proxy is not None
                FocusableField::ProxyHost
                | FocusableField::ProxyPort
                | FocusableField::ProxyUser
                | FocusableField::ProxyPass => {
                    if tab.proxy_dropdown.current() != "None" {
                        fields.push(field);
                    }
                }
                // Upload random min/max only visible when random enabled
                FocusableField::UploadRandomMin | FocusableField::UploadRandomMax => {
                    if tab.upload_random_enabled {
                        fields.push(field);
                    }
                }
                // Download random min/max only visible when random enabled
                FocusableField::DownloadRandomMin | FocusableField::DownloadRandomMax => {
                    if tab.download_random_enabled {
                        fields.push(field);
                    }
                }
                // Stop value only visible when stop type is not Never
                FocusableField::StopValue => {
                    if tab.stop_dropdown.current() != "Never" {
                        fields.push(field);
                    }
                }
                _ => fields.push(field),
            }
        }
        fields
    }

    /// Returns true if the currently focused field is a text input.
    pub fn is_text_field_focused(&self) -> bool {
        matches!(
            self.focused_field,
            Some(
                FocusableField::Port
                    | FocusableField::UploadSpeed
                    | FocusableField::UploadRandomMin
                    | FocusableField::UploadRandomMax
                    | FocusableField::DownloadSpeed
                    | FocusableField::DownloadRandomMin
                    | FocusableField::DownloadRandomMax
                    | FocusableField::Interval
                    | FocusableField::StopValue
                    | FocusableField::ProxyHost
                    | FocusableField::ProxyPort
                    | FocusableField::ProxyUser
                    | FocusableField::ProxyPass
                    | FocusableField::CustomPeerId
                    | FocusableField::CustomKey
            )
        )
    }

    /// Returns true if the currently focused field is a checkbox.
    pub fn is_checkbox_focused(&self) -> bool {
        matches!(
            self.focused_field,
            Some(
                FocusableField::TcpListener
                    | FocusableField::Scrape
                    | FocusableField::UploadRandomEnabled
                    | FocusableField::DownloadRandomEnabled
                    | FocusableField::IgnoreFailure
            )
        )
    }

    /// Returns true if the currently focused field is a dropdown.
    pub fn is_dropdown_focused(&self) -> bool {
        matches!(
            self.focused_field,
            Some(FocusableField::Client | FocusableField::StopType | FocusableField::ProxyType)
        )
    }

    /// Toggle the checkbox for the currently focused field.
    pub fn toggle_focused_checkbox(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            match self.focused_field {
                Some(FocusableField::TcpListener) => tab.tcp_listener = !tab.tcp_listener,
                Some(FocusableField::Scrape) => tab.scrape = !tab.scrape,
                Some(FocusableField::UploadRandomEnabled) => {
                    tab.upload_random_enabled = !tab.upload_random_enabled;
                }
                Some(FocusableField::DownloadRandomEnabled) => {
                    tab.download_random_enabled = !tab.download_random_enabled;
                }
                Some(FocusableField::IgnoreFailure) => {
                    tab.ignore_failure = !tab.ignore_failure;
                }
                _ => {}
            }
        }
    }

    /// Get a mutable reference to the text input for the focused field.
    pub fn focused_text_input(&mut self) -> Option<&mut TextInput> {
        let field = self.focused_field?;
        let tab = self.tabs.get_mut(self.active_tab)?;
        match field {
            FocusableField::Port => Some(&mut tab.port),
            FocusableField::UploadSpeed => Some(&mut tab.upload_speed),
            FocusableField::UploadRandomMin => Some(&mut tab.upload_random_min),
            FocusableField::UploadRandomMax => Some(&mut tab.upload_random_max),
            FocusableField::DownloadSpeed => Some(&mut tab.download_speed),
            FocusableField::DownloadRandomMin => Some(&mut tab.download_random_min),
            FocusableField::DownloadRandomMax => Some(&mut tab.download_random_max),
            FocusableField::Interval => Some(&mut tab.interval),
            FocusableField::StopValue => Some(&mut tab.stop_value),
            FocusableField::ProxyHost => Some(&mut tab.proxy_host),
            FocusableField::ProxyPort => Some(&mut tab.proxy_port),
            FocusableField::ProxyUser => Some(&mut tab.proxy_user),
            FocusableField::ProxyPass => Some(&mut tab.proxy_pass),
            FocusableField::CustomPeerId => Some(&mut tab.custom_peer_id),
            FocusableField::CustomKey => Some(&mut tab.custom_key),
            _ => None,
        }
    }

    /// Open the dropdown for the currently focused field.
    pub fn open_focused_dropdown(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            match self.focused_field {
                Some(FocusableField::Client) => tab.client_dropdown.open(),
                Some(FocusableField::StopType) => tab.stop_dropdown.open(),
                Some(FocusableField::ProxyType) => tab.proxy_dropdown.open(),
                _ => return,
            }
        }
        self.mode = AppMode::DropdownOpen;
    }

    /// Navigate the currently open dropdown up.
    pub fn dropdown_up(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            match self.focused_field {
                Some(FocusableField::Client) => tab.client_dropdown.up(),
                Some(FocusableField::StopType) => tab.stop_dropdown.up(),
                Some(FocusableField::ProxyType) => tab.proxy_dropdown.up(),
                _ => {}
            }
        }
    }

    /// Navigate the currently open dropdown down.
    pub fn dropdown_down(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            match self.focused_field {
                Some(FocusableField::Client) => tab.client_dropdown.down(),
                Some(FocusableField::StopType) => tab.stop_dropdown.down(),
                Some(FocusableField::ProxyType) => tab.proxy_dropdown.down(),
                _ => {}
            }
        }
    }

    /// Confirm the currently open dropdown selection.
    pub fn dropdown_confirm(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            match self.focused_field {
                Some(FocusableField::Client) => {
                    tab.client_dropdown.confirm();
                }
                Some(FocusableField::StopType) => {
                    tab.stop_dropdown.confirm();
                }
                Some(FocusableField::ProxyType) => {
                    tab.proxy_dropdown.confirm();
                }
                _ => {}
            }
        }
        self.mode = AppMode::Normal;
    }

    /// Close the currently open dropdown without confirming.
    pub fn dropdown_cancel(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            match self.focused_field {
                Some(FocusableField::Client) => tab.client_dropdown.close(),
                Some(FocusableField::StopType) => tab.stop_dropdown.close(),
                Some(FocusableField::ProxyType) => tab.proxy_dropdown.close(),
                _ => {}
            }
        }
        self.mode = AppMode::Normal;
    }

    // -- Tab management --

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    pub fn close_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        self.stop_engine();
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn move_tab_left(&mut self) {
        if self.active_tab > 0 {
            self.tabs.swap(self.active_tab, self.active_tab - 1);
            self.active_tab -= 1;
        }
    }

    pub fn move_tab_right(&mut self) {
        if self.active_tab < self.tabs.len() - 1 {
            self.tabs.swap(self.active_tab, self.active_tab + 1);
            self.active_tab += 1;
        }
    }

    /// Saves the current log to a file.
    pub fn save_log(&self) {
        let path = config::config_dir().join("ratiomaster.log");
        let content: String = self
            .log_entries
            .iter()
            .map(|e| format!("[{}] {}", e.timestamp, e.message))
            .collect::<Vec<_>>()
            .join("\n");
        if let Err(e) = std::fs::write(&path, content) {
            tracing::error!("failed to save log: {e}");
        }
    }

    /// Clears the log.
    pub fn clear_log(&mut self) {
        self.log_entries.clear();
        self.log_scroll = 0;
    }
}

/// Build ProxyConfig from the current tab's proxy settings.
fn build_proxy_config(tab: &TorrentTab) -> ProxyConfig {
    match tab.proxy_dropdown.current() {
        "SOCKS4" => ProxyConfig::Socks4 {
            proxy_host: tab.proxy_host.value.clone(),
            proxy_port: tab.proxy_port.as_u16(),
            user_id: tab.proxy_user.value.clone(),
        },
        "SOCKS4a" => ProxyConfig::Socks4a {
            proxy_host: tab.proxy_host.value.clone(),
            proxy_port: tab.proxy_port.as_u16(),
            user_id: tab.proxy_user.value.clone(),
        },
        "SOCKS5" => {
            let credentials = if tab.proxy_user.value.is_empty() {
                None
            } else {
                Some(ratiomaster_core::proxy::socks5::Credentials {
                    username: tab.proxy_user.value.clone(),
                    password: tab.proxy_pass.value.clone(),
                })
            };
            ProxyConfig::Socks5 {
                proxy_host: tab.proxy_host.value.clone(),
                proxy_port: tab.proxy_port.as_u16(),
                credentials,
            }
        }
        "HTTP Connect" => {
            let credentials = if tab.proxy_user.value.is_empty() {
                None
            } else {
                Some(ratiomaster_core::proxy::http::Credentials {
                    username: tab.proxy_user.value.clone(),
                    password: tab.proxy_pass.value.clone(),
                })
            };
            ProxyConfig::HttpConnect {
                proxy_host: tab.proxy_host.value.clone(),
                proxy_port: tab.proxy_port.as_u16(),
                credentials,
            }
        }
        _ => ProxyConfig::None,
    }
}

/// Returns the current time as HH:MM:SS.
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

/// Formats a byte count to human-readable.
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

/// Formats a duration as HH:MM:SS.
pub fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

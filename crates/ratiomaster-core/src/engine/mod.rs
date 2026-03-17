/// Core announce engine for tracker communication.
///
/// Manages the lifecycle of tracker announces: start -> periodic announce -> stop.
/// Handles speed simulation, stop conditions, failure retry, seed mode,
/// and manual announce triggers.
pub mod batch;
pub mod listener;
pub mod speed;
pub mod stop;
pub mod system_info;

use std::time::{Duration, Instant};

use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::client::generator;
use crate::client::ClientProfile;
use crate::network::http::HttpVersion;
use crate::proxy::ProxyConfig;
use crate::torrent::TorrentMetainfo;
use crate::tracker::client::{HttpTrackerClient, TrackerClient};
use crate::tracker::{announce, response};

use speed::{SpeedConfig, SpeedState};
use stop::{StopCheckState, StopCondition};

/// Configuration for the announce engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Port to report to the tracker.
    pub port: u16,
    /// Speed simulation settings.
    pub speed: SpeedConfig,
    /// When to stop automatically.
    pub stop_condition: StopCondition,
    /// Whether to ignore tracker failure responses and continue.
    pub ignore_failure: bool,
    /// Maximum retry attempts per announce on connection errors.
    pub max_retries: u32,
    /// Initial downloaded percentage (0-100). 100 = seed mode.
    pub initial_downloaded_percent: u8,
    /// HTTP request timeout.
    pub http_timeout: Duration,
    /// Bind address for the TCP handshake listener.
    pub bind_address: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            port: 6881,
            speed: SpeedConfig::default(),
            stop_condition: StopCondition::Never,
            ignore_failure: false,
            max_retries: 5,
            initial_downloaded_percent: 0,
            http_timeout: Duration::from_secs(30),
            bind_address: "127.0.0.1".into(),
        }
    }
}

/// Mutable state tracked across announces.
#[derive(Debug, Clone)]
pub struct EngineState {
    /// Total bytes reported as uploaded.
    pub uploaded: u64,
    /// Total bytes reported as downloaded.
    pub downloaded: u64,
    /// Bytes remaining.
    pub left: u64,
    /// Number of successful announces.
    pub announce_count: u32,
    /// Whether "completed" event has been sent.
    pub completed_sent: bool,
    /// Last known seeder count.
    pub seeders: u32,
    /// Last known leecher count.
    pub leechers: u32,
    /// Current announce interval from tracker.
    pub interval: u64,
    /// Session start time.
    pub started_at: Instant,
}

/// Result of a single announce.
#[derive(Debug)]
pub struct AnnounceResult {
    /// Tracker-specified interval for next announce.
    pub interval: u64,
    /// Number of seeders.
    pub seeders: Option<u64>,
    /// Number of leechers.
    pub leechers: Option<u64>,
    /// Warning message from tracker.
    pub warning: Option<String>,
}

/// The core announce engine.
pub struct Engine {
    torrent: TorrentMetainfo,
    profile: ClientProfile,
    config: EngineConfig,
    state: EngineState,
    peer_id: [u8; 20],
    key: String,
    speed_state: SpeedState,
    force_announce_rx: mpsc::Receiver<()>,
    force_announce_tx: mpsc::Sender<()>,
    pub shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    state_watch_tx: watch::Sender<EngineState>,
    local_ip: String,
    tracker_client: Box<dyn TrackerClient>,
}

impl Engine {
    /// Creates a new engine for the given torrent and client profile.
    pub fn new(
        torrent: TorrentMetainfo,
        profile: ClientProfile,
        proxy: ProxyConfig,
        config: EngineConfig,
    ) -> Self {
        let tracker_client = Box::new(HttpTrackerClient::new(proxy, config.http_timeout));
        Self::new_with_client(torrent, profile, config, tracker_client)
    }

    /// Creates a new engine with a custom tracker client (for testing or custom transports).
    pub fn new_with_client(
        torrent: TorrentMetainfo,
        profile: ClientProfile,
        config: EngineConfig,
        tracker_client: Box<dyn TrackerClient>,
    ) -> Self {
        let total_size = torrent.total_size();
        let initial_downloaded = total_size * config.initial_downloaded_percent as u64 / 100;
        let left = total_size.saturating_sub(initial_downloaded);

        let peer_id = generator::generate_peer_id(&profile);
        let key = generator::generate_key(&profile);
        let speed_state = speed::init_speed(&config.speed);

        let (force_tx, force_rx) = mpsc::channel(1);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let initial_state = EngineState {
            uploaded: 0,
            downloaded: initial_downloaded,
            left,
            announce_count: 0,
            completed_sent: left == 0,
            seeders: 0,
            leechers: 0,
            interval: 1800,
            started_at: Instant::now(),
        };

        let (state_watch_tx, _) = watch::channel(initial_state.clone());

        let local_ip = crate::network::local_ip::detect_local_ip().to_string();

        Self {
            torrent,
            profile,
            config,
            state: initial_state,
            peer_id,
            key,
            speed_state,
            force_announce_rx: force_rx,
            force_announce_tx: force_tx,
            shutdown_tx,
            shutdown_rx,
            state_watch_tx,
            local_ip,
            tracker_client,
        }
    }

    /// Returns a handle for triggering manual announces.
    pub fn force_announce_handle(&self) -> mpsc::Sender<()> {
        self.force_announce_tx.clone()
    }

    /// Returns a handle for shutting down the engine.
    pub fn shutdown_handle(&self) -> watch::Sender<bool> {
        self.shutdown_tx.clone()
    }

    /// Returns a clone of the shutdown receiver (for the handshake listener).
    pub fn shutdown_receiver(&self) -> watch::Receiver<bool> {
        self.shutdown_rx.clone()
    }

    /// Returns the current engine state.
    pub fn state(&self) -> &EngineState {
        &self.state
    }

    /// Returns the peer ID.
    pub fn peer_id(&self) -> &[u8; 20] {
        &self.peer_id
    }

    /// Returns the torrent metadata.
    pub fn torrent(&self) -> &TorrentMetainfo {
        &self.torrent
    }

    /// Returns the client profile.
    pub fn profile(&self) -> &ClientProfile {
        &self.profile
    }

    /// Returns a watch receiver for engine state updates.
    ///
    /// The state is published after each announce and state change.
    pub fn subscribe_state(&self) -> watch::Receiver<EngineState> {
        self.state_watch_tx.subscribe()
    }

    /// Sends the "started" event to the tracker.
    pub async fn start(&mut self) -> Result<AnnounceResult, EngineError> {
        info!("sending started event to tracker");
        self.do_announce("started").await
    }

    /// Sends a regular announce (no event).
    pub async fn announce(&mut self) -> Result<AnnounceResult, EngineError> {
        self.do_announce("").await
    }

    /// Sends the "stopped" event to the tracker.
    pub async fn stop(&mut self) -> Result<AnnounceResult, EngineError> {
        info!("sending stopped event to tracker");
        let _ = self.shutdown_tx.send(true);
        self.do_announce("stopped").await
    }

    /// Forces an immediate announce, resetting the interval timer.
    pub async fn force_announce(&mut self) -> Result<AnnounceResult, EngineError> {
        info!("manual announce triggered");
        self.do_announce("").await
    }

    /// Runs the main engine loop: start -> announce every interval -> check stop -> stop.
    pub async fn run(&mut self) -> Result<(), EngineError> {
        system_info::log_system_info();

        // Start the handshake listener
        let listener_addr = listener::start_listener(
            &self.config.bind_address,
            self.config.port,
            self.torrent.info_hash,
            self.peer_id,
            self.shutdown_rx.clone(),
        )
        .await
        .map_err(EngineError::Io)?;
        info!("handshake listener on {listener_addr}");

        // Send started event (seed mode sends completed immediately)
        let start_result = self.start().await?;
        self.state.interval = start_result.interval;
        self.state.announce_count += 1;
        let _ = self.state_watch_tx.send(self.state.clone());

        if self.state.left == 0 && !self.state.completed_sent {
            info!("seed mode: sending completed event");
            let completed_result = self.do_announce("completed").await?;
            self.state.interval = completed_result.interval;
            self.state.announce_count += 1;
            self.state.completed_sent = true;
            let _ = self.state_watch_tx.send(self.state.clone());
        }

        let mut secs_since_announce = 0u64;

        loop {
            // Tick every second for smooth speed simulation
            let mut force_announce = false;

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                _ = self.force_announce_rx.recv() => {
                    debug!("force announce received");
                    force_announce = true;
                }
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        break;
                    }
                }
            }

            if *self.shutdown_rx.borrow() {
                break;
            }

            secs_since_announce += 1;

            // Simulate transfer each second
            speed::vary_speed(&mut self.speed_state, &self.config.speed);
            let upload_bytes = speed::bytes_for_interval(self.speed_state.current_upload, 1);
            let download_bytes = speed::bytes_for_interval(self.speed_state.current_download, 1);

            self.state.uploaded = self.state.uploaded.saturating_add(upload_bytes);

            if self.state.left > 0 {
                let actual_download = download_bytes.min(self.state.left);
                self.state.downloaded = self.state.downloaded.saturating_add(actual_download);
                self.state.left = self.state.left.saturating_sub(actual_download);
            }

            // Publish state every second for live GUI updates
            let _ = self.state_watch_tx.send(self.state.clone());

            // Announce at interval or on force
            let should_announce = force_announce || secs_since_announce >= self.state.interval;

            if should_announce {
                secs_since_announce = 0;

                // Check if download completed
                let event = if self.state.left == 0 && !self.state.completed_sent {
                    self.state.completed_sent = true;
                    info!("download complete, sending completed event");
                    "completed"
                } else {
                    ""
                };

                // Announce with retry
                match self.do_announce_with_retry(event).await {
                    Ok(result) => {
                        self.state.interval = result.interval;
                        self.state.announce_count += 1;
                    }
                    Err(e) => {
                        error!("announce failed after retries: {e}");
                        if !force_announce && !self.config.ignore_failure {
                            self.stop().await.ok();
                            return Err(e);
                        }
                        // Force announces and ignore_failure mode: log and continue
                        warn!("continuing despite announce failure");
                    }
                }
            }

            // Check stop condition
            let check_state = StopCheckState {
                uploaded: self.state.uploaded,
                downloaded: self.state.downloaded,
                elapsed: self.state.started_at.elapsed(),
                seeders: self.state.seeders,
                leechers: self.state.leechers,
            };

            if stop::should_stop(&self.config.stop_condition, &check_state) {
                info!("stop condition met");
                break;
            }
        }

        self.stop().await.ok();
        info!("engine stopped");
        Ok(())
    }

    async fn do_announce_with_retry(&mut self, event: &str) -> Result<AnnounceResult, EngineError> {
        let mut last_err = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_secs(2u64.pow(attempt.min(5)));
                warn!(
                    "retry {attempt}/{} after {backoff:?}",
                    self.config.max_retries
                );
                tokio::time::sleep(backoff).await;
            }

            match self.do_announce(event).await {
                Ok(result) => return Ok(result),
                Err(EngineError::TrackerFailure(ref reason)) if !self.config.ignore_failure => {
                    error!("tracker failure: {reason}");
                    return Err(EngineError::TrackerFailure(reason.clone()));
                }
                Err(e) => {
                    warn!("announce attempt {attempt} failed: {e}");
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or(EngineError::MaxRetriesExceeded))
    }

    async fn do_announce(&mut self, event: &str) -> Result<AnnounceResult, EngineError> {
        let local_ip = self.local_ip.clone();

        let event_param = if event.is_empty() {
            String::new()
        } else {
            format!("&event={event}")
        };

        let params = announce::AnnounceParams {
            info_hash: self.torrent.info_hash,
            peer_id: self.peer_id,
            port: self.config.port,
            uploaded: self.state.uploaded,
            downloaded: self.state.downloaded,
            left: self.state.left,
            numwant: self.profile.default_numwant as u32,
            key: self.key.clone(),
            event: event_param,
            local_ip,
        };

        let url = announce::build_announce_url(
            &self.torrent.announce,
            &self.profile.query_template,
            &params,
            self.profile.hash_uppercase,
        );

        let headers = announce::build_headers(
            &self.profile.headers_template,
            &params,
            self.profile.hash_uppercase,
        );

        let http_version = if self.profile.http_protocol == "HTTP/1.0" {
            HttpVersion::Http10
        } else {
            HttpVersion::Http11
        };

        debug!("announce URL: {url}");

        let resp = self
            .tracker_client
            .announce(&url, &headers, http_version)
            .await
            .map_err(EngineError::Http)?;

        let tracker_resp = response::parse(&resp.body).map_err(|e| match e {
            response::TrackerResponseError::TrackerFailure(reason) => {
                EngineError::TrackerFailure(reason)
            }
            other => EngineError::TrackerResponse(other),
        })?;

        if let Some(complete) = tracker_resp.complete {
            self.state.seeders = complete as u32;
        }
        if let Some(incomplete) = tracker_resp.incomplete {
            self.state.leechers = incomplete as u32;
        }

        info!(
            "announce ok: interval={}, seeders={}, leechers={}, uploaded={}, downloaded={}, left={}",
            tracker_resp.interval,
            self.state.seeders,
            self.state.leechers,
            self.state.uploaded,
            self.state.downloaded,
            self.state.left,
        );

        // Publish state update
        let _ = self.state_watch_tx.send(self.state.clone());

        Ok(AnnounceResult {
            interval: tracker_resp.interval,
            seeders: tracker_resp.complete,
            leechers: tracker_resp.incomplete,
            warning: tracker_resp.warning_message,
        })
    }
}

/// Errors from the announce engine.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EngineError {
    /// HTTP communication error.
    #[error("http error: {0}")]
    Http(#[from] crate::network::http::HttpError),

    /// Tracker returned a failure reason.
    #[error("tracker failure: {0}")]
    TrackerFailure(String),

    /// Tracker response parsing error.
    #[error("tracker response error: {0}")]
    TrackerResponse(#[from] crate::tracker::response::TrackerResponseError),

    /// I/O error (e.g., listener binding).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Maximum retry attempts exceeded.
    #[error("maximum retry attempts exceeded")]
    MaxRetriesExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::profiles;

    fn test_torrent() -> TorrentMetainfo {
        TorrentMetainfo {
            announce: "http://tracker.test/announce".into(),
            announce_list: None,
            name: "test.txt".into(),
            piece_length: 262144,
            pieces: vec![0u8; 20],
            length: Some(1_000_000),
            files: None,
            comment: None,
            created_by: None,
            creation_date: None,
            info_hash: [0x42u8; 20],
        }
    }

    #[test]
    fn engine_initial_state() {
        let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
        let engine = Engine::new(
            test_torrent(),
            profile,
            ProxyConfig::None,
            EngineConfig::default(),
        );

        assert_eq!(engine.state.uploaded, 0);
        assert_eq!(engine.state.downloaded, 0);
        assert_eq!(engine.state.left, 1_000_000);
        assert!(!engine.state.completed_sent);
    }

    #[test]
    fn engine_seed_mode() {
        let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
        let config = EngineConfig {
            initial_downloaded_percent: 100,
            ..EngineConfig::default()
        };

        let engine = Engine::new(test_torrent(), profile, ProxyConfig::None, config);

        assert_eq!(engine.state.downloaded, 1_000_000);
        assert_eq!(engine.state.left, 0);
        assert!(engine.state.completed_sent);
    }

    #[test]
    fn engine_partial_download() {
        let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
        let config = EngineConfig {
            initial_downloaded_percent: 50,
            ..EngineConfig::default()
        };

        let engine = Engine::new(test_torrent(), profile, ProxyConfig::None, config);

        assert_eq!(engine.state.downloaded, 500_000);
        assert_eq!(engine.state.left, 500_000);
        assert!(!engine.state.completed_sent);
    }

    #[test]
    fn engine_peer_id_matches_profile() {
        let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
        let engine = Engine::new(
            test_torrent(),
            profile,
            ProxyConfig::None,
            EngineConfig::default(),
        );

        assert_eq!(&engine.peer_id[..8], b"-UT3320-");
    }

    #[test]
    fn engine_key_matches_profile_format() {
        let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
        let engine = Engine::new(
            test_torrent(),
            profile,
            ProxyConfig::None,
            EngineConfig::default(),
        );

        assert_eq!(engine.key.len(), 8);
        for ch in engine.key.chars() {
            assert!("0123456789abcdef".contains(ch));
        }
    }

    #[test]
    fn engine_default_config() {
        let config = EngineConfig::default();
        assert_eq!(config.port, 6881);
        assert_eq!(config.max_retries, 5);
        assert!(!config.ignore_failure);
        assert_eq!(config.initial_downloaded_percent, 0);
        assert_eq!(config.stop_condition, StopCondition::Never);
    }

    #[test]
    fn force_announce_handle_clone() {
        let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
        let engine = Engine::new(
            test_torrent(),
            profile,
            ProxyConfig::None,
            EngineConfig::default(),
        );
        let handle = engine.force_announce_handle();
        // Should be sendable
        assert!(!handle.is_closed());
    }
}

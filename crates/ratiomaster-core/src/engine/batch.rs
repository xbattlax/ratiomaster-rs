/// Batch operations for managing multiple Engine instances.
///
/// Allows starting, stopping, and updating multiple torrents concurrently.
use tokio::sync::{mpsc, watch};
use tracing::{error, info};

use crate::client::ClientProfile;
use crate::proxy::ProxyConfig;
use crate::torrent::TorrentMetainfo;

use super::{Engine, EngineConfig, EngineError};

/// An entry in the batch engine.
struct BatchEntry {
    engine: Option<Engine>,
    handle: Option<tokio::task::JoinHandle<Result<(), EngineError>>>,
    shutdown_tx: watch::Sender<bool>,
    force_announce_tx: mpsc::Sender<()>,
}

/// Manages multiple `Engine` instances for batch torrent operations.
pub struct BatchEngine {
    entries: Vec<BatchEntry>,
}

impl BatchEngine {
    /// Creates a new empty batch engine.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Adds a torrent to the batch.
    pub fn add(
        &mut self,
        torrent: TorrentMetainfo,
        profile: ClientProfile,
        proxy: ProxyConfig,
        config: EngineConfig,
    ) {
        let engine = Engine::new(torrent, profile, proxy, config);
        let shutdown_tx = engine.shutdown_handle();
        let force_announce_tx = engine.force_announce_handle();
        self.entries.push(BatchEntry {
            engine: Some(engine),
            handle: None,
            shutdown_tx,
            force_announce_tx,
        });
    }

    /// Returns the number of torrents in the batch.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the batch has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Starts all engines concurrently.
    ///
    /// Each engine runs in its own spawned task.
    pub fn start_all(&mut self) {
        info!("starting {} engines", self.entries.len());

        for (i, entry) in self.entries.iter_mut().enumerate() {
            if entry.handle.is_some() {
                continue; // already running
            }

            let Some(mut engine) = entry.engine.take() else {
                continue; // already taken
            };

            let handle = tokio::spawn(async move {
                let result = engine.run().await;
                if let Err(ref e) = result {
                    error!("engine {i} error: {e}");
                }
                result
            });

            entry.handle = Some(handle);
        }
    }

    /// Signals all engines to stop.
    pub fn stop_all(&self) {
        info!("stopping {} engines", self.entries.len());
        for entry in &self.entries {
            let _ = entry.shutdown_tx.send(true);
        }
    }

    /// Waits for all engines to complete and returns their results.
    pub async fn join_all(&mut self) -> Vec<Result<(), EngineError>> {
        let mut results = Vec::with_capacity(self.entries.len());

        for entry in &mut self.entries {
            if let Some(handle) = entry.handle.take() {
                match handle.await {
                    Ok(result) => results.push(result),
                    Err(e) => results.push(Err(EngineError::Io(std::io::Error::other(format!(
                        "task panicked: {e}"
                    ))))),
                }
            }
        }

        results
    }

    /// Triggers a force-announce on all engines.
    pub async fn force_announce_all(&self) {
        for entry in &self.entries {
            let _ = entry.force_announce_tx.send(()).await;
        }
    }
}

impl Default for BatchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::profiles;

    fn test_torrent(name: &str) -> TorrentMetainfo {
        TorrentMetainfo {
            announce: "http://tracker.test/announce".into(),
            announce_list: None,
            name: name.into(),
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
    fn batch_new_empty() {
        let batch = BatchEngine::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn batch_add_entries() {
        let mut batch = BatchEngine::new();
        let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();

        batch.add(
            test_torrent("torrent1"),
            profile.clone(),
            ProxyConfig::None,
            EngineConfig::default(),
        );
        batch.add(
            test_torrent("torrent2"),
            profile,
            ProxyConfig::None,
            EngineConfig::default(),
        );

        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
    }

    #[test]
    fn batch_default() {
        let batch = BatchEngine::default();
        assert!(batch.is_empty());
    }

    #[test]
    fn batch_stop_all_no_panic() {
        let mut batch = BatchEngine::new();
        let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();

        batch.add(
            test_torrent("torrent1"),
            profile,
            ProxyConfig::None,
            EngineConfig::default(),
        );

        // Stopping without starting should not panic
        batch.stop_all();
    }
}

/// Session save/load for persisting engine state across restarts.
///
/// Sessions are stored as JSON files in the config directory.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ConfigError;

/// Saved session state for a single torrent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Path to the .torrent file.
    pub torrent_path: String,

    /// Bytes reported as uploaded.
    pub uploaded: u64,

    /// Bytes reported as downloaded.
    pub downloaded: u64,

    /// Bytes remaining.
    pub left: u64,

    /// Client profile name.
    pub client_name: String,

    /// Port reported to tracker.
    pub port: u16,

    /// Upload speed in KB/s.
    pub upload_speed: u64,

    /// Download speed in KB/s.
    pub download_speed: u64,

    /// Announce interval in seconds.
    pub interval: u64,

    /// Whether TCP listener was enabled.
    pub tcp_listener: bool,

    /// Whether scrape was enabled.
    pub scrape_enabled: bool,

    /// Tracker URL override, if any.
    pub tracker_override: Option<String>,
}

/// Returns the sessions directory.
pub fn sessions_dir() -> PathBuf {
    super::config_dir().join("sessions")
}

/// Saves a session to a JSON file.
///
/// The filename is derived from the torrent path.
pub fn save_session(session: &Session) -> Result<PathBuf, ConfigError> {
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir).map_err(ConfigError::Io)?;

    let filename = session_filename(&session.torrent_path);
    let path = dir.join(filename);

    let json = serde_json::to_string_pretty(session).map_err(ConfigError::Json)?;
    std::fs::write(&path, json).map_err(ConfigError::Io)?;

    Ok(path)
}

/// Loads a session from a JSON file by torrent path.
pub fn load_session_for(torrent_path: &str) -> Result<Session, ConfigError> {
    let filename = session_filename(torrent_path);
    let path = sessions_dir().join(filename);
    load_session_from(&path)
}

/// Loads a session from a specific file path.
pub fn load_session_from(path: &Path) -> Result<Session, ConfigError> {
    let contents = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
    serde_json::from_str(&contents).map_err(ConfigError::Json)
}

/// Lists all saved sessions.
pub fn list_sessions() -> Result<Vec<Session>, ConfigError> {
    let dir = sessions_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(ConfigError::Io)?;

    for entry in entries {
        let entry = entry.map_err(ConfigError::Io)?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            if let Ok(session) = load_session_from(&path) {
                sessions.push(session);
            }
        }
    }

    Ok(sessions)
}

/// Deletes a saved session by torrent path.
pub fn delete_session(torrent_path: &str) -> Result<(), ConfigError> {
    let filename = session_filename(torrent_path);
    let path = sessions_dir().join(filename);
    if path.exists() {
        std::fs::remove_file(&path).map_err(ConfigError::Io)?;
    }
    Ok(())
}

/// Generates a filename from a torrent path by hashing it.
fn session_filename(torrent_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    torrent_path.hash(&mut hasher);
    format!("{:016x}.json", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session() -> Session {
        Session {
            torrent_path: "/tmp/test.torrent".into(),
            uploaded: 1_000_000,
            downloaded: 500_000,
            left: 500_000,
            client_name: "uTorrent 3.3.2".into(),
            port: 6881,
            upload_speed: 100,
            download_speed: 50,
            interval: 1800,
            tcp_listener: true,
            scrape_enabled: true,
            tracker_override: None,
        }
    }

    #[test]
    fn session_json_roundtrip() {
        let session = test_session();
        let json = serde_json::to_string_pretty(&session).unwrap();
        let parsed: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.torrent_path, session.torrent_path);
        assert_eq!(parsed.uploaded, session.uploaded);
        assert_eq!(parsed.client_name, session.client_name);
    }

    #[test]
    fn session_filename_deterministic() {
        let f1 = session_filename("/tmp/test.torrent");
        let f2 = session_filename("/tmp/test.torrent");
        assert_eq!(f1, f2);
        assert!(f1.ends_with(".json"));
    }

    #[test]
    fn session_filename_different_paths() {
        let f1 = session_filename("/tmp/a.torrent");
        let f2 = session_filename("/tmp/b.torrent");
        assert_ne!(f1, f2);
    }

    #[test]
    fn sessions_dir_path() {
        let dir = sessions_dir();
        assert!(dir.to_str().unwrap().contains("sessions"));
    }
}

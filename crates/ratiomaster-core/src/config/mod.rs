/// Application configuration with TOML persistence.
///
/// Provides platform-aware config directory detection, default config generation,
/// and serialization to/from TOML files.
pub mod custom_profiles;
pub mod session;
pub mod version;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::engine::speed::SpeedConfig;
use crate::engine::stop::StopCondition;
use crate::engine::EngineConfig;
use crate::proxy::ProxyConfig;

/// Top-level application configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// General engine settings.
    #[serde(default)]
    pub general: GeneralConfig,

    /// Upload speed randomization.
    #[serde(default)]
    pub upload: SpeedRangeConfig,

    /// Download speed randomization.
    #[serde(default)]
    pub download: SpeedRangeConfig,

    /// Proxy settings.
    #[serde(default)]
    pub proxy: ProxyToml,

    /// Stop condition settings.
    #[serde(default)]
    pub stop: StopToml,

    /// UI preferences.
    #[serde(default)]
    pub ui: UiConfig,
}

/// General engine defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Default client profile name.
    #[serde(default = "default_client")]
    pub default_client: String,

    /// Default upload speed in KB/s.
    #[serde(default = "default_upload_speed")]
    pub default_upload_speed: u64,

    /// Default download speed in KB/s.
    #[serde(default)]
    pub default_download_speed: u64,

    /// Default announce interval in seconds.
    #[serde(default = "default_interval")]
    pub default_interval: u64,

    /// Whether to enable the TCP handshake listener.
    #[serde(default)]
    pub tcp_listener: bool,

    /// Whether to enable scrape requests.
    #[serde(default = "default_true")]
    pub scrape_enabled: bool,

    /// Whether to ignore tracker failure reasons.
    #[serde(default)]
    pub ignore_failure_reason: bool,
}

/// Speed randomization range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedRangeConfig {
    /// Whether random speed variation is enabled.
    #[serde(default)]
    pub random_enabled: bool,

    /// Minimum speed in KB/s.
    #[serde(default = "default_speed_min")]
    pub random_min: u64,

    /// Maximum speed in KB/s.
    #[serde(default = "default_speed_max")]
    pub random_max: u64,
}

/// Proxy configuration in TOML format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyToml {
    /// Proxy type: "none", "socks4", "socks4a", "socks5", "http".
    #[serde(default = "default_proxy_type", rename = "type")]
    pub proxy_type: String,

    /// Proxy host.
    #[serde(default)]
    pub host: String,

    /// Proxy port.
    #[serde(default = "default_proxy_port")]
    pub port: u16,

    /// Proxy username (SOCKS5/HTTP).
    #[serde(default)]
    pub username: String,

    /// Proxy password (SOCKS5/HTTP).
    #[serde(default)]
    pub password: String,
}

/// Stop condition in TOML format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopToml {
    /// Stop type: "never", "upload", "download", "time", "ratio".
    #[serde(default = "default_stop_type", rename = "type")]
    pub stop_type: String,

    /// Stop value (bytes, seconds, or ratio depending on type).
    #[serde(default)]
    pub value: u64,
}

/// UI preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Color theme.
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Maximum log lines to retain.
    #[serde(default = "default_log_max_lines")]
    pub log_max_lines: usize,
}

// -- Defaults --

fn default_client() -> String {
    "uTorrent 3.3.2".into()
}
fn default_upload_speed() -> u64 {
    100
}
fn default_interval() -> u64 {
    1800
}
fn default_true() -> bool {
    true
}
fn default_speed_min() -> u64 {
    50
}
fn default_speed_max() -> u64 {
    150
}
fn default_proxy_type() -> String {
    "none".into()
}
fn default_proxy_port() -> u16 {
    1080
}
fn default_stop_type() -> String {
    "never".into()
}
fn default_theme() -> String {
    "dark".into()
}
fn default_log_max_lines() -> usize {
    1000
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_client: default_client(),
            default_upload_speed: default_upload_speed(),
            default_download_speed: 0,
            default_interval: default_interval(),
            tcp_listener: false,
            scrape_enabled: true,
            ignore_failure_reason: false,
        }
    }
}

impl Default for SpeedRangeConfig {
    fn default() -> Self {
        Self {
            random_enabled: false,
            random_min: default_speed_min(),
            random_max: default_speed_max(),
        }
    }
}

impl Default for ProxyToml {
    fn default() -> Self {
        Self {
            proxy_type: default_proxy_type(),
            host: String::new(),
            port: default_proxy_port(),
            username: String::new(),
            password: String::new(),
        }
    }
}

impl Default for StopToml {
    fn default() -> Self {
        Self {
            stop_type: default_stop_type(),
            value: 0,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            log_max_lines: default_log_max_lines(),
        }
    }
}

/// Returns the platform-specific configuration directory.
///
/// - Linux: `~/.config/ratiomaster/`
/// - macOS: `~/Library/Application Support/ratiomaster/`
/// - Windows: `%APPDATA%\ratiomaster\`
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ratiomaster")
}

/// Returns the path to the main configuration file.
pub fn config_file_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Loads the application configuration from the default TOML file.
///
/// If the file does not exist, creates it with default values.
pub fn load_config() -> AppConfig {
    let path = config_file_path();

    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => return config,
                Err(e) => {
                    tracing::warn!(
                        "failed to parse config {}: {e}, using defaults",
                        path.display()
                    );
                }
            },
            Err(e) => {
                tracing::warn!(
                    "failed to read config {}: {e}, using defaults",
                    path.display()
                );
            }
        }
    } else {
        // Auto-create with defaults on first run
        let config = AppConfig::default();
        if let Err(e) = save_config(&config) {
            tracing::warn!("failed to create default config: {e}");
        }
        return config;
    }

    AppConfig::default()
}

/// Loads configuration from a specific TOML file path.
pub fn load_config_from(path: &std::path::Path) -> Result<AppConfig, ConfigError> {
    let contents = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
    toml::from_str(&contents).map_err(ConfigError::Parse)
}

/// Saves the application configuration to the default TOML file.
pub fn save_config(config: &AppConfig) -> Result<(), ConfigError> {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
    }
    let contents = toml::to_string_pretty(config).map_err(ConfigError::Serialize)?;
    std::fs::write(&path, contents).map_err(ConfigError::Io)?;
    Ok(())
}

/// Converts the TOML proxy config to a `ProxyConfig`.
impl ProxyToml {
    pub fn to_proxy_config(&self) -> ProxyConfig {
        match self.proxy_type.as_str() {
            "socks4" => ProxyConfig::Socks4 {
                proxy_host: self.host.clone(),
                proxy_port: self.port,
                user_id: self.username.clone(),
            },
            "socks4a" => ProxyConfig::Socks4a {
                proxy_host: self.host.clone(),
                proxy_port: self.port,
                user_id: self.username.clone(),
            },
            "socks5" => ProxyConfig::Socks5 {
                proxy_host: self.host.clone(),
                proxy_port: self.port,
                credentials: if self.username.is_empty() {
                    None
                } else {
                    Some(crate::proxy::socks5::Credentials {
                        username: self.username.clone(),
                        password: self.password.clone(),
                    })
                },
            },
            "http" => ProxyConfig::HttpConnect {
                proxy_host: self.host.clone(),
                proxy_port: self.port,
                credentials: if self.username.is_empty() {
                    None
                } else {
                    Some(crate::proxy::http::Credentials {
                        username: self.username.clone(),
                        password: self.password.clone(),
                    })
                },
            },
            _ => ProxyConfig::None,
        }
    }
}

/// Converts the TOML stop config to a `StopCondition`.
impl StopToml {
    pub fn to_stop_condition(&self) -> StopCondition {
        match self.stop_type.as_str() {
            "upload" => StopCondition::AfterUpload(self.value),
            "download" => StopCondition::AfterDownload(self.value),
            "time" => StopCondition::AfterTime(std::time::Duration::from_secs(self.value)),
            "ratio" => StopCondition::AfterRatio(self.value as f64),
            _ => StopCondition::Never,
        }
    }
}

/// Builds an `EngineConfig` from the application configuration, with optional CLI overrides.
impl AppConfig {
    pub fn to_engine_config(&self) -> EngineConfig {
        let upload_speed_kbs = self.general.default_upload_speed;
        let download_speed_kbs = self.general.default_download_speed;

        let (upload_min, upload_max) = if self.upload.random_enabled {
            (self.upload.random_min * 1024, self.upload.random_max * 1024)
        } else {
            (upload_speed_kbs * 1024, upload_speed_kbs * 1024)
        };

        let (download_min, download_max) = if self.download.random_enabled {
            (
                self.download.random_min * 1024,
                self.download.random_max * 1024,
            )
        } else {
            (download_speed_kbs * 1024, download_speed_kbs * 1024)
        };

        EngineConfig {
            port: 6881,
            speed: SpeedConfig {
                upload_min,
                upload_max,
                download_min,
                download_max,
                variation: 10 * 1024,
            },
            stop_condition: self.stop.to_stop_condition(),
            ignore_failure: self.general.ignore_failure_reason,
            max_retries: 5,
            initial_downloaded_percent: 0,
            http_timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// Errors from configuration operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[source] std::io::Error),

    #[error("toml parse error: {0}")]
    Parse(#[source] toml::de::Error),

    #[error("toml serialize error: {0}")]
    Serialize(#[source] toml::ser::Error),

    #[error("json error: {0}")]
    Json(#[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.general.default_client, "uTorrent 3.3.2");
        assert_eq!(parsed.general.default_upload_speed, 100);
        assert_eq!(parsed.general.default_interval, 1800);
        assert!(parsed.general.scrape_enabled);
    }

    #[test]
    fn proxy_toml_to_config_none() {
        let proxy = ProxyToml::default();
        assert!(matches!(proxy.to_proxy_config(), ProxyConfig::None));
    }

    #[test]
    fn proxy_toml_to_config_socks5() {
        let proxy = ProxyToml {
            proxy_type: "socks5".into(),
            host: "127.0.0.1".into(),
            port: 9050,
            username: "user".into(),
            password: "pass".into(),
        };
        match proxy.to_proxy_config() {
            ProxyConfig::Socks5 {
                proxy_host,
                proxy_port,
                credentials,
            } => {
                assert_eq!(proxy_host, "127.0.0.1");
                assert_eq!(proxy_port, 9050);
                assert!(credentials.is_some());
            }
            other => panic!("expected Socks5, got {other:?}"),
        }
    }

    #[test]
    fn stop_toml_to_condition() {
        let stop = StopToml {
            stop_type: "upload".into(),
            value: 1_000_000,
        };
        assert_eq!(
            stop.to_stop_condition(),
            StopCondition::AfterUpload(1_000_000)
        );
    }

    #[test]
    fn stop_toml_never() {
        let stop = StopToml::default();
        assert_eq!(stop.to_stop_condition(), StopCondition::Never);
    }

    #[test]
    fn engine_config_from_app_config() {
        let config = AppConfig::default();
        let engine = config.to_engine_config();
        assert_eq!(engine.port, 6881);
        assert_eq!(engine.speed.upload_min, 100 * 1024);
        assert_eq!(engine.speed.upload_max, 100 * 1024);
    }

    #[test]
    fn engine_config_with_random_speeds() {
        let mut config = AppConfig::default();
        config.upload.random_enabled = true;
        config.upload.random_min = 50;
        config.upload.random_max = 200;
        let engine = config.to_engine_config();
        assert_eq!(engine.speed.upload_min, 50 * 1024);
        assert_eq!(engine.speed.upload_max, 200 * 1024);
    }

    #[test]
    fn config_dir_not_empty() {
        let dir = config_dir();
        assert!(dir.to_str().unwrap().contains("ratiomaster"));
    }

    #[test]
    fn parse_partial_toml() {
        let toml_str = r#"
[general]
default_client = "Transmission 1.93"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.default_client, "Transmission 1.93");
        // Other fields get defaults
        assert_eq!(config.general.default_upload_speed, 100);
        assert!(config.general.scrape_enabled);
    }
}

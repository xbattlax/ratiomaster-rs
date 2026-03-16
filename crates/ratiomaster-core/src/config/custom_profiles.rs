/// Custom client profile loading from TOML files.
///
/// Users can define additional BitTorrent client profiles in TOML files
/// placed in the config directory under `profiles/`.
use serde::{Deserialize, Serialize};

use crate::client::{ClientFamily, ClientProfile, KeyFormat, RandomType};

/// TOML representation of a custom client profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProfileDef {
    /// Human-readable name (e.g., "MyClient 1.0").
    pub name: String,

    /// Client family: "utorrent", "bitcomet", "vuze", "transmission", etc.
    #[serde(default = "default_family")]
    pub family: String,

    /// Version string.
    pub version: String,

    /// Peer ID prefix as a string (e.g., "-UT3320-").
    pub peer_id_prefix: String,

    /// Random type for peer ID suffix: "alphanumeric", "numeric", "random", "hex".
    #[serde(default = "default_random_type")]
    pub peer_id_random_type: String,

    /// Whether to URL-encode the peer ID.
    #[serde(default)]
    pub peer_id_url_encode: bool,

    /// Whether URL-encoded hex is uppercase.
    #[serde(default)]
    pub peer_id_url_encode_uppercase: bool,

    /// Key format: "numeric:N", "alphanumeric:N", "hex:N".
    #[serde(default = "default_key_format")]
    pub key_format: String,

    /// Whether the key uses uppercase.
    #[serde(default)]
    pub key_uppercase: bool,

    /// HTTP protocol version: "HTTP/1.0" or "HTTP/1.1".
    #[serde(default = "default_http_protocol")]
    pub http_protocol: String,

    /// Query template with placeholders.
    pub query_template: String,

    /// Headers template with `{host}` placeholder.
    pub headers_template: String,

    /// Whether info_hash is encoded with uppercase hex.
    #[serde(default)]
    pub hash_uppercase: bool,

    /// Default numwant value.
    #[serde(default = "default_numwant")]
    pub default_numwant: u16,

    /// Whether to request compact peer format.
    #[serde(default = "default_true")]
    pub compact: bool,

    /// Whether to request peers without IDs.
    #[serde(default)]
    pub no_peer_id: bool,
}

fn default_family() -> String {
    "utorrent".into()
}
fn default_random_type() -> String {
    "alphanumeric".into()
}
fn default_key_format() -> String {
    "hex:8".into()
}
fn default_http_protocol() -> String {
    "HTTP/1.1".into()
}
fn default_numwant() -> u16 {
    200
}
fn default_true() -> bool {
    true
}

/// TOML file containing custom profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProfilesFile {
    /// List of profile definitions.
    #[serde(default)]
    pub profiles: Vec<CustomProfileDef>,
}

/// Returns the directory for custom profile files.
pub fn profiles_dir() -> std::path::PathBuf {
    super::config_dir().join("profiles")
}

/// Loads all custom client profiles from TOML files in the profiles directory.
pub fn load_custom_profiles() -> Vec<ClientProfile> {
    let dir = profiles_dir();
    if !dir.exists() {
        return Vec::new();
    }

    let mut profiles = Vec::new();

    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!("failed to read profiles dir {}: {e}", dir.display());
            return profiles;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str::<CustomProfilesFile>(&contents) {
                    Ok(file) => {
                        for def in file.profiles {
                            match def.to_client_profile() {
                                Ok(profile) => profiles.push(profile),
                                Err(e) => {
                                    tracing::warn!(
                                        "invalid profile '{}' in {}: {e}",
                                        def.name,
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("failed to parse {}: {e}", path.display());
                    }
                },
                Err(e) => {
                    tracing::warn!("failed to read {}: {e}", path.display());
                }
            }
        }
    }

    profiles
}

impl CustomProfileDef {
    /// Converts the TOML definition to a `ClientProfile`.
    pub fn to_client_profile(&self) -> Result<ClientProfile, String> {
        let family = match self.family.to_lowercase().as_str() {
            "utorrent" => ClientFamily::UTorrent,
            "bitcomet" => ClientFamily::BitComet,
            "vuze" => ClientFamily::Vuze,
            "azureus" => ClientFamily::Azureus,
            "bittorrent" => ClientFamily::BitTorrent,
            "transmission" => ClientFamily::Transmission,
            "abc" => ClientFamily::ABC,
            "bitlord" => ClientFamily::BitLord,
            "btuga" => ClientFamily::BTuga,
            "bittornado" => ClientFamily::BitTornado,
            "burst" => ClientFamily::Burst,
            "bittyrant" => ClientFamily::BitTyrant,
            "bitspirit" => ClientFamily::BitSpirit,
            "ktorrent" => ClientFamily::KTorrent,
            "deluge" => ClientFamily::Deluge,
            "gnomebt" => ClientFamily::GnomeBT,
            other => return Err(format!("unknown client family: {other}")),
        };

        let random_type = match self.peer_id_random_type.to_lowercase().as_str() {
            "alphanumeric" => RandomType::Alphanumeric,
            "numeric" => RandomType::Numeric,
            "random" => RandomType::Random,
            "hex" => RandomType::Hex,
            other => return Err(format!("unknown random type: {other}")),
        };

        let key_format = parse_key_format(&self.key_format)?;

        Ok(ClientProfile {
            name: self.name.clone(),
            family,
            version: self.version.clone(),
            peer_id_prefix: self.peer_id_prefix.as_bytes().to_vec(),
            peer_id_random_type: random_type,
            peer_id_url_encode: self.peer_id_url_encode,
            peer_id_url_encode_uppercase: self.peer_id_url_encode_uppercase,
            key_format,
            key_uppercase: self.key_uppercase,
            http_protocol: self.http_protocol.clone(),
            query_template: self.query_template.clone(),
            headers_template: self.headers_template.clone(),
            hash_uppercase: self.hash_uppercase,
            default_numwant: self.default_numwant,
            compact: self.compact,
            no_peer_id: self.no_peer_id,
        })
    }
}

/// Parses a key format string like "hex:8" or "numeric:10".
fn parse_key_format(s: &str) -> Result<KeyFormat, String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid key format: {s} (expected 'type:length')"));
    }

    let len: usize = parts[1]
        .parse()
        .map_err(|_| format!("invalid key length: {}", parts[1]))?;

    match parts[0].to_lowercase().as_str() {
        "numeric" => Ok(KeyFormat::Numeric(len)),
        "alphanumeric" => Ok(KeyFormat::Alphanumeric(len)),
        "hex" => Ok(KeyFormat::Hex(len)),
        other => Err(format!("unknown key format type: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_format_hex() {
        assert_eq!(parse_key_format("hex:8").unwrap(), KeyFormat::Hex(8));
    }

    #[test]
    fn parse_key_format_numeric() {
        assert_eq!(
            parse_key_format("numeric:10").unwrap(),
            KeyFormat::Numeric(10)
        );
    }

    #[test]
    fn parse_key_format_invalid() {
        assert!(parse_key_format("bad").is_err());
    }

    #[test]
    fn custom_profile_roundtrip() {
        let toml_str = r#"
[[profiles]]
name = "TestClient 1.0"
family = "utorrent"
version = "1.0"
peer_id_prefix = "-TC1000-"
query_template = "info_hash={infohash}&peer_id={peerid}&port={port}&uploaded={uploaded}&downloaded={downloaded}&left={left}&compact=1&numwant={numwant}&key={key}{event}"
headers_template = "Host: {host}\r\nUser-Agent: TestClient/1.0\r\nAccept-Encoding: gzip\r\nConnection: close"
"#;
        let file: CustomProfilesFile = toml::from_str(toml_str).unwrap();
        assert_eq!(file.profiles.len(), 1);

        let profile = file.profiles[0].to_client_profile().unwrap();
        assert_eq!(profile.name, "TestClient 1.0");
        assert_eq!(profile.family, ClientFamily::UTorrent);
        assert_eq!(profile.peer_id_prefix, b"-TC1000-");
    }

    #[test]
    fn unknown_family_errors() {
        let def = CustomProfileDef {
            name: "Bad".into(),
            family: "unknown_client".into(),
            version: "1.0".into(),
            peer_id_prefix: "-XX0000-".into(),
            peer_id_random_type: "alphanumeric".into(),
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: "hex:8".into(),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: String::new(),
            headers_template: String::new(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        };
        assert!(def.to_client_profile().is_err());
    }

    #[test]
    fn profiles_dir_path() {
        let dir = profiles_dir();
        assert!(dir.to_str().unwrap().contains("profiles"));
    }
}

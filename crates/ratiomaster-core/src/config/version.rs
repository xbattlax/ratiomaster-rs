//! Version checking against GitHub releases.
//!
//! Provides version comparison logic. The actual HTTP fetch of release data
//! is left to the caller since the core library does not include a TLS client.

/// Current application version from Cargo.toml.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub releases API URL for this project.
pub const RELEASES_URL: &str =
    "https://api.github.com/repos/xbattlax/ratiomaster-rs/releases/latest";

/// Parses the latest version tag from a GitHub releases API JSON response.
///
/// Expects the response to contain a `"tag_name"` field like `"v0.2.0"`.
pub fn parse_latest_version(json_body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json_body).ok()?;
    let tag = value.get("tag_name")?.as_str()?;
    // Strip leading 'v' if present
    let version = tag.strip_prefix('v').unwrap_or(tag);
    Some(version.to_string())
}

/// Compares two semver version strings.
///
/// Returns `true` if `latest` is newer than `current`.
pub fn is_newer(current: &str, latest: &str) -> bool {
    let current_parts = parse_semver(current);
    let latest_parts = parse_semver(latest);

    match (current_parts, latest_parts) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Checks if an update is available given a GitHub API response body.
///
/// Returns `Some(version)` if a newer version exists.
pub fn check_update(json_body: &str) -> Option<String> {
    let latest = parse_latest_version(json_body)?;
    if is_newer(CURRENT_VERSION, &latest) {
        Some(latest)
    } else {
        None
    }
}

/// Parses a version string like "1.2.3" into (major, minor, patch).
fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    let major = parts[0].parse().ok()?;
    let minor = parts[1].parse().ok()?;
    let patch = if parts.len() > 2 {
        parts[2].parse().unwrap_or(0)
    } else {
        0
    };

    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_latest_version_valid() {
        let json = r#"{"tag_name": "v0.2.0", "name": "Release 0.2.0"}"#;
        assert_eq!(parse_latest_version(json), Some("0.2.0".into()));
    }

    #[test]
    fn parse_latest_version_no_prefix() {
        let json = r#"{"tag_name": "1.0.0"}"#;
        assert_eq!(parse_latest_version(json), Some("1.0.0".into()));
    }

    #[test]
    fn parse_latest_version_missing_field() {
        let json = r#"{"name": "Release"}"#;
        assert_eq!(parse_latest_version(json), None);
    }

    #[test]
    fn parse_latest_version_invalid_json() {
        assert_eq!(parse_latest_version("not json"), None);
    }

    #[test]
    fn is_newer_true() {
        assert!(is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("0.1.0", "1.0.0"));
        assert!(is_newer("1.0.0", "1.0.1"));
    }

    #[test]
    fn is_newer_false() {
        assert!(!is_newer("0.2.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn check_update_newer_available() {
        let json = r#"{"tag_name": "v99.0.0"}"#;
        assert!(check_update(json).is_some());
    }

    #[test]
    fn check_update_current_is_latest() {
        let json = format!(r#"{{"tag_name": "v{}"}}"#, CURRENT_VERSION);
        assert!(check_update(&json).is_none());
    }

    #[test]
    fn current_version_not_empty() {
        assert!(!CURRENT_VERSION.is_empty());
    }

    #[test]
    fn parse_semver_valid() {
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver("0.1.0"), Some((0, 1, 0)));
    }

    #[test]
    fn parse_semver_two_parts() {
        assert_eq!(parse_semver("1.0"), Some((1, 0, 0)));
    }

    #[test]
    fn parse_semver_invalid() {
        assert_eq!(parse_semver("bad"), None);
    }
}

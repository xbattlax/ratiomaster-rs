/// Tracker scrape support.
///
/// Converts an announce URL to a scrape URL and parses scrape responses.
/// BEP 48: scrape URL is derived by replacing "/announce" with "/scrape" in the path.
use crate::bencode;

/// Errors that can occur during scrape operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ScrapeError {
    /// The announce URL cannot be converted to a scrape URL.
    #[error("cannot derive scrape URL from: {0}")]
    InvalidAnnounceUrl(String),

    /// The scrape response could not be parsed.
    #[error("invalid scrape response: {0}")]
    InvalidResponse(String),
}

/// Scrape statistics for a single torrent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrapeStats {
    /// Number of seeders (complete peers).
    pub complete: u64,
    /// Number of leechers (incomplete peers).
    pub incomplete: u64,
    /// Number of times the torrent has been downloaded.
    pub downloaded: u64,
}

/// Converts an announce URL to a scrape URL.
///
/// Replaces the last occurrence of "/announce" in the path with "/scrape".
/// Returns an error if the URL doesn't contain "/announce".
pub fn announce_to_scrape_url(announce_url: &str) -> Result<String, ScrapeError> {
    match announce_url.rfind("/announce") {
        Some(pos) => {
            let mut scrape_url = String::with_capacity(announce_url.len());
            scrape_url.push_str(&announce_url[..pos]);
            scrape_url.push_str("/scrape");
            scrape_url.push_str(&announce_url[pos + "/announce".len()..]);
            Ok(scrape_url)
        }
        None => Err(ScrapeError::InvalidAnnounceUrl(announce_url.to_string())),
    }
}

/// Builds the scrape request URL by appending the info_hash parameter.
pub fn build_scrape_url(scrape_url: &str, info_hash: &[u8; 20], uppercase: bool) -> String {
    let encoded = crate::encoding::url_encode(info_hash, uppercase);
    let separator = if scrape_url.contains('?') { '&' } else { '?' };
    format!("{scrape_url}{separator}info_hash={encoded}")
}

/// Parses a BEncoded scrape response body.
///
/// Expected format:
/// ```text
/// d5:filesd20:<info_hash>d8:completei42e10:downloadedi100e10:incompletei7eeee
/// ```
pub fn parse(body: &[u8], info_hash: &[u8; 20]) -> Result<ScrapeStats, ScrapeError> {
    let value = bencode::decode(body).map_err(|e| ScrapeError::InvalidResponse(e.to_string()))?;

    let root = value
        .as_dict()
        .ok_or_else(|| ScrapeError::InvalidResponse("response is not a dict".into()))?;

    let files = root
        .get(b"files".as_ref())
        .and_then(|v| v.as_dict())
        .ok_or_else(|| ScrapeError::InvalidResponse("missing 'files' dict".into()))?;

    let torrent = files
        .get(info_hash.as_ref())
        .and_then(|v| v.as_dict())
        .ok_or_else(|| {
            ScrapeError::InvalidResponse("info_hash not found in scrape response".into())
        })?;

    let complete = torrent
        .get(b"complete".as_ref())
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u64;

    let incomplete = torrent
        .get(b"incomplete".as_ref())
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u64;

    let downloaded = torrent
        .get(b"downloaded".as_ref())
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u64;

    Ok(ScrapeStats {
        complete,
        incomplete,
        downloaded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::{encode, BValue};
    use std::collections::BTreeMap;

    #[test]
    fn announce_to_scrape_basic() {
        assert_eq!(
            announce_to_scrape_url("http://tracker.example.com/announce").unwrap(),
            "http://tracker.example.com/scrape"
        );
    }

    #[test]
    fn announce_to_scrape_with_path() {
        assert_eq!(
            announce_to_scrape_url("http://tracker.example.com:6969/announce").unwrap(),
            "http://tracker.example.com:6969/scrape"
        );
    }

    #[test]
    fn announce_to_scrape_with_passkey() {
        assert_eq!(
            announce_to_scrape_url("http://tracker.test/announce?passkey=abc123").unwrap(),
            "http://tracker.test/scrape?passkey=abc123"
        );
    }

    #[test]
    fn announce_to_scrape_nested_path() {
        assert_eq!(
            announce_to_scrape_url("http://tracker.test/tracker/announce").unwrap(),
            "http://tracker.test/tracker/scrape"
        );
    }

    #[test]
    fn announce_to_scrape_no_announce() {
        assert!(announce_to_scrape_url("http://tracker.test/something").is_err());
    }

    #[test]
    fn build_scrape_url_basic() {
        let hash = [0u8; 20];
        let url = build_scrape_url("http://tracker.test/scrape", &hash, true);
        assert!(url.starts_with("http://tracker.test/scrape?info_hash="));
    }

    #[test]
    fn build_scrape_url_with_existing_query() {
        let hash = [0u8; 20];
        let url = build_scrape_url("http://tracker.test/scrape?passkey=abc", &hash, true);
        assert!(url.contains("scrape?passkey=abc&info_hash="));
    }

    #[test]
    fn parse_scrape_response() {
        let info_hash = [0xABu8; 20];

        let mut torrent_stats = BTreeMap::new();
        torrent_stats.insert(b"complete".to_vec(), BValue::Integer(42));
        torrent_stats.insert(b"downloaded".to_vec(), BValue::Integer(1000));
        torrent_stats.insert(b"incomplete".to_vec(), BValue::Integer(7));

        let mut files = BTreeMap::new();
        files.insert(info_hash.to_vec(), BValue::Dict(torrent_stats));

        let mut root = BTreeMap::new();
        root.insert(b"files".to_vec(), BValue::Dict(files));

        let body = encode(&BValue::Dict(root));
        let stats = parse(&body, &info_hash).unwrap();

        assert_eq!(stats.complete, 42);
        assert_eq!(stats.incomplete, 7);
        assert_eq!(stats.downloaded, 1000);
    }

    #[test]
    fn parse_scrape_missing_hash() {
        let info_hash = [0xAB; 20];
        let other_hash = [0xCD; 20];

        let mut torrent_stats = BTreeMap::new();
        torrent_stats.insert(b"complete".to_vec(), BValue::Integer(1));
        torrent_stats.insert(b"incomplete".to_vec(), BValue::Integer(2));
        torrent_stats.insert(b"downloaded".to_vec(), BValue::Integer(3));

        let mut files = BTreeMap::new();
        files.insert(other_hash.to_vec(), BValue::Dict(torrent_stats));

        let mut root = BTreeMap::new();
        root.insert(b"files".to_vec(), BValue::Dict(files));

        let body = encode(&BValue::Dict(root));
        let err = parse(&body, &info_hash).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn parse_scrape_missing_fields_default_zero() {
        let info_hash = [0xAB; 20];

        // Only "complete" present, others missing
        let mut torrent_stats = BTreeMap::new();
        torrent_stats.insert(b"complete".to_vec(), BValue::Integer(5));

        let mut files = BTreeMap::new();
        files.insert(info_hash.to_vec(), BValue::Dict(torrent_stats));

        let mut root = BTreeMap::new();
        root.insert(b"files".to_vec(), BValue::Dict(files));

        let body = encode(&BValue::Dict(root));
        let stats = parse(&body, &info_hash).unwrap();

        assert_eq!(stats.complete, 5);
        assert_eq!(stats.incomplete, 0);
        assert_eq!(stats.downloaded, 0);
    }
}

/// HTTP tracker announce request builder.
///
/// Builds announce URLs from a tracker URL and query template with placeholder
/// substitution. Supports the same placeholder format as RatioMaster.NET's
/// client profiles.
use crate::encoding::url_encode;

/// Parameters for an announce request.
#[derive(Debug, Clone)]
pub struct AnnounceParams {
    /// URL-encoded info_hash (20 bytes).
    pub info_hash: [u8; 20],
    /// URL-encoded peer_id (20 bytes).
    pub peer_id: [u8; 20],
    /// Listening port.
    pub port: u16,
    /// Total bytes uploaded.
    pub uploaded: u64,
    /// Total bytes downloaded.
    pub downloaded: u64,
    /// Bytes remaining (left = total_size - downloaded).
    pub left: u64,
    /// Number of peers wanted.
    pub numwant: u32,
    /// Random key for this session.
    pub key: String,
    /// Event: "started", "stopped", "completed", or "" for regular announces.
    pub event: String,
    /// Local IP address (may be empty).
    pub local_ip: String,
}

/// Builds the full announce URL by appending query parameters to the tracker URL.
///
/// Uses a query template string with placeholders that get replaced with actual values.
/// Placeholders: `{infohash}`, `{peerid}`, `{port}`, `{uploaded}`, `{downloaded}`,
/// `{left}`, `{numwant}`, `{key}`, `{event}`, `{localip}`.
///
/// Reference: RatioMaster.NET's getUrlString() method.
pub fn build_announce_url(
    tracker_url: &str,
    query_template: &str,
    params: &AnnounceParams,
    uppercase_hash: bool,
) -> String {
    let info_hash_encoded = url_encode(&params.info_hash, uppercase_hash);
    let peer_id_encoded = url_encode(&params.peer_id, uppercase_hash);

    let uploaded = round_by_denominator(params.uploaded, 0x4000);
    let downloaded = round_by_denominator(params.downloaded, 0x10);

    let uploaded_str = uploaded.to_string();
    let downloaded_str = downloaded.to_string();
    let port_str = params.port.to_string();
    let left_str = params.left.to_string();
    let numwant_str = params.numwant.to_string();

    let replacements: &[(&str, &str)] = &[
        ("{infohash}", &info_hash_encoded),
        ("{peerid}", &peer_id_encoded),
        ("{port}", &port_str),
        ("{uploaded}", &uploaded_str),
        ("{downloaded}", &downloaded_str),
        ("{left}", &left_str),
        ("{numwant}", &numwant_str),
        ("{key}", &params.key),
        ("{event}", &params.event),
        ("{localip}", &params.local_ip),
    ];

    let query = substitute_placeholders(query_template, replacements);

    let separator = if tracker_url.contains('?') { '&' } else { '?' };
    format!("{tracker_url}{separator}{query}")
}

/// Builds custom HTTP headers from a header template.
///
/// The template contains one header per line. Placeholders are substituted
/// the same way as the query template.
pub fn build_headers(
    headers_template: &str,
    params: &AnnounceParams,
    uppercase_hash: bool,
) -> Vec<(String, String)> {
    let info_hash_encoded = url_encode(&params.info_hash, uppercase_hash);
    let peer_id_encoded = url_encode(&params.peer_id, uppercase_hash);

    let port_str = params.port.to_string();
    let uploaded_str = params.uploaded.to_string();
    let downloaded_str = params.downloaded.to_string();
    let left_str = params.left.to_string();
    let numwant_str = params.numwant.to_string();

    let replacements: &[(&str, &str)] = &[
        ("{infohash}", &info_hash_encoded),
        ("{peerid}", &peer_id_encoded),
        ("{port}", &port_str),
        ("{uploaded}", &uploaded_str),
        ("{downloaded}", &downloaded_str),
        ("{left}", &left_str),
        ("{numwant}", &numwant_str),
        ("{key}", &params.key),
        ("{event}", &params.event),
        ("{localip}", &params.local_ip),
    ];

    let mut headers = Vec::new();

    for line in headers_template.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let processed = substitute_placeholders(line, replacements);

        if let Some((key, value)) = processed.split_once(':') {
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    }

    headers
}

/// Single-pass placeholder substitution.
///
/// Scans the template once, copying literal text and replacing `{name}` placeholders
/// as they are encountered.
fn substitute_placeholders(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'{' {
            if let Some(close) = template[i..].find('}') {
                let placeholder = &template[i..i + close + 1];
                if let Some((_key, value)) = replacements.iter().find(|(k, _)| *k == placeholder) {
                    result.push_str(value);
                    i += close + 1;
                    continue;
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }

    result
}

/// Rounds a value down to the nearest multiple of the denominator.
///
/// This matches RatioMaster.NET's RoundByDenominator behavior:
/// `value - (value % denominator)`
fn round_by_denominator(value: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return value;
    }
    value - (value % denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> AnnounceParams {
        AnnounceParams {
            info_hash: [
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
                0xde, 0xf0, 0x12, 0x34, 0x56, 0x78,
            ],
            peer_id: *b"-UT3600-abcdefghijkl",
            port: 6881,
            uploaded: 123456789,
            downloaded: 987654321,
            left: 1000000,
            numwant: 200,
            key: "A1B2C3D4".into(),
            event: "started".into(),
            local_ip: "192.168.1.100".into(),
        }
    }

    #[test]
    fn build_url_basic() {
        let params = test_params();
        let template = "info_hash={infohash}&peer_id={peerid}&port={port}&uploaded={uploaded}&downloaded={downloaded}&left={left}&numwant={numwant}&key={key}&event={event}";
        let url = build_announce_url("http://tracker.test/announce", template, &params, true);

        assert!(url.starts_with("http://tracker.test/announce?"));
        assert!(url.contains("info_hash="));
        assert!(url.contains("peer_id="));
        assert!(url.contains("port=6881"));
        assert!(url.contains("numwant=200"));
        assert!(url.contains("key=A1B2C3D4"));
        assert!(url.contains("event=started"));
    }

    #[test]
    fn build_url_tracker_with_existing_query() {
        let params = test_params();
        let url = build_announce_url(
            "http://tracker.test/announce?passkey=abc",
            "info_hash={infohash}",
            &params,
            true,
        );
        // Should use & instead of ? since tracker URL already has ?
        assert!(url.contains("announce?passkey=abc&info_hash="));
    }

    #[test]
    fn round_by_denominator_upload() {
        // 0x4000 = 16384
        assert_eq!(round_by_denominator(123456789, 0x4000), 123453440);
        assert_eq!(round_by_denominator(0, 0x4000), 0);
        assert_eq!(round_by_denominator(16384, 0x4000), 16384);
        assert_eq!(round_by_denominator(16385, 0x4000), 16384);
    }

    #[test]
    fn round_by_denominator_download() {
        // 0x10 = 16
        assert_eq!(round_by_denominator(987654321, 0x10), 987654320);
        assert_eq!(round_by_denominator(16, 0x10), 16);
        assert_eq!(round_by_denominator(17, 0x10), 16);
    }

    #[test]
    fn round_by_denominator_zero() {
        assert_eq!(round_by_denominator(100, 0), 100);
    }

    #[test]
    fn uploaded_downloaded_are_rounded() {
        let params = AnnounceParams {
            info_hash: [0; 20],
            peer_id: [0; 20],
            port: 6881,
            uploaded: 100000,   // 100000 % 0x4000 = 100000 % 16384 = 1696
            downloaded: 200000, // 200000 % 0x10 = 200000 % 16 = 0
            left: 0,
            numwant: 50,
            key: "".into(),
            event: "".into(),
            local_ip: "".into(),
        };
        let url = build_announce_url(
            "http://t.test/ann",
            "uploaded={uploaded}&downloaded={downloaded}",
            &params,
            true,
        );
        // uploaded: 100000 - 1696 = 98304
        assert!(url.contains("uploaded=98304"));
        // downloaded: 200000 - 0 = 200000
        assert!(url.contains("downloaded=200000"));
    }

    #[test]
    fn empty_event_for_regular_announce() {
        let mut params = test_params();
        params.event = String::new();
        let url = build_announce_url("http://t/a", "event={event}&port={port}", &params, true);
        assert!(url.contains("event=&port=6881"));
    }

    #[test]
    fn build_headers_basic() {
        let params = test_params();
        let template = "User-Agent: uTorrent/3600\r\nAccept-Encoding: gzip";
        let headers = build_headers(template, &params, true);

        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].0, "User-Agent");
        assert_eq!(headers[0].1, "uTorrent/3600");
        assert_eq!(headers[1].0, "Accept-Encoding");
        assert_eq!(headers[1].1, "gzip");
    }

    #[test]
    fn build_headers_with_placeholders() {
        let params = test_params();
        let template = "X-Port: {port}";
        let headers = build_headers(template, &params, true);

        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "X-Port");
        assert_eq!(headers[0].1, "6881");
    }

    #[test]
    fn build_headers_empty_template() {
        let params = test_params();
        let headers = build_headers("", &params, true);
        assert!(headers.is_empty());
    }

    #[test]
    fn localip_placeholder() {
        let params = test_params();
        let url = build_announce_url("http://t/a", "ip={localip}", &params, true);
        assert!(url.contains("ip=192.168.1.100"));
    }
}

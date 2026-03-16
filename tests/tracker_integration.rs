//! Integration tests for tracker communication.
//!
//! These tests verify announce URL construction, response parsing, scrape URL
//! conversion, and HTTP response handling at the library boundary.

use std::collections::BTreeMap;

use ratiomaster_core::bencode::{encode, BValue};
use ratiomaster_core::client::profiles::{all_profiles, get_profile};
use ratiomaster_core::network::http::{
    decode_chunked, decompress_gzip, parse_response as parse_http_response,
};
use ratiomaster_core::tracker::announce::{build_announce_url, build_headers, AnnounceParams};
use ratiomaster_core::tracker::response::{parse as parse_tracker_response, TrackerResponseError};
use ratiomaster_core::tracker::scrape::{announce_to_scrape_url, parse as parse_scrape};

fn make_announce_params() -> AnnounceParams {
    AnnounceParams {
        info_hash: [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0, 0x12, 0x34, 0x56, 0x78,
        ],
        peer_id: *b"-UT3320-abcdefghijkl",
        port: 6881,
        uploaded: 1048576,
        downloaded: 524288,
        left: 2097152,
        numwant: 200,
        key: "A1B2C3D4".into(),
        event: "started".into(),
        local_ip: "192.168.1.100".into(),
    }
}

// ========================================================================
// Announce URL construction per client family
// ========================================================================

#[test]
fn announce_url_utorrent_family() {
    let profile = get_profile("uTorrent 3.3.2").unwrap();
    let params = make_announce_params();
    let url = build_announce_url(
        "http://tracker.test/announce",
        &profile.query_template,
        &params,
        profile.hash_uppercase,
    );

    assert!(url.starts_with("http://tracker.test/announce?"));
    assert!(url.contains("info_hash="));
    assert!(url.contains("peer_id="));
    assert!(url.contains("port=6881"));
    assert!(url.contains("compact=1"));
    assert!(url.contains("no_peer_id=1"));
    assert!(url.contains("corrupt=0"));
    assert!(url.contains("key=A1B2C3D4"));
    assert!(url.contains("numwant=200"));
}

#[test]
fn announce_url_bitcomet_family() {
    let profile = get_profile("BitComet 1.20").unwrap();
    let params = make_announce_params();
    let url = build_announce_url(
        "http://tracker.test/announce",
        &profile.query_template,
        &params,
        profile.hash_uppercase,
    );

    assert!(url.contains("natmapped=1"));
    assert!(url.contains("localip=192.168.1.100"));
    assert!(url.contains("port_type=wan"));
}

#[test]
fn announce_url_azureus_family() {
    let profile = get_profile("Vuze 4.2.0.8").unwrap();
    let params = make_announce_params();
    let url = build_announce_url(
        "http://tracker.test/announce",
        &profile.query_template,
        &params,
        profile.hash_uppercase,
    );

    assert!(url.contains("azudp=6881"));
    assert!(url.contains("supportcrypto=1"));
    assert!(url.contains("azver=3"));
}

#[test]
fn announce_url_transmission_family() {
    let profile = get_profile("Transmission 2.92(14714)").unwrap();
    let params = make_announce_params();
    let url = build_announce_url(
        "http://tracker.test/announce",
        &profile.query_template,
        &params,
        profile.hash_uppercase,
    );

    assert!(url.contains("supportcrypto=1"));
}

#[test]
fn announce_url_all_profiles_valid() {
    let params = make_announce_params();
    for profile in all_profiles() {
        let url = build_announce_url(
            "http://tracker.test/announce",
            &profile.query_template,
            &params,
            profile.hash_uppercase,
        );
        assert!(
            url.contains("info_hash="),
            "profile {} missing info_hash",
            profile.name
        );
        assert!(
            url.contains("peer_id="),
            "profile {} missing peer_id",
            profile.name
        );
        assert!(
            url.contains("port="),
            "profile {} missing port",
            profile.name
        );
    }
}

// ========================================================================
// Headers construction per client family
// ========================================================================

#[test]
fn headers_contain_user_agent() {
    for profile in all_profiles() {
        let params = make_announce_params();
        let headers = build_headers(&profile.headers_template, &params, profile.hash_uppercase);
        let has_ua = headers.iter().any(|(k, _)| k == "User-Agent");
        assert!(has_ua, "profile {} missing User-Agent header", profile.name);
    }
}

#[test]
fn headers_contain_accept_encoding() {
    for profile in all_profiles() {
        let params = make_announce_params();
        let headers = build_headers(&profile.headers_template, &params, profile.hash_uppercase);
        let has_ae = headers.iter().any(|(k, _)| k == "Accept-Encoding");
        assert!(
            has_ae,
            "profile {} missing Accept-Encoding header",
            profile.name
        );
    }
}

// ========================================================================
// Tracker response parsing (realistic responses)
// ========================================================================

#[test]
fn parse_realistic_compact_response() {
    let mut dict = BTreeMap::new();
    dict.insert(b"complete".to_vec(), BValue::Integer(150));
    dict.insert(b"incomplete".to_vec(), BValue::Integer(23));
    dict.insert(b"interval".to_vec(), BValue::Integer(1800));
    dict.insert(b"min interval".to_vec(), BValue::Integer(900));

    // 5 peers in compact format
    let mut peers_data = Vec::new();
    let peers_info: &[(u8, u8, u8, u8, u16)] = &[
        (192, 168, 1, 10, 6881),
        (10, 0, 0, 1, 51413),
        (172, 16, 0, 5, 8999),
        (82, 221, 15, 200, 6969),
        (95, 17, 33, 100, 12345),
    ];
    for &(a, b, c, d, port) in peers_info {
        peers_data.extend_from_slice(&[a, b, c, d]);
        peers_data.extend_from_slice(&port.to_be_bytes());
    }

    dict.insert(b"peers".to_vec(), BValue::String(peers_data));
    let body = encode(&BValue::Dict(dict));
    let resp = parse_tracker_response(&body).unwrap();

    assert_eq!(resp.interval, 1800);
    assert_eq!(resp.min_interval, Some(900));
    assert_eq!(resp.complete, Some(150));
    assert_eq!(resp.incomplete, Some(23));
    assert_eq!(resp.peers.len(), 5);
    assert_eq!(resp.peers[0].port, 6881);
    assert_eq!(resp.peers[4].port, 12345);
}

#[test]
fn parse_realistic_dict_peer_response() {
    let mut peer1 = BTreeMap::new();
    peer1.insert(b"ip".to_vec(), BValue::String(b"93.184.216.34".to_vec()));
    peer1.insert(
        b"peer id".to_vec(),
        BValue::String(b"-TR2920-012345678901".to_vec()),
    );
    peer1.insert(b"port".to_vec(), BValue::Integer(51413));

    let mut peer2 = BTreeMap::new();
    peer2.insert(b"ip".to_vec(), BValue::String(b"198.51.100.1".to_vec()));
    peer2.insert(
        b"peer id".to_vec(),
        BValue::String(b"-DE1200-abcdefghijkl".to_vec()),
    );
    peer2.insert(b"port".to_vec(), BValue::Integer(6881));

    let mut dict = BTreeMap::new();
    dict.insert(b"interval".to_vec(), BValue::Integer(1800));
    dict.insert(
        b"peers".to_vec(),
        BValue::List(vec![BValue::Dict(peer1), BValue::Dict(peer2)]),
    );

    let body = encode(&BValue::Dict(dict));
    let resp = parse_tracker_response(&body).unwrap();

    assert_eq!(resp.peers.len(), 2);
    assert_eq!(resp.peers[0].port, 51413);
    assert_eq!(
        resp.peers[0].peer_id.as_deref(),
        Some(b"-TR2920-012345678901".as_ref())
    );
    assert_eq!(resp.peers[1].port, 6881);
}

#[test]
fn parse_tracker_failure_reason() {
    let mut dict = BTreeMap::new();
    dict.insert(
        b"failure reason".to_vec(),
        BValue::String(b"Torrent not registered with this tracker".to_vec()),
    );
    let body = encode(&BValue::Dict(dict));
    let err = parse_tracker_response(&body).unwrap_err();
    match err {
        TrackerResponseError::TrackerFailure(msg) => {
            assert!(msg.contains("not registered"));
        }
        other => panic!("expected TrackerFailure, got {other:?}"),
    }
}

#[test]
fn parse_tracker_warning_message() {
    let mut dict = BTreeMap::new();
    dict.insert(b"interval".to_vec(), BValue::Integer(1800));
    dict.insert(b"peers".to_vec(), BValue::String(vec![]));
    dict.insert(
        b"warning message".to_vec(),
        BValue::String(b"Your ratio is below minimum".to_vec()),
    );
    let body = encode(&BValue::Dict(dict));
    let resp = parse_tracker_response(&body).unwrap();
    assert_eq!(
        resp.warning_message.as_deref(),
        Some("Your ratio is below minimum")
    );
}

#[test]
fn parse_tracker_with_tracker_id() {
    let mut dict = BTreeMap::new();
    dict.insert(b"interval".to_vec(), BValue::Integer(600));
    dict.insert(b"peers".to_vec(), BValue::String(vec![]));
    dict.insert(
        b"tracker id".to_vec(),
        BValue::String(b"abc123tracker".to_vec()),
    );
    let body = encode(&BValue::Dict(dict));
    let resp = parse_tracker_response(&body).unwrap();
    assert_eq!(resp.tracker_id.as_deref(), Some("abc123tracker"));
}

// ========================================================================
// Scrape URL conversion
// ========================================================================

#[test]
fn scrape_url_standard_tracker() {
    assert_eq!(
        announce_to_scrape_url("http://tracker.example.com:6969/announce").unwrap(),
        "http://tracker.example.com:6969/scrape"
    );
}

#[test]
fn scrape_url_with_passkey() {
    assert_eq!(
        announce_to_scrape_url("http://private.tracker.com/announce?passkey=secret123").unwrap(),
        "http://private.tracker.com/scrape?passkey=secret123"
    );
}

#[test]
fn scrape_url_nested_path() {
    assert_eq!(
        announce_to_scrape_url("http://tracker.com/tracker/announce").unwrap(),
        "http://tracker.com/tracker/scrape"
    );
}

#[test]
fn scrape_url_php_announce() {
    assert_eq!(
        announce_to_scrape_url("http://tracker.com/announce.php?info_hash=abc").unwrap(),
        "http://tracker.com/scrape.php?info_hash=abc"
    );
}

#[test]
fn scrape_url_no_announce_fails() {
    assert!(announce_to_scrape_url("http://tracker.com/something").is_err());
}

#[test]
fn scrape_response_parsing() {
    let info_hash = [0xAB; 20];

    let mut stats = BTreeMap::new();
    stats.insert(b"complete".to_vec(), BValue::Integer(50));
    stats.insert(b"downloaded".to_vec(), BValue::Integer(500));
    stats.insert(b"incomplete".to_vec(), BValue::Integer(10));

    let mut files = BTreeMap::new();
    files.insert(info_hash.to_vec(), BValue::Dict(stats));

    let mut root = BTreeMap::new();
    root.insert(b"files".to_vec(), BValue::Dict(files));

    let body = encode(&BValue::Dict(root));
    let result = parse_scrape(&body, &info_hash).unwrap();
    assert_eq!(result.complete, 50);
    assert_eq!(result.incomplete, 10);
    assert_eq!(result.downloaded, 500);
}

// ========================================================================
// HTTP response parsing: gzip
// ========================================================================

#[test]
fn parse_gzip_compressed_tracker_response() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    // Build a bencoded tracker response
    let mut dict = BTreeMap::new();
    dict.insert(b"interval".to_vec(), BValue::Integer(1800));
    dict.insert(b"peers".to_vec(), BValue::String(vec![]));
    dict.insert(b"complete".to_vec(), BValue::Integer(42));
    dict.insert(b"incomplete".to_vec(), BValue::Integer(7));
    let bencoded_body = encode(&BValue::Dict(dict));

    // gzip compress it
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&bencoded_body).unwrap();
    let compressed = encoder.finish().unwrap();

    // Build full HTTP response
    let mut raw =
        b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Type: text/plain\r\n\r\n".to_vec();
    raw.extend_from_slice(&compressed);

    let http_resp = parse_http_response(&raw).unwrap();
    assert_eq!(http_resp.status_code, 200);
    // The body should be decompressed
    let tracker_resp = parse_tracker_response(&http_resp.body).unwrap();
    assert_eq!(tracker_resp.interval, 1800);
    assert_eq!(tracker_resp.complete, Some(42));
}

// ========================================================================
// HTTP response parsing: chunked transfer encoding
// ========================================================================

#[test]
fn parse_chunked_tracker_response() {
    // Build bencoded body
    let mut dict = BTreeMap::new();
    dict.insert(b"interval".to_vec(), BValue::Integer(900));
    dict.insert(b"peers".to_vec(), BValue::String(vec![]));
    let bencoded_body = encode(&BValue::Dict(dict));

    // Chunk it: send as a single chunk
    let chunk_size = format!("{:x}", bencoded_body.len());
    let mut chunked_body = Vec::new();
    chunked_body.extend_from_slice(chunk_size.as_bytes());
    chunked_body.extend_from_slice(b"\r\n");
    chunked_body.extend_from_slice(&bencoded_body);
    chunked_body.extend_from_slice(b"\r\n0\r\n\r\n");

    let mut raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
    raw.extend_from_slice(&chunked_body);

    let http_resp = parse_http_response(&raw).unwrap();
    assert_eq!(http_resp.status_code, 200);
    let tracker_resp = parse_tracker_response(&http_resp.body).unwrap();
    assert_eq!(tracker_resp.interval, 900);
}

// ========================================================================
// HTTP response parsing: redirect (302)
// ========================================================================

#[test]
fn parse_302_redirect_response() {
    let raw = b"HTTP/1.1 302 Found\r\nLocation: http://new-tracker.test/announce\r\n\r\n";
    let resp = parse_http_response(raw).unwrap();
    assert_eq!(resp.status_code, 302);
    assert_eq!(
        resp.header("location"),
        Some("http://new-tracker.test/announce")
    );
}

#[test]
fn parse_301_redirect_response() {
    let raw = b"HTTP/1.1 301 Moved Permanently\r\nLocation: http://other.test/announce\r\n\r\n";
    let resp = parse_http_response(raw).unwrap();
    assert_eq!(resp.status_code, 301);
    assert_eq!(resp.header("location"), Some("http://other.test/announce"));
}

// ========================================================================
// HTTP response parsing: various status codes
// ========================================================================

#[test]
fn parse_http_500_error() {
    let raw = b"HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\n\r\nServer Error";
    let resp = parse_http_response(raw).unwrap();
    assert_eq!(resp.status_code, 500);
    assert_eq!(resp.body, b"Server Error");
}

#[test]
fn parse_http_403_forbidden() {
    let raw = b"HTTP/1.1 403 Forbidden\r\n\r\nAccess denied";
    let resp = parse_http_response(raw).unwrap();
    assert_eq!(resp.status_code, 403);
}

// ========================================================================
// Chunked encoding edge cases
// ========================================================================

#[test]
fn decode_chunked_multi_chunk() {
    // Three chunks
    let data = b"5\r\nHello\r\n7\r\n, World\r\n1\r\n!\r\n0\r\n\r\n";
    let decoded = decode_chunked(data).unwrap();
    assert_eq!(decoded, b"Hello, World!");
}

#[test]
fn decode_chunked_empty() {
    let data = b"0\r\n\r\n";
    let decoded = decode_chunked(data).unwrap();
    assert!(decoded.is_empty());
}

// ========================================================================
// Gzip decompression
// ========================================================================

#[test]
fn gzip_roundtrip_with_bencode() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let original = b"d8:intervali1800e5:peers0:e";
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(original).unwrap();
    let compressed = encoder.finish().unwrap();
    let decompressed = decompress_gzip(&compressed).unwrap();
    assert_eq!(decompressed, original);
}

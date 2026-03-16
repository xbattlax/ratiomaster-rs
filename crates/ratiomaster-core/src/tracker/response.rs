/// HTTP tracker response parser.
///
/// Parses the BEncoded body from a tracker announce response to extract:
/// interval, peers (compact or dict format), seeders/leechers, and failure reasons.
use std::net::{Ipv4Addr, SocketAddrV4};

use crate::bencode::{self, BValue};

/// Errors that can occur when parsing tracker responses.
#[derive(Debug, thiserror::Error)]
pub enum TrackerResponseError {
    /// The tracker returned a failure reason.
    #[error("tracker failure: {0}")]
    TrackerFailure(String),

    /// The response body could not be parsed as BEncoded data.
    #[error("invalid bencode in response: {0}")]
    InvalidBencode(String),

    /// A required field was missing from the response.
    #[error("missing field: {0}")]
    MissingField(String),

    /// A field had an unexpected type or format.
    #[error("invalid field: {0}")]
    InvalidField(String),
}

/// A peer returned by the tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub peer_id: Option<Vec<u8>>,
}

/// Parsed tracker announce response.
#[derive(Debug, Clone)]
pub struct TrackerResponse {
    /// Seconds between regular announces.
    pub interval: u64,
    /// Minimum interval (optional).
    pub min_interval: Option<u64>,
    /// List of peers.
    pub peers: Vec<Peer>,
    /// Number of seeders (complete).
    pub complete: Option<u64>,
    /// Number of leechers (incomplete).
    pub incomplete: Option<u64>,
    /// Number of times the torrent has been downloaded.
    pub downloaded: Option<u64>,
    /// Tracker ID (some trackers send this).
    pub tracker_id: Option<String>,
    /// Warning message from the tracker.
    pub warning_message: Option<String>,
}

/// Parses a BEncoded tracker response body.
pub fn parse(body: &[u8]) -> Result<TrackerResponse, TrackerResponseError> {
    let value =
        bencode::decode(body).map_err(|e| TrackerResponseError::InvalidBencode(e.to_string()))?;

    let dict = value
        .as_dict()
        .ok_or_else(|| TrackerResponseError::InvalidBencode("response is not a dict".into()))?;

    // Check for failure reason first
    if let Some(failure) = dict.get(b"failure reason".as_ref()) {
        let reason = failure.as_str().unwrap_or("unknown failure").to_string();
        return Err(TrackerResponseError::TrackerFailure(reason));
    }

    let interval =
        dict.get(b"interval".as_ref())
            .and_then(|v| v.as_integer())
            .ok_or_else(|| TrackerResponseError::MissingField("interval".into()))? as u64;

    let min_interval = dict
        .get(b"min interval".as_ref())
        .and_then(|v| v.as_integer())
        .map(|v| v as u64);

    let peers = parse_peers(dict)?;

    let complete = dict
        .get(b"complete".as_ref())
        .and_then(|v| v.as_integer())
        .map(|v| v as u64);

    let incomplete = dict
        .get(b"incomplete".as_ref())
        .and_then(|v| v.as_integer())
        .map(|v| v as u64);

    let downloaded = dict
        .get(b"downloaded".as_ref())
        .and_then(|v| v.as_integer())
        .map(|v| v as u64);

    let tracker_id = dict
        .get(b"tracker id".as_ref())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let warning_message = dict
        .get(b"warning message".as_ref())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(TrackerResponse {
        interval,
        min_interval,
        peers,
        complete,
        incomplete,
        downloaded,
        tracker_id,
        warning_message,
    })
}

/// Parses peers from either compact (6-byte per peer) or dictionary format.
fn parse_peers(
    dict: &std::collections::BTreeMap<Vec<u8>, BValue>,
) -> Result<Vec<Peer>, TrackerResponseError> {
    let peers_value = match dict.get(b"peers".as_ref()) {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    match peers_value {
        BValue::String(data) => parse_compact_peers(data),
        BValue::List(list) => parse_dict_peers(list),
        _ => Err(TrackerResponseError::InvalidField(
            "peers must be a string (compact) or list (dict)".into(),
        )),
    }
}

/// Parses compact peer format: each peer is 6 bytes (4 IP + 2 port).
fn parse_compact_peers(data: &[u8]) -> Result<Vec<Peer>, TrackerResponseError> {
    if !data.len().is_multiple_of(6) {
        return Err(TrackerResponseError::InvalidField(format!(
            "compact peers length {} is not a multiple of 6",
            data.len()
        )));
    }

    let peers = data
        .chunks_exact(6)
        .map(|chunk| {
            let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
            let port = u16::from_be_bytes([chunk[4], chunk[5]]);
            Peer {
                ip,
                port,
                peer_id: None,
            }
        })
        .collect();

    Ok(peers)
}

/// Parses dictionary-format peers: each peer is a dict with "ip", "port", optionally "peer id".
fn parse_dict_peers(list: &[BValue]) -> Result<Vec<Peer>, TrackerResponseError> {
    let mut peers = Vec::with_capacity(list.len());

    for item in list {
        let peer_dict = item
            .as_dict()
            .ok_or_else(|| TrackerResponseError::InvalidField("peer entry is not a dict".into()))?;

        let ip_str = peer_dict
            .get(b"ip".as_ref())
            .and_then(|v| v.as_str())
            .ok_or_else(|| TrackerResponseError::MissingField("peer.ip".into()))?;

        let ip: Ipv4Addr = ip_str.parse().map_err(|_| {
            TrackerResponseError::InvalidField(format!("invalid peer IP: {ip_str}"))
        })?;

        let port = peer_dict
            .get(b"port".as_ref())
            .and_then(|v| v.as_integer())
            .ok_or_else(|| TrackerResponseError::MissingField("peer.port".into()))?
            as u16;

        let peer_id = peer_dict
            .get(b"peer id".as_ref())
            .and_then(|v| v.as_bytes())
            .map(|b| b.to_vec());

        peers.push(Peer { ip, port, peer_id });
    }

    Ok(peers)
}

/// Convenience: parse a raw socket address from a compact peer entry.
pub fn peer_to_socket_addr(peer: &Peer) -> SocketAddrV4 {
    SocketAddrV4::new(peer.ip, peer.port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::{encode, BValue};
    use std::collections::BTreeMap;

    fn build_response(interval: i64, peers: BValue) -> Vec<u8> {
        let mut dict = BTreeMap::new();
        dict.insert(b"interval".to_vec(), BValue::Integer(interval));
        dict.insert(b"peers".to_vec(), peers);
        encode(&BValue::Dict(dict))
    }

    #[test]
    fn parse_compact_peers_basic() {
        // Two peers: 192.168.1.1:6881, 10.0.0.1:8080
        let mut peers_data = Vec::new();
        peers_data.extend_from_slice(&[192, 168, 1, 1]);
        peers_data.extend_from_slice(&6881u16.to_be_bytes());
        peers_data.extend_from_slice(&[10, 0, 0, 1]);
        peers_data.extend_from_slice(&8080u16.to_be_bytes());

        let body = build_response(1800, BValue::String(peers_data));
        let resp = parse(&body).unwrap();

        assert_eq!(resp.interval, 1800);
        assert_eq!(resp.peers.len(), 2);
        assert_eq!(resp.peers[0].ip, Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(resp.peers[0].port, 6881);
        assert_eq!(resp.peers[1].ip, Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(resp.peers[1].port, 8080);
    }

    #[test]
    fn parse_dict_peers() {
        let mut peer1 = BTreeMap::new();
        peer1.insert(b"ip".to_vec(), BValue::String(b"192.168.1.1".to_vec()));
        peer1.insert(b"port".to_vec(), BValue::Integer(6881));
        peer1.insert(
            b"peer id".to_vec(),
            BValue::String(b"-UT3600-xxxxxxxxxxxx".to_vec()),
        );

        let peers = BValue::List(vec![BValue::Dict(peer1)]);
        let body = build_response(900, peers);
        let resp = parse(&body).unwrap();

        assert_eq!(resp.interval, 900);
        assert_eq!(resp.peers.len(), 1);
        assert_eq!(resp.peers[0].ip, Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(resp.peers[0].port, 6881);
        assert_eq!(
            resp.peers[0].peer_id.as_deref(),
            Some(b"-UT3600-xxxxxxxxxxxx".as_ref())
        );
    }

    #[test]
    fn parse_failure_reason() {
        let mut dict = BTreeMap::new();
        dict.insert(
            b"failure reason".to_vec(),
            BValue::String(b"torrent not registered".to_vec()),
        );
        let body = encode(&BValue::Dict(dict));

        let err = parse(&body).unwrap_err();
        match err {
            TrackerResponseError::TrackerFailure(reason) => {
                assert_eq!(reason, "torrent not registered");
            }
            other => panic!("expected TrackerFailure, got {other:?}"),
        }
    }

    #[test]
    fn parse_complete_incomplete_downloaded() {
        let mut dict = BTreeMap::new();
        dict.insert(b"complete".to_vec(), BValue::Integer(42));
        dict.insert(b"downloaded".to_vec(), BValue::Integer(1000));
        dict.insert(b"incomplete".to_vec(), BValue::Integer(7));
        dict.insert(b"interval".to_vec(), BValue::Integer(1800));
        dict.insert(b"peers".to_vec(), BValue::String(vec![]));

        let body = encode(&BValue::Dict(dict));
        let resp = parse(&body).unwrap();

        assert_eq!(resp.complete, Some(42));
        assert_eq!(resp.incomplete, Some(7));
        assert_eq!(resp.downloaded, Some(1000));
    }

    #[test]
    fn parse_min_interval() {
        let mut dict = BTreeMap::new();
        dict.insert(b"interval".to_vec(), BValue::Integer(1800));
        dict.insert(b"min interval".to_vec(), BValue::Integer(900));
        dict.insert(b"peers".to_vec(), BValue::String(vec![]));

        let body = encode(&BValue::Dict(dict));
        let resp = parse(&body).unwrap();

        assert_eq!(resp.interval, 1800);
        assert_eq!(resp.min_interval, Some(900));
    }

    #[test]
    fn parse_warning_message() {
        let mut dict = BTreeMap::new();
        dict.insert(b"interval".to_vec(), BValue::Integer(1800));
        dict.insert(b"peers".to_vec(), BValue::String(vec![]));
        dict.insert(
            b"warning message".to_vec(),
            BValue::String(b"your ratio is low".to_vec()),
        );

        let body = encode(&BValue::Dict(dict));
        let resp = parse(&body).unwrap();

        assert_eq!(resp.warning_message.as_deref(), Some("your ratio is low"));
    }

    #[test]
    fn parse_empty_peers() {
        let body = build_response(1800, BValue::String(vec![]));
        let resp = parse(&body).unwrap();
        assert!(resp.peers.is_empty());
    }

    #[test]
    fn compact_peers_invalid_length() {
        // 7 bytes is not a multiple of 6
        let body = build_response(1800, BValue::String(vec![0; 7]));
        let err = parse(&body).unwrap_err();
        assert!(matches!(err, TrackerResponseError::InvalidField(_)));
    }

    #[test]
    fn missing_interval() {
        let mut dict = BTreeMap::new();
        dict.insert(b"peers".to_vec(), BValue::String(vec![]));
        let body = encode(&BValue::Dict(dict));

        let err = parse(&body).unwrap_err();
        assert!(matches!(err, TrackerResponseError::MissingField(_)));
    }

    #[test]
    fn peer_to_socket_addr_conversion() {
        let peer = Peer {
            ip: Ipv4Addr::new(10, 0, 0, 1),
            port: 6881,
            peer_id: None,
        };
        let addr = peer_to_socket_addr(&peer);
        assert_eq!(addr.ip(), &Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(addr.port(), 6881);
    }
}

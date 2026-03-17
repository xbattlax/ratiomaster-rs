use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ratiomaster_core::bencode::{encode, BValue};
use ratiomaster_core::client::profiles;
use ratiomaster_core::engine::{Engine, EngineConfig, EngineError};
use ratiomaster_core::network::http::{HttpError, HttpResponse, HttpVersion};
use ratiomaster_core::torrent::TorrentMetainfo;
use ratiomaster_core::tracker::client::TrackerClient;

/// What the mock should return on each announce.
enum MockResponse {
    Ok(Vec<u8>),
    Timeout,
    ConnectionError(String),
}

/// A mock tracker client that returns predefined responses.
struct MockTrackerClient {
    response: MockResponse,
    announce_count: Arc<AtomicUsize>,
}

impl MockTrackerClient {
    fn ok(body: Vec<u8>) -> Self {
        Self {
            response: MockResponse::Ok(body),
            announce_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn timeout() -> Self {
        Self {
            response: MockResponse::Timeout,
            announce_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn connection_error(msg: &str) -> Self {
        Self {
            response: MockResponse::ConnectionError(msg.to_string()),
            announce_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn count(&self) -> Arc<AtomicUsize> {
        self.announce_count.clone()
    }
}

impl TrackerClient for MockTrackerClient {
    fn announce<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _http_version: HttpVersion,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpError>> + Send + 'a>> {
        self.announce_count.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            match &self.response {
                MockResponse::Ok(body) => Ok(HttpResponse {
                    status_code: 200,
                    headers: vec![],
                    body: body.clone(),
                }),
                MockResponse::Timeout => Err(HttpError::Timeout),
                MockResponse::ConnectionError(msg) => Err(HttpError::Connection(msg.clone())),
            }
        })
    }

    fn scrape<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _http_version: HttpVersion,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpError>> + Send + 'a>> {
        Box::pin(async { Err(HttpError::Timeout) })
    }
}

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

fn build_tracker_response(interval: i64, complete: i64, incomplete: i64) -> Vec<u8> {
    let mut dict = BTreeMap::new();
    dict.insert(b"complete".to_vec(), BValue::Integer(complete));
    dict.insert(b"incomplete".to_vec(), BValue::Integer(incomplete));
    dict.insert(b"interval".to_vec(), BValue::Integer(interval));
    dict.insert(b"peers".to_vec(), BValue::String(vec![]));
    encode(&BValue::Dict(dict))
}

fn build_failure_response(reason: &str) -> Vec<u8> {
    let mut dict = BTreeMap::new();
    dict.insert(
        b"failure reason".to_vec(),
        BValue::String(reason.as_bytes().to_vec()),
    );
    encode(&BValue::Dict(dict))
}

fn build_warning_response(interval: i64, warning: &str) -> Vec<u8> {
    let mut dict = BTreeMap::new();
    dict.insert(b"interval".to_vec(), BValue::Integer(interval));
    dict.insert(b"peers".to_vec(), BValue::String(vec![]));
    dict.insert(
        b"warning message".to_vec(),
        BValue::String(warning.as_bytes().to_vec()),
    );
    encode(&BValue::Dict(dict))
}

#[tokio::test]
async fn start_sends_started_event_and_parses_response() {
    let body = build_tracker_response(1800, 50, 10);
    let mock = MockTrackerClient::ok(body);
    let count = mock.count();

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    let result = engine.start().await.unwrap();

    assert_eq!(result.interval, 1800);
    assert_eq!(result.seeders, Some(50));
    assert_eq!(result.leechers, Some(10));
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // Engine state should be updated with seeder/leecher counts
    assert_eq!(engine.state().seeders, 50);
    assert_eq!(engine.state().leechers, 10);
}

#[tokio::test]
async fn announce_sends_regular_announce_and_updates_state() {
    let body = build_tracker_response(900, 100, 20);
    let mock = MockTrackerClient::ok(body);
    let count = mock.count();

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    let result = engine.announce().await.unwrap();

    assert_eq!(result.interval, 900);
    assert_eq!(result.seeders, Some(100));
    assert_eq!(result.leechers, Some(20));
    assert_eq!(count.load(Ordering::SeqCst), 1);

    assert_eq!(engine.state().seeders, 100);
    assert_eq!(engine.state().leechers, 20);
}

#[tokio::test]
async fn stop_sends_stopped_event() {
    let body = build_tracker_response(1800, 5, 3);
    let mock = MockTrackerClient::ok(body);
    let count = mock.count();

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    let result = engine.stop().await.unwrap();

    assert_eq!(result.interval, 1800);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn engine_handles_tracker_failure_response() {
    let body = build_failure_response("torrent not registered");
    let mock = MockTrackerClient::ok(body);

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    let err = engine.start().await.unwrap_err();
    match err {
        EngineError::TrackerFailure(reason) => {
            assert_eq!(reason, "torrent not registered");
        }
        other => panic!("expected TrackerFailure, got {other:?}"),
    }
}

#[tokio::test]
async fn engine_handles_warning_message() {
    let body = build_warning_response(1800, "your ratio is low");
    let mock = MockTrackerClient::ok(body);

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    let result = engine.start().await.unwrap();

    assert_eq!(result.warning.as_deref(), Some("your ratio is low"));
    assert_eq!(result.interval, 1800);
}

#[tokio::test]
async fn engine_handles_timeout_error() {
    let mock = MockTrackerClient::timeout();

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    let err = engine.start().await.unwrap_err();
    assert!(matches!(err, EngineError::Http(HttpError::Timeout)));
}

#[tokio::test]
async fn engine_handles_connection_error() {
    let mock = MockTrackerClient::connection_error("refused");

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    let err = engine.start().await.unwrap_err();
    assert!(matches!(err, EngineError::Http(HttpError::Connection(_))));
}

#[tokio::test]
async fn start_then_announce_then_stop_lifecycle() {
    let body = build_tracker_response(600, 25, 5);
    let mock = MockTrackerClient::ok(body);
    let count = mock.count();

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    // Full lifecycle
    engine.start().await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);

    engine.announce().await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 2);

    engine.stop().await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn seed_mode_initial_state_with_mock() {
    let body = build_tracker_response(1800, 10, 2);
    let mock = MockTrackerClient::ok(body);

    let profile = profiles::get_profile("uTorrent 3.3.2").unwrap().clone();
    let config = EngineConfig {
        initial_downloaded_percent: 100,
        ..EngineConfig::default()
    };

    let mut engine = Engine::new_with_client(test_torrent(), profile, config, Box::new(mock));

    assert_eq!(engine.state().left, 0);
    assert!(engine.state().completed_sent);

    let result = engine.start().await.unwrap();
    assert_eq!(result.interval, 1800);
}

#[tokio::test]
async fn multiple_announces_update_state_each_time() {
    // Use a mock that always returns the same response but we verify state updates
    let body = build_tracker_response(300, 42, 7);
    let mock = MockTrackerClient::ok(body);

    let profile = profiles::get_profile("Transmission 2.82(14160)")
        .unwrap()
        .clone();
    let mut engine = Engine::new_with_client(
        test_torrent(),
        profile,
        EngineConfig::default(),
        Box::new(mock),
    );

    for _ in 0..5 {
        let result = engine.announce().await.unwrap();
        assert_eq!(result.interval, 300);
        assert_eq!(engine.state().seeders, 42);
        assert_eq!(engine.state().leechers, 7);
    }
}

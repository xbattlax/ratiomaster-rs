use proptest::prelude::*;
use std::collections::BTreeMap;

use ratiomaster_core::bencode::{decode, encode, BValue};
use ratiomaster_core::client::generator::{generate_key, generate_peer_id};
use ratiomaster_core::client::{ClientFamily, ClientProfile, KeyFormat, RandomType};
use ratiomaster_core::encoding::url_encode;
use ratiomaster_core::engine::speed::{init_speed, vary_speed, SpeedConfig};
use ratiomaster_core::torrent;

// ---------------------------------------------------------------------------
// Bencode: encode(decode(x)) roundtrip
// ---------------------------------------------------------------------------

fn arb_bvalue() -> impl Strategy<Value = BValue> {
    let leaf = prop_oneof![
        any::<Vec<u8>>().prop_map(BValue::String),
        any::<i64>().prop_map(BValue::Integer),
    ];

    leaf.prop_recursive(
        4,  // depth
        64, // max nodes
        8,  // items per collection
        |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..8).prop_map(BValue::List),
                prop::collection::vec((any::<Vec<u8>>(), inner), 0..8).prop_map(|entries| {
                    let dict: BTreeMap<Vec<u8>, BValue> = entries.into_iter().collect();
                    BValue::Dict(dict)
                }),
            ]
        },
    )
}

proptest! {
    #[test]
    fn bencode_roundtrip(value in arb_bvalue()) {
        let encoded = encode(&value);
        let decoded = decode(&encoded).unwrap();
        prop_assert_eq!(decoded, value);
    }

    // ---------------------------------------------------------------------------
    // Bencode: decode never panics on arbitrary bytes
    // ---------------------------------------------------------------------------

    #[test]
    fn bencode_decode_no_panic(data in any::<Vec<u8>>()) {
        let _ = decode(&data);
    }

    // ---------------------------------------------------------------------------
    // URL encode: output is always valid percent-encoding
    // ---------------------------------------------------------------------------

    #[test]
    fn url_encode_valid_percent_encoding(data in any::<Vec<u8>>(), uppercase in any::<bool>()) {
        let encoded = url_encode(&data, uppercase);

        // Walk the encoded string and verify structure
        let bytes = encoded.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                // Must have two hex digits following
                prop_assert!(i + 2 < bytes.len(), "truncated percent encoding at pos {}", i);
                let h1 = bytes[i + 1];
                let h2 = bytes[i + 2];
                prop_assert!(
                    h1.is_ascii_hexdigit() && h2.is_ascii_hexdigit(),
                    "invalid hex digits after % at pos {}: {:?}{:?}", i, h1 as char, h2 as char
                );
                i += 3;
            } else {
                // Must be an unreserved character
                let b = bytes[i];
                prop_assert!(
                    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~'),
                    "non-unreserved char '{}' at pos {} not percent-encoded", b as char, i
                );
                i += 1;
            }
        }
    }

    // ---------------------------------------------------------------------------
    // URL encode: unreserved chars always pass through unchanged
    // ---------------------------------------------------------------------------

    #[test]
    fn url_encode_unreserved_passthrough(
        s in "[a-zA-Z0-9._~-]{1,100}"
    ) {
        let encoded = url_encode(s.as_bytes(), true);
        prop_assert_eq!(&encoded, &s);
    }

    // ---------------------------------------------------------------------------
    // Torrent parser: never panics on arbitrary bytes
    // ---------------------------------------------------------------------------

    #[test]
    fn torrent_parse_no_panic(data in any::<Vec<u8>>()) {
        let _ = torrent::parse(&data);
    }

    // ---------------------------------------------------------------------------
    // Speed: init_speed produces values within configured range
    // ---------------------------------------------------------------------------

    #[test]
    fn init_speed_within_range(
        upload_min in 0u64..=1_000_000u64,
        upload_spread in 0u64..=1_000_000u64,
        download_min in 0u64..=1_000_000u64,
        download_spread in 0u64..=1_000_000u64,
    ) {
        let upload_max = upload_min.saturating_add(upload_spread);
        let download_max = download_min.saturating_add(download_spread);

        let config = SpeedConfig {
            upload_min,
            upload_max,
            download_min,
            download_max,
            variation: 0,
        };

        let state = init_speed(&config);
        prop_assert!(state.base_upload >= upload_min && state.base_upload <= upload_max,
            "upload {} not in [{}, {}]", state.base_upload, upload_min, upload_max);
        prop_assert!(state.base_download >= download_min && state.base_download <= download_max,
            "download {} not in [{}, {}]", state.base_download, download_min, download_max);
        prop_assert_eq!(state.current_upload, state.base_upload);
        prop_assert_eq!(state.current_download, state.base_download);
    }

    // ---------------------------------------------------------------------------
    // Speed: vary_speed never underflows (result always >= 0)
    // ---------------------------------------------------------------------------

    #[test]
    fn vary_speed_no_underflow(
        base_upload in 0u64..=1_000_000u64,
        base_download in 0u64..=1_000_000u64,
        variation in 0u64..=1_000_000u64,
    ) {
        let config = SpeedConfig {
            upload_min: base_upload,
            upload_max: base_upload,
            download_min: base_download,
            download_max: base_download,
            variation,
        };

        let mut state = init_speed(&config);
        for _ in 0..10 {
            vary_speed(&mut state, &config);
            // current_upload and current_download are u64, so they can't be negative,
            // but we verify the logic doesn't panic or produce unexpected values
            prop_assert!(state.current_upload <= base_upload + variation,
                "upload {} > base {} + variation {}", state.current_upload, base_upload, variation);
        }
    }

    // ---------------------------------------------------------------------------
    // Peer ID: always returns exactly 20 bytes with correct prefix
    // ---------------------------------------------------------------------------

    #[test]
    fn peer_id_always_20_bytes(prefix_len in 0usize..=25) {
        let prefix: Vec<u8> = (0..prefix_len).map(|i| b'A' + (i as u8 % 26)).collect();

        let profile = ClientProfile {
            name: "Test".into(),
            family: ClientFamily::UTorrent,
            version: "1.0".into(),
            peer_id_prefix: prefix.clone(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Hex(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: String::new(),
            headers_template: String::new(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        };

        let peer_id = generate_peer_id(&profile);
        prop_assert_eq!(peer_id.len(), 20);

        // Verify prefix is correct (truncated to 20 if needed)
        let expected_prefix_len = prefix_len.min(20);
        prop_assert_eq!(&peer_id[..expected_prefix_len], &prefix[..expected_prefix_len]);
    }

    // ---------------------------------------------------------------------------
    // Key: always returns correct length and charset
    // ---------------------------------------------------------------------------

    #[test]
    fn key_correct_length_and_charset(
        key_len in 1usize..=32,
        format_idx in 0u8..3,
        uppercase in any::<bool>(),
    ) {
        let key_format = match format_idx {
            0 => KeyFormat::Numeric(key_len),
            1 => KeyFormat::Alphanumeric(key_len),
            _ => KeyFormat::Hex(key_len),
        };

        let profile = ClientProfile {
            name: "Test".into(),
            family: ClientFamily::UTorrent,
            version: "1.0".into(),
            peer_id_prefix: b"-UT1000-".to_vec(),
            peer_id_random_type: RandomType::Random,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format,
            key_uppercase: uppercase,
            http_protocol: "HTTP/1.1".into(),
            query_template: String::new(),
            headers_template: String::new(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        };

        let key = generate_key(&profile);
        prop_assert_eq!(key.len(), key_len, "key length mismatch");

        for ch in key.chars() {
            match format_idx {
                0 => prop_assert!(ch.is_ascii_digit(), "numeric key has non-digit '{ch}'"),
                1 => prop_assert!(ch.is_ascii_alphanumeric(), "alnum key has non-alnum '{ch}'"),
                _ => prop_assert!(ch.is_ascii_hexdigit(), "hex key has non-hex '{ch}'"),
            }
        }
    }
}

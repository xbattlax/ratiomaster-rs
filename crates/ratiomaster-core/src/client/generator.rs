/// Peer ID and session key generation for client emulation.
///
/// Generates 20-byte peer IDs and session keys matching the format
/// of each emulated BitTorrent client.
use rand::Rng;

use super::{ClientProfile, KeyFormat, RandomType};

/// Generates a 20-byte peer ID based on the client profile.
///
/// The peer ID consists of the profile's fixed prefix followed by random bytes
/// generated according to the profile's `peer_id_random_type`. The total is
/// always exactly 20 bytes.
pub fn generate_peer_id(profile: &ClientProfile) -> [u8; 20] {
    let mut rng = rand::thread_rng();
    let mut peer_id = [0u8; 20];

    let prefix = &profile.peer_id_prefix;
    let prefix_len = prefix.len().min(20);
    peer_id[..prefix_len].copy_from_slice(&prefix[..prefix_len]);

    let suffix_len = 20 - prefix_len;
    let suffix = generate_random_bytes(&mut rng, suffix_len, profile.peer_id_random_type);
    peer_id[prefix_len..].copy_from_slice(&suffix);

    peer_id
}

/// Generates a session key string based on the client profile.
pub fn generate_key(profile: &ClientProfile) -> String {
    let mut rng = rand::thread_rng();

    match profile.key_format {
        KeyFormat::Numeric(len) => (0..len)
            .map(|_| (b'0' + rng.gen_range(0..10)) as char)
            .collect(),
        KeyFormat::Alphanumeric(len) => {
            let chars: Vec<char> = if profile.key_uppercase {
                "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect()
            } else {
                "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
                    .chars()
                    .collect()
            };
            (0..len)
                .map(|_| chars[rng.gen_range(0..chars.len())])
                .collect()
        }
        KeyFormat::Hex(len) => {
            let hex_chars: &[u8] = if profile.key_uppercase {
                b"0123456789ABCDEF"
            } else {
                b"0123456789abcdef"
            };
            (0..len)
                .map(|_| hex_chars[rng.gen_range(0..16)] as char)
                .collect()
        }
    }
}

/// Generates random bytes of the specified type.
fn generate_random_bytes(rng: &mut impl Rng, len: usize, random_type: RandomType) -> Vec<u8> {
    match random_type {
        RandomType::Alphanumeric => {
            let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            (0..len)
                .map(|_| chars[rng.gen_range(0..chars.len())])
                .collect()
        }
        RandomType::Numeric => (0..len).map(|_| b'0' + rng.gen_range(0..10)).collect(),
        RandomType::Random => (0..len).map(|_| rng.gen::<u8>()).collect(),
        RandomType::Hex => {
            let hex = b"0123456789ABCDEF";
            (0..len).map(|_| hex[rng.gen_range(0..16)]).collect()
        }
    }
}

/// URL-encodes a peer ID, encoding non-unreserved bytes as `%XX`.
///
/// Used when `peer_id_url_encode` is true in the profile to produce the
/// percent-encoded form for tracker query strings.
pub fn url_encode_peer_id(peer_id: &[u8; 20], uppercase: bool) -> String {
    let mut result = String::with_capacity(60);
    for &byte in peer_id.iter() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            result.push(byte as char);
        } else if uppercase {
            result.push_str(&format!("%{:02X}", byte));
        } else {
            result.push_str(&format!("%{:02x}", byte));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{ClientFamily, KeyFormat, RandomType};

    fn utorrent_profile() -> ClientProfile {
        ClientProfile {
            name: "uTorrent 3.3.2".into(),
            family: ClientFamily::UTorrent,
            version: "3.3.2".into(),
            peer_id_prefix: b"-UT3320-".to_vec(),
            peer_id_random_type: RandomType::Random,
            peer_id_url_encode: true,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Hex(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: String::new(),
            headers_template: String::new(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: true,
        }
    }

    #[test]
    fn peer_id_length() {
        let profile = utorrent_profile();
        let peer_id = generate_peer_id(&profile);
        assert_eq!(peer_id.len(), 20);
    }

    #[test]
    fn peer_id_prefix() {
        let profile = utorrent_profile();
        let peer_id = generate_peer_id(&profile);
        assert_eq!(&peer_id[..8], b"-UT3320-");
    }

    #[test]
    fn peer_id_random_suffix_varies() {
        let profile = utorrent_profile();
        let id1 = generate_peer_id(&profile);
        let id2 = generate_peer_id(&profile);
        // Prefix should match, suffix should differ (with overwhelming probability)
        assert_eq!(&id1[..8], &id2[..8]);
        assert_ne!(&id1[8..], &id2[8..]);
    }

    #[test]
    fn peer_id_alphanumeric_suffix() {
        let mut profile = utorrent_profile();
        profile.peer_id_random_type = RandomType::Alphanumeric;
        let peer_id = generate_peer_id(&profile);
        for &b in &peer_id[8..] {
            assert!(
                b.is_ascii_alphanumeric(),
                "byte {b:#04x} is not alphanumeric"
            );
        }
    }

    #[test]
    fn peer_id_numeric_suffix() {
        let mut profile = utorrent_profile();
        profile.peer_id_random_type = RandomType::Numeric;
        let peer_id = generate_peer_id(&profile);
        for &b in &peer_id[8..] {
            assert!(b.is_ascii_digit(), "byte {b:#04x} is not a digit");
        }
    }

    #[test]
    fn peer_id_hex_suffix() {
        let mut profile = utorrent_profile();
        profile.peer_id_random_type = RandomType::Hex;
        let peer_id = generate_peer_id(&profile);
        for &b in &peer_id[8..] {
            assert!(b"0123456789ABCDEF".contains(&b), "byte {b:#04x} is not hex");
        }
    }

    #[test]
    fn key_hex_length_and_chars() {
        let profile = utorrent_profile();
        let key = generate_key(&profile);
        assert_eq!(key.len(), 8);
        for ch in key.chars() {
            assert!(
                "0123456789abcdef".contains(ch),
                "char '{ch}' not in lowercase hex"
            );
        }
    }

    #[test]
    fn key_hex_uppercase() {
        let mut profile = utorrent_profile();
        profile.key_format = KeyFormat::Hex(8);
        profile.key_uppercase = true;
        let key = generate_key(&profile);
        assert_eq!(key.len(), 8);
        for ch in key.chars() {
            assert!(
                "0123456789ABCDEF".contains(ch),
                "char '{ch}' not in uppercase hex"
            );
        }
    }

    #[test]
    fn key_numeric() {
        let mut profile = utorrent_profile();
        profile.key_format = KeyFormat::Numeric(5);
        let key = generate_key(&profile);
        assert_eq!(key.len(), 5);
        for ch in key.chars() {
            assert!(ch.is_ascii_digit(), "char '{ch}' is not a digit");
        }
    }

    #[test]
    fn key_alphanumeric() {
        let mut profile = utorrent_profile();
        profile.key_format = KeyFormat::Alphanumeric(8);
        profile.key_uppercase = false;
        let key = generate_key(&profile);
        assert_eq!(key.len(), 8);
        for ch in key.chars() {
            assert!(ch.is_ascii_alphanumeric(), "char '{ch}' not alphanumeric");
        }
    }

    #[test]
    fn key_alphanumeric_uppercase() {
        let mut profile = utorrent_profile();
        profile.key_format = KeyFormat::Alphanumeric(8);
        profile.key_uppercase = true;
        let key = generate_key(&profile);
        assert_eq!(key.len(), 8);
        for ch in key.chars() {
            assert!(
                ch.is_ascii_uppercase() || ch.is_ascii_digit(),
                "char '{ch}' not uppercase alphanumeric"
            );
        }
    }

    #[test]
    fn url_encode_peer_id_basic() {
        let mut peer_id = [0u8; 20];
        peer_id[..8].copy_from_slice(b"-UT3320-");
        peer_id[8] = 0xFF;
        peer_id[9] = 0x00;
        peer_id[10..].copy_from_slice(b"abcdefghij");

        let encoded = url_encode_peer_id(&peer_id, false);
        assert!(encoded.contains("-UT3320-"));
        assert!(encoded.contains("%ff"));
        assert!(encoded.contains("%00"));
        assert!(encoded.contains("abcdefghij"));
    }

    #[test]
    fn url_encode_peer_id_uppercase() {
        let mut peer_id = [0u8; 20];
        peer_id[0] = 0xFF;
        peer_id[1..].fill(b'A');

        let encoded = url_encode_peer_id(&peer_id, true);
        assert!(encoded.starts_with("%FF"));
    }

    #[test]
    fn key_varies_between_calls() {
        let profile = utorrent_profile();
        let k1 = generate_key(&profile);
        let k2 = generate_key(&profile);
        // With 8 hex chars, collision probability is ~1/4 billion
        assert_ne!(k1, k2);
    }

    #[test]
    fn long_prefix_truncated() {
        let mut profile = utorrent_profile();
        profile.peer_id_prefix = vec![b'X'; 25]; // longer than 20
        let peer_id = generate_peer_id(&profile);
        assert_eq!(peer_id.len(), 20);
        assert!(peer_id.iter().all(|&b| b == b'X'));
    }

    #[test]
    fn peer_id_20_bytes_all_profiles() {
        use crate::client::profiles::all_profiles;
        for profile in all_profiles() {
            let peer_id = generate_peer_id(profile);
            assert_eq!(
                peer_id.len(),
                20,
                "{}: peer_id length is {}",
                profile.name,
                peer_id.len()
            );
        }
    }

    #[test]
    fn peer_id_prefix_matches_all_profiles() {
        use crate::client::profiles::all_profiles;
        for profile in all_profiles() {
            let peer_id = generate_peer_id(profile);
            let prefix_len = profile.peer_id_prefix.len().min(20);
            assert_eq!(
                &peer_id[..prefix_len],
                &profile.peer_id_prefix[..prefix_len],
                "{}: prefix mismatch",
                profile.name
            );
        }
    }

    #[test]
    fn key_correct_length_all_profiles() {
        use crate::client::profiles::all_profiles;
        for profile in all_profiles() {
            let key = generate_key(profile);
            let expected_len = match profile.key_format {
                KeyFormat::Numeric(n) | KeyFormat::Alphanumeric(n) | KeyFormat::Hex(n) => n,
            };
            assert_eq!(
                key.len(),
                expected_len,
                "{}: key length {} != expected {}",
                profile.name,
                key.len(),
                expected_len
            );
        }
    }

    #[test]
    fn key_format_correct_all_profiles() {
        use crate::client::profiles::all_profiles;
        for profile in all_profiles() {
            let key = generate_key(profile);
            match profile.key_format {
                KeyFormat::Numeric(_) => {
                    for ch in key.chars() {
                        assert!(
                            ch.is_ascii_digit(),
                            "{}: numeric key contains '{}'",
                            profile.name,
                            ch
                        );
                    }
                }
                KeyFormat::Alphanumeric(_) => {
                    for ch in key.chars() {
                        assert!(
                            ch.is_ascii_alphanumeric(),
                            "{}: alphanumeric key contains '{}'",
                            profile.name,
                            ch
                        );
                    }
                }
                KeyFormat::Hex(_) => {
                    for ch in key.chars() {
                        assert!(
                            ch.is_ascii_hexdigit(),
                            "{}: hex key contains '{}'",
                            profile.name,
                            ch
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn url_encode_peer_id_preserves_unreserved() {
        let mut peer_id = [0u8; 20];
        // Fill with unreserved chars: a-z, 0-9
        for (i, b) in b"abcdefghij0123456789".iter().enumerate() {
            peer_id[i] = *b;
        }
        let encoded = url_encode_peer_id(&peer_id, false);
        assert_eq!(encoded, "abcdefghij0123456789");
    }

    #[test]
    fn url_encode_peer_id_encodes_all_special() {
        let mut peer_id = [0u8; 20];
        // Fill with bytes that need encoding
        for (i, slot) in peer_id.iter_mut().enumerate() {
            *slot = i as u8; // 0x00-0x13 all need encoding
        }
        let encoded_lower = url_encode_peer_id(&peer_id, false);
        let encoded_upper = url_encode_peer_id(&peer_id, true);
        // All 20 bytes should be encoded as %XX = 60 chars
        assert_eq!(encoded_lower.len(), 60);
        assert_eq!(encoded_upper.len(), 60);
        // Lowercase vs uppercase
        assert!(encoded_lower.contains("%00"));
        assert!(encoded_upper.contains("%00"));
        assert!(encoded_lower.contains("%0a"));
        assert!(encoded_upper.contains("%0A"));
    }

    #[test]
    fn peer_id_suffix_alphanumeric_valid_chars() {
        use crate::client::profiles::all_profiles;
        for profile in all_profiles()
            .iter()
            .filter(|p| p.peer_id_random_type == RandomType::Alphanumeric)
        {
            let peer_id = generate_peer_id(profile);
            let prefix_len = profile.peer_id_prefix.len().min(20);
            for &b in &peer_id[prefix_len..] {
                assert!(
                    b.is_ascii_alphanumeric(),
                    "{}: alphanumeric suffix byte {:#04x} not alphanumeric",
                    profile.name,
                    b
                );
            }
        }
    }

    #[test]
    fn peer_id_suffix_numeric_valid_chars() {
        use crate::client::profiles::all_profiles;
        for profile in all_profiles()
            .iter()
            .filter(|p| p.peer_id_random_type == RandomType::Numeric)
        {
            let peer_id = generate_peer_id(profile);
            let prefix_len = profile.peer_id_prefix.len().min(20);
            for &b in &peer_id[prefix_len..] {
                assert!(
                    b.is_ascii_digit(),
                    "{}: numeric suffix byte {:#04x} not a digit",
                    profile.name,
                    b
                );
            }
        }
    }

    #[test]
    fn peer_id_suffix_hex_valid_chars() {
        use crate::client::profiles::all_profiles;
        for profile in all_profiles()
            .iter()
            .filter(|p| p.peer_id_random_type == RandomType::Hex)
        {
            let peer_id = generate_peer_id(profile);
            let prefix_len = profile.peer_id_prefix.len().min(20);
            for &b in &peer_id[prefix_len..] {
                assert!(
                    b"0123456789ABCDEF".contains(&b),
                    "{}: hex suffix byte {:#04x} not in hex charset",
                    profile.name,
                    b
                );
            }
        }
    }
}

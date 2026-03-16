/// Static registry of all supported BitTorrent client profiles.
///
/// Contains 41 profiles across 16 client families, matching the behavior
/// of the original RatioMaster.NET client emulation.
use std::sync::OnceLock;

use super::{ClientFamily, ClientProfile, KeyFormat, RandomType};

static PROFILES: OnceLock<Vec<ClientProfile>> = OnceLock::new();

/// Returns all registered client profiles.
pub fn all_profiles() -> &'static [ClientProfile] {
    PROFILES.get_or_init(build_all_profiles)
}

/// Looks up a client profile by name (case-insensitive).
pub fn get_profile(name: &str) -> Option<&'static ClientProfile> {
    all_profiles()
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

fn build_all_profiles() -> Vec<ClientProfile> {
    let mut profiles = Vec::with_capacity(41);
    profiles.extend(utorrent_profiles());
    profiles.extend(bitcomet_profiles());
    profiles.extend(vuze_azureus_profiles());
    profiles.extend(other_profiles());
    profiles
}

// ---------------------------------------------------------------------------
// uTorrent (12 versions)
// ---------------------------------------------------------------------------

fn utorrent_profiles() -> Vec<ClientProfile> {
    let versions: &[(&str, &str, &str)] = &[
        ("uTorrent 3.3.2", "3.3.2", "-UT3320-"),
        ("uTorrent 3.3.0", "3.3.0", "-UT3300-"),
        ("uTorrent 3.2.0", "3.2.0", "-UT3200-"),
        ("uTorrent 2.0.1(19078)", "2.0.1", "-UT2010-"),
        ("uTorrent 1.8.5(17414)", "1.8.5", "-UT1850-"),
        ("uTorrent 1.8.1-beta(11903)", "1.8.1", "-UT1810-"),
        ("uTorrent 1.8.0", "1.8.0", "-UT1800-"),
        ("uTorrent 1.7.7", "1.7.7", "-UT1770-"),
        ("uTorrent 1.7.6", "1.7.6", "-UT1760-"),
        ("uTorrent 1.7.5", "1.7.5", "-UT1750-"),
        ("uTorrent 1.6.1", "1.6.1", "-UT1610-"),
        ("uTorrent 1.6", "1.6", "-UT1600-"),
    ];

    let build_numbers: &[(&str, &str)] = &[
        ("uTorrent 3.3.2", "33200"),
        ("uTorrent 3.3.0", "33000"),
        ("uTorrent 3.2.0", "32000"),
        ("uTorrent 2.0.1(19078)", "20100"),
        ("uTorrent 1.8.5(17414)", "18500"),
        ("uTorrent 1.8.1-beta(11903)", "18100"),
        ("uTorrent 1.8.0", "18000"),
        ("uTorrent 1.7.7", "17700"),
        ("uTorrent 1.7.6", "17600"),
        ("uTorrent 1.7.5", "17500"),
        ("uTorrent 1.6.1", "16100"),
        ("uTorrent 1.6", "16000"),
    ];

    versions
        .iter()
        .zip(build_numbers.iter())
        .map(|((name, version, prefix), (_, build))| ClientProfile {
            name: name.to_string(),
            family: ClientFamily::UTorrent,
            version: version.to_string(),
            peer_id_prefix: prefix.as_bytes().to_vec(),
            peer_id_random_type: RandomType::Random,
            peer_id_url_encode: true,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Hex(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                     &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                     &corrupt=0&key={key}{event}&numwant={numwant}\
                     &compact=1&no_peer_id=1"
                .into(),
            headers_template: format!("User-Agent: uTorrent/{build}\r\nAccept-Encoding: gzip"),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: true,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// BitComet (6 versions)
// ---------------------------------------------------------------------------

fn bitcomet_profiles() -> Vec<ClientProfile> {
    let versions: &[(&str, &str, &str, &str)] = &[
        ("BitComet 1.20", "1.20", "-BC0120-", "120"),
        ("BitComet 1.03", "1.03", "-BC0103-", "103"),
        ("BitComet 0.98", "0.98", "-BC0098-", "098"),
        ("BitComet 0.96", "0.96", "-BC0096-", "096"),
        ("BitComet 0.93", "0.93", "-BC0093-", "093"),
        ("BitComet 0.92", "0.92", "-BC0092-", "092"),
    ];

    versions
        .iter()
        .map(|(name, version, prefix, ver_num)| ClientProfile {
            name: name.to_string(),
            family: ClientFamily::BitComet,
            version: version.to_string(),
            peer_id_prefix: prefix.as_bytes().to_vec(),
            peer_id_random_type: RandomType::Random,
            peer_id_url_encode: true,
            peer_id_url_encode_uppercase: true,
            key_format: KeyFormat::Numeric(5),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                     &natmapped=1&localip={localip}&port_type=wan\
                     &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                     &key={key}{event}&numwant={numwant}&compact=1&no_peer_id=1"
                .into(),
            headers_template: format!("User-Agent: BitComet/{ver_num}\r\nAccept-Encoding: gzip"),
            hash_uppercase: true,
            default_numwant: 200,
            compact: true,
            no_peer_id: true,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Vuze / Azureus (7 versions)
// ---------------------------------------------------------------------------

fn vuze_azureus_profiles() -> Vec<ClientProfile> {
    let versions: &[(&str, ClientFamily, &str, &str, &str)] = &[
        (
            "Vuze 4.2.0.8",
            ClientFamily::Vuze,
            "4.2.0.8",
            "-AZ4208-",
            "4208",
        ),
        (
            "Azureus 3.1.1.0",
            ClientFamily::Azureus,
            "3.1.1.0",
            "-AZ3110-",
            "3110",
        ),
        (
            "Azureus 3.0.5.0",
            ClientFamily::Azureus,
            "3.0.5.0",
            "-AZ3050-",
            "3050",
        ),
        (
            "Azureus 3.0.4.2",
            ClientFamily::Azureus,
            "3.0.4.2",
            "-AZ3042-",
            "3042",
        ),
        (
            "Azureus 3.0.3.4",
            ClientFamily::Azureus,
            "3.0.3.4",
            "-AZ3034-",
            "3034",
        ),
        (
            "Azureus 3.0.2.2",
            ClientFamily::Azureus,
            "3.0.2.2",
            "-AZ3022-",
            "3022",
        ),
        (
            "Azureus 2.5.0.4",
            ClientFamily::Azureus,
            "2.5.0.4",
            "-AZ2504-",
            "2504",
        ),
    ];

    versions
        .iter()
        .map(|(name, family, version, prefix, ver_num)| ClientProfile {
            name: name.to_string(),
            family: *family,
            version: version.to_string(),
            peer_id_prefix: prefix.as_bytes().to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                     &azudp={port}&uploaded={uploaded}&downloaded={downloaded}\
                     &left={left}&corrupt=0&key={key}{event}\
                     &numwant={numwant}&no_peer_id=1&compact=1\
                     &supportcrypto=1&azver=3"
                .into(),
            headers_template: format!(
                "User-Agent: Azureus {ver_num};{{}}\r\nAccept-Encoding: gzip"
            ),
            hash_uppercase: true,
            default_numwant: 50,
            compact: true,
            no_peer_id: true,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Remaining 16 clients
// ---------------------------------------------------------------------------

fn other_profiles() -> Vec<ClientProfile> {
    vec![
        // BitTorrent 6.0.3(8642)
        ClientProfile {
            name: "BitTorrent 6.0.3(8642)".into(),
            family: ClientFamily::BitTorrent,
            version: "6.0.3".into(),
            peer_id_prefix: b"M6-0-3--".to_vec(),
            peer_id_random_type: RandomType::Random,
            peer_id_url_encode: true,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Hex(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &corrupt=0&key={key}{event}&numwant={numwant}\
                 &compact=1&no_peer_id=1"
                .into(),
            headers_template: "User-Agent: BitTorrent/8642\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: true,
        },
        // Transmission 2.82(14160)
        ClientProfile {
            name: "Transmission 2.82(14160)".into(),
            family: ClientFamily::Transmission,
            version: "2.82".into(),
            peer_id_prefix: b"-TR2500-".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Hex(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &numwant={numwant}&key={key}&compact=1&supportcrypto=1{event}"
                .into(),
            headers_template: "User-Agent: Transmission/2.82\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 80,
            compact: true,
            no_peer_id: false,
        },
        // Transmission 2.92(14714)
        ClientProfile {
            name: "Transmission 2.92(14714)".into(),
            family: ClientFamily::Transmission,
            version: "2.92".into(),
            peer_id_prefix: b"-TR2920-".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Hex(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &numwant={numwant}&key={key}&compact=1&supportcrypto=1{event}"
                .into(),
            headers_template: "User-Agent: Transmission/2.92\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 80,
            compact: true,
            no_peer_id: false,
        },
        // Deluge 1.2.0
        ClientProfile {
            name: "Deluge 1.2.0".into(),
            family: ClientFamily::Deluge,
            version: "1.2.0".into(),
            peer_id_prefix: b"-DE1200-".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.0".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1&supportcrypto=1"
                .into(),
            headers_template: "User-Agent: Deluge 1.2.0\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // Deluge 0.5.8.7
        ClientProfile {
            name: "Deluge 0.5.8.7".into(),
            family: ClientFamily::Deluge,
            version: "0.5.8.7".into(),
            peer_id_prefix: b"-DE0587-".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(8),
            key_uppercase: true,
            http_protocol: "HTTP/1.0".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: Deluge 0.5.8.7\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // Deluge 0.5.8.6
        ClientProfile {
            name: "Deluge 0.5.8.6".into(),
            family: ClientFamily::Deluge,
            version: "0.5.8.6".into(),
            peer_id_prefix: b"-DE0586-".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(8),
            key_uppercase: true,
            http_protocol: "HTTP/1.0".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: Deluge 0.5.8.6\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // BitLord 1.1
        ClientProfile {
            name: "BitLord 1.1".into(),
            family: ClientFamily::BitLord,
            version: "1.1".into(),
            // exbc\x01\x01LORDCz\x03\x92 (14 bytes prefix)
            peer_id_prefix: vec![
                b'e', b'x', b'b', b'c', 0x01, 0x01, b'L', b'O', b'R', b'D', b'C', b'z', 0x03, 0x92,
            ],
            peer_id_random_type: RandomType::Random,
            peer_id_url_encode: true,
            peer_id_url_encode_uppercase: true,
            key_format: KeyFormat::Numeric(4),
            key_uppercase: false,
            http_protocol: "HTTP/1.0".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &natmapped=1&localip={localip}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: BitLord 1.1\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: true,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // ABC 3.1
        ClientProfile {
            name: "ABC 3.1".into(),
            family: ClientFamily::ABC,
            version: "3.1".into(),
            peer_id_prefix: b"A310--".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(6),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1&trackerid=48"
                .into(),
            headers_template: "User-Agent: ABC/3.1\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: true,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // BTuga 2.1.8
        ClientProfile {
            name: "BTuga 2.1.8".into(),
            family: ClientFamily::BTuga,
            version: "2.1.8".into(),
            peer_id_prefix: b"R26---".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(6),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: BTuga/2.1.8\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: true,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // BitTornado 0.3.17
        ClientProfile {
            name: "BitTornado 0.3.17".into(),
            family: ClientFamily::BitTornado,
            version: "0.3.17".into(),
            peer_id_prefix: b"T03H-----".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(6),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: BitTornado/T-0.3.17\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: true,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // Burst 3.1.0b
        ClientProfile {
            name: "Burst 3.1.0b".into(),
            family: ClientFamily::Burst,
            version: "3.1.0b".into(),
            peer_id_prefix: b"Mbrst1-1-3".to_vec(),
            peer_id_random_type: RandomType::Hex,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Hex(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.0".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: Burst/3.1.0b\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: true,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // BitTyrant 1.1
        ClientProfile {
            name: "BitTyrant 1.1".into(),
            family: ClientFamily::BitTyrant,
            version: "1.1".into(),
            peer_id_prefix: b"AZ2500BT".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &azudp={port}&uploaded={uploaded}&downloaded={downloaded}\
                 &left={left}&corrupt=0&key={key}{event}\
                 &numwant={numwant}&no_peer_id=1&compact=1&supportcrypto=1"
                .into(),
            headers_template: "User-Agent: BitTyrant/1.1\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: true,
            default_numwant: 50,
            compact: true,
            no_peer_id: true,
        },
        // BitSpirit 3.6.0.200
        ClientProfile {
            name: "BitSpirit 3.6.0.200".into(),
            family: ClientFamily::BitSpirit,
            version: "3.6.0.200".into(),
            // %2dSP3602 = "-SP3602" (using 0x2d for '-')
            peer_id_prefix: vec![0x2d, b'S', b'P', b'3', b'6', b'0', b'2'],
            peer_id_random_type: RandomType::Random,
            peer_id_url_encode: true,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Hex(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.0".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: BitSpirit/3.6.0.200\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // BitSpirit 3.1.0.077
        ClientProfile {
            name: "BitSpirit 3.1.0.077".into(),
            family: ClientFamily::BitSpirit,
            version: "3.1.0.077".into(),
            // %00%03BS = \x00\x03BS
            peer_id_prefix: vec![0x00, 0x03, b'B', b'S'],
            peer_id_random_type: RandomType::Random,
            peer_id_url_encode: true,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Numeric(3),
            key_uppercase: false,
            http_protocol: "HTTP/1.0".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: BitSpirit/3.1.0.077\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 200,
            compact: true,
            no_peer_id: false,
        },
        // KTorrent 2.2.1
        ClientProfile {
            name: "KTorrent 2.2.1".into(),
            family: ClientFamily::KTorrent,
            version: "2.2.1".into(),
            peer_id_prefix: b"-KT2210-".to_vec(),
            peer_id_random_type: RandomType::Numeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Numeric(10),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: KTorrent/2.2.1\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: false,
            default_numwant: 100,
            compact: true,
            no_peer_id: false,
        },
        // Gnome BT 0.0.28-1
        ClientProfile {
            name: "Gnome BT 0.0.28-1".into(),
            family: ClientFamily::GnomeBT,
            version: "0.0.28-1".into(),
            peer_id_prefix: b"M3-4-2--".to_vec(),
            peer_id_random_type: RandomType::Alphanumeric,
            peer_id_url_encode: false,
            peer_id_url_encode_uppercase: false,
            key_format: KeyFormat::Alphanumeric(8),
            key_uppercase: false,
            http_protocol: "HTTP/1.1".into(),
            query_template: "info_hash={infohash}&peer_id={peerid}&port={port}\
                 &uploaded={uploaded}&downloaded={downloaded}&left={left}\
                 &key={key}{event}&numwant={numwant}&compact=1"
                .into(),
            headers_template: "User-Agent: GnomeBT/0.0.28-1\r\nAccept-Encoding: gzip".into(),
            hash_uppercase: true,
            default_numwant: 100,
            compact: true,
            no_peer_id: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_profile_count() {
        assert_eq!(all_profiles().len(), 41);
    }

    #[test]
    fn utorrent_count() {
        let count = all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::UTorrent)
            .count();
        assert_eq!(count, 12);
    }

    #[test]
    fn bitcomet_count() {
        let count = all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::BitComet)
            .count();
        assert_eq!(count, 6);
    }

    #[test]
    fn vuze_azureus_count() {
        let count = all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::Vuze || p.family == ClientFamily::Azureus)
            .count();
        assert_eq!(count, 7);
    }

    #[test]
    fn other_profiles_count() {
        let count = all_profiles()
            .iter()
            .filter(|p| {
                !matches!(
                    p.family,
                    ClientFamily::UTorrent
                        | ClientFamily::BitComet
                        | ClientFamily::Vuze
                        | ClientFamily::Azureus
                )
            })
            .count();
        assert_eq!(count, 16);
    }

    #[test]
    fn unique_names() {
        let names: Vec<&str> = all_profiles().iter().map(|p| p.name.as_str()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len(), "duplicate profile names found");
    }

    #[test]
    fn get_profile_by_name() {
        let profile = get_profile("uTorrent 3.3.2").unwrap();
        assert_eq!(profile.family, ClientFamily::UTorrent);
        assert_eq!(profile.version, "3.3.2");
    }

    #[test]
    fn get_profile_case_insensitive() {
        let profile = get_profile("UTORRENT 3.3.2").unwrap();
        assert_eq!(profile.family, ClientFamily::UTorrent);
    }

    #[test]
    fn get_profile_not_found() {
        assert!(get_profile("NonExistent Client").is_none());
    }

    #[test]
    fn utorrent_profiles_common_settings() {
        for profile in all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::UTorrent)
        {
            assert_eq!(profile.http_protocol, "HTTP/1.1", "{}", profile.name);
            assert!(!profile.hash_uppercase, "{}", profile.name);
            assert_eq!(profile.key_format, KeyFormat::Hex(8), "{}", profile.name);
            assert_eq!(profile.default_numwant, 200, "{}", profile.name);
            assert!(profile.compact, "{}", profile.name);
            assert!(profile.no_peer_id, "{}", profile.name);
            assert_eq!(
                profile.peer_id_random_type,
                RandomType::Random,
                "{}",
                profile.name
            );
            assert!(profile.peer_id_url_encode, "{}", profile.name);
            assert!(!profile.peer_id_url_encode_uppercase, "{}", profile.name);
        }
    }

    #[test]
    fn bitcomet_profiles_common_settings() {
        for profile in all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::BitComet)
        {
            assert!(profile.hash_uppercase, "{}", profile.name);
            assert_eq!(
                profile.key_format,
                KeyFormat::Numeric(5),
                "{}",
                profile.name
            );
            assert_eq!(profile.default_numwant, 200, "{}", profile.name);
            assert!(
                profile.query_template.contains("natmapped=1"),
                "{}",
                profile.name
            );
        }
    }

    #[test]
    fn vuze_profiles_common_settings() {
        for profile in all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::Vuze || p.family == ClientFamily::Azureus)
        {
            assert!(profile.hash_uppercase, "{}", profile.name);
            assert_eq!(
                profile.key_format,
                KeyFormat::Alphanumeric(8),
                "{}",
                profile.name
            );
            assert_eq!(profile.default_numwant, 50, "{}", profile.name);
            assert!(
                profile.query_template.contains("supportcrypto=1"),
                "{}",
                profile.name
            );
        }
    }

    #[test]
    fn peer_id_prefix_valid_length() {
        for profile in all_profiles() {
            assert!(
                profile.peer_id_prefix.len() <= 20,
                "{}: prefix len {} > 20",
                profile.name,
                profile.peer_id_prefix.len()
            );
        }
    }

    #[test]
    fn all_profiles_have_query_template() {
        for profile in all_profiles() {
            assert!(
                profile.query_template.contains("{infohash}"),
                "{}: missing infohash placeholder",
                profile.name
            );
            assert!(
                profile.query_template.contains("{peerid}"),
                "{}: missing peerid placeholder",
                profile.name
            );
        }
    }

    #[test]
    fn bitlord_prefix() {
        let profile = get_profile("BitLord 1.1").unwrap();
        assert_eq!(profile.peer_id_prefix.len(), 14);
        assert_eq!(&profile.peer_id_prefix[..4], b"exbc");
        assert_eq!(profile.peer_id_prefix[4], 0x01);
        assert_eq!(profile.peer_id_prefix[5], 0x01);
        assert_eq!(&profile.peer_id_prefix[6..10], b"LORD");
    }

    #[test]
    fn bitspirit_old_prefix() {
        let profile = get_profile("BitSpirit 3.1.0.077").unwrap();
        assert_eq!(profile.peer_id_prefix[0], 0x00);
        assert_eq!(profile.peer_id_prefix[1], 0x03);
        assert_eq!(&profile.peer_id_prefix[2..4], b"BS");
    }

    #[test]
    fn transmission_supportcrypto() {
        for name in &["Transmission 2.82(14160)", "Transmission 2.92(14714)"] {
            let profile = get_profile(name).unwrap();
            assert!(profile.query_template.contains("supportcrypto=1"), "{name}");
        }
    }

    #[test]
    fn deluge_http10() {
        for name in &["Deluge 1.2.0", "Deluge 0.5.8.7", "Deluge 0.5.8.6"] {
            let profile = get_profile(name).unwrap();
            assert_eq!(profile.http_protocol, "HTTP/1.0", "{name}");
        }
    }

    #[test]
    fn ktorrent_numeric_peer_id() {
        let profile = get_profile("KTorrent 2.2.1").unwrap();
        assert_eq!(profile.peer_id_random_type, RandomType::Numeric);
        assert_eq!(profile.key_format, KeyFormat::Numeric(10));
    }

    #[test]
    fn all_41_profiles_loadable_by_name() {
        let names: Vec<String> = all_profiles().iter().map(|p| p.name.clone()).collect();
        assert_eq!(names.len(), 41);
        for name in &names {
            assert!(
                get_profile(name).is_some(),
                "profile '{}' not found by name",
                name
            );
        }
    }

    #[test]
    fn all_profiles_have_valid_http_protocol() {
        for profile in all_profiles() {
            assert!(
                profile.http_protocol == "HTTP/1.0" || profile.http_protocol == "HTTP/1.1",
                "{}: invalid http_protocol '{}'",
                profile.name,
                profile.http_protocol
            );
        }
    }

    #[test]
    fn all_profiles_have_port_placeholder() {
        for profile in all_profiles() {
            assert!(
                profile.query_template.contains("{port}"),
                "{}: missing {{port}} placeholder",
                profile.name
            );
        }
    }

    #[test]
    fn all_profiles_have_uploaded_placeholder() {
        for profile in all_profiles() {
            assert!(
                profile.query_template.contains("{uploaded}"),
                "{}: missing {{uploaded}} placeholder",
                profile.name
            );
        }
    }

    #[test]
    fn all_profiles_have_downloaded_placeholder() {
        for profile in all_profiles() {
            assert!(
                profile.query_template.contains("{downloaded}"),
                "{}: missing {{downloaded}} placeholder",
                profile.name
            );
        }
    }

    #[test]
    fn all_profiles_have_left_placeholder() {
        for profile in all_profiles() {
            assert!(
                profile.query_template.contains("{left}"),
                "{}: missing {{left}} placeholder",
                profile.name
            );
        }
    }

    #[test]
    fn all_profiles_have_key_placeholder() {
        for profile in all_profiles() {
            assert!(
                profile.query_template.contains("{key}"),
                "{}: missing {{key}} placeholder",
                profile.name
            );
        }
    }

    #[test]
    fn all_profiles_have_event_placeholder() {
        for profile in all_profiles() {
            assert!(
                profile.query_template.contains("{event}"),
                "{}: missing {{event}} placeholder",
                profile.name
            );
        }
    }

    #[test]
    fn all_profiles_have_numwant_placeholder() {
        for profile in all_profiles() {
            assert!(
                profile.query_template.contains("{numwant}"),
                "{}: missing {{numwant}} placeholder",
                profile.name
            );
        }
    }

    #[test]
    fn all_profiles_nonzero_numwant() {
        for profile in all_profiles() {
            assert!(
                profile.default_numwant > 0,
                "{}: default_numwant is 0",
                profile.name
            );
        }
    }

    #[test]
    fn all_profiles_nonempty_name() {
        for profile in all_profiles() {
            assert!(!profile.name.is_empty(), "profile has empty name");
        }
    }

    #[test]
    fn all_profiles_nonempty_version() {
        for profile in all_profiles() {
            assert!(
                !profile.version.is_empty(),
                "{}: has empty version",
                profile.name
            );
        }
    }

    #[test]
    fn family_coverage_all_16() {
        use std::collections::HashSet;
        let families: HashSet<ClientFamily> = all_profiles().iter().map(|p| p.family).collect();
        assert_eq!(families.len(), 16);
    }

    #[test]
    fn utorrent_peer_id_prefixes_start_with_ut() {
        for profile in all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::UTorrent)
        {
            let prefix_str = std::str::from_utf8(&profile.peer_id_prefix).unwrap();
            assert!(
                prefix_str.starts_with("-UT"),
                "{}: prefix '{}' doesn't start with -UT",
                profile.name,
                prefix_str
            );
        }
    }

    #[test]
    fn bitcomet_peer_id_prefixes_start_with_bc() {
        for profile in all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::BitComet)
        {
            let prefix_str = std::str::from_utf8(&profile.peer_id_prefix).unwrap();
            assert!(
                prefix_str.starts_with("-BC"),
                "{}: prefix '{}' doesn't start with -BC",
                profile.name,
                prefix_str
            );
        }
    }

    #[test]
    fn azureus_peer_id_prefixes_start_with_az() {
        for profile in all_profiles()
            .iter()
            .filter(|p| p.family == ClientFamily::Azureus || p.family == ClientFamily::Vuze)
        {
            let prefix_str = std::str::from_utf8(&profile.peer_id_prefix).unwrap();
            assert!(
                prefix_str.starts_with("-AZ"),
                "{}: prefix '{}' doesn't start with -AZ",
                profile.name,
                prefix_str
            );
        }
    }
}

use sha1::{Digest, Sha1};

use crate::bencode::{decode, encode};

use super::error::TorrentError;
use super::types::{TorrentFile, TorrentMetainfo};

/// Parses a .torrent file from its raw bytes.
///
/// Extracts all metadata including the announce URL, file information,
/// piece hashes, and computes the SHA1 info_hash from the raw bencoded
/// info dictionary.
///
/// # Examples
///
/// ```no_run
/// use ratiomaster_core::torrent::parse;
///
/// let torrent_bytes = std::fs::read("example.torrent").unwrap();
/// let metainfo = parse(&torrent_bytes).unwrap();
/// println!("Name: {}", metainfo.name);
/// println!("Info hash: {:x?}", metainfo.info_hash);
/// ```
pub fn parse(data: &[u8]) -> Result<TorrentMetainfo, TorrentError> {
    let root = decode(data).map_err(|e| TorrentError::Bencode(e.to_string()))?;

    if root.as_dict().is_none() {
        return Err(TorrentError::InvalidStructure(
            "root is not a dictionary".into(),
        ));
    }

    // announce (required)
    let announce = root
        .dict_get("announce")
        .and_then(|v| v.as_str())
        .ok_or(TorrentError::MissingField("announce".into()))?
        .to_string();

    // announce-list (optional, BEP 12)
    let announce_list = root.dict_get("announce-list").and_then(|v| {
        v.as_list().map(|tiers| {
            tiers
                .iter()
                .filter_map(|tier| {
                    tier.as_list().map(|urls| {
                        urls.iter()
                            .filter_map(|u| u.as_str().map(String::from))
                            .collect()
                    })
                })
                .collect()
        })
    });

    // info dict (required)
    let info = root
        .dict_get("info")
        .ok_or(TorrentError::MissingField("info".into()))?;

    let info_dict = info.as_dict().ok_or(TorrentError::InvalidStructure(
        "info is not a dictionary".into(),
    ))?;

    // Compute info_hash from the raw bencoded info dict
    let info_bytes = encode(info);
    let info_hash: [u8; 20] = Sha1::digest(&info_bytes).into();

    // name (required)
    let name = info
        .dict_get("name")
        .and_then(|v| v.as_str())
        .ok_or(TorrentError::MissingField("info.name".into()))?
        .to_string();

    // piece length (required)
    let piece_length =
        info.dict_get("piece length")
            .and_then(|v| v.as_integer())
            .ok_or(TorrentError::MissingField("info.piece length".into()))? as u64;

    // pieces (required) - concatenated 20-byte SHA1 hashes
    let pieces = info
        .dict_get("pieces")
        .and_then(|v| v.as_bytes())
        .ok_or(TorrentError::MissingField("info.pieces".into()))?
        .to_vec();

    if pieces.len() % 20 != 0 {
        return Err(TorrentError::InvalidStructure(format!(
            "pieces length {} is not a multiple of 20",
            pieces.len()
        )));
    }

    // Single-file vs multi-file
    let length = info
        .dict_get("length")
        .and_then(|v| v.as_integer())
        .map(|n| n as u64);

    let files = if length.is_none() {
        let files_list = info_dict
            .get(b"files".as_slice())
            .and_then(|v| v.as_list())
            .ok_or(TorrentError::MissingField(
                "info.length or info.files".into(),
            ))?;

        let mut parsed_files = Vec::new();
        for file_val in files_list {
            let file_dict = file_val.as_dict().ok_or(TorrentError::InvalidStructure(
                "file entry is not a dictionary".into(),
            ))?;

            let file_length = file_val
                .dict_get("length")
                .and_then(|v| v.as_integer())
                .ok_or(TorrentError::MissingField("file.length".into()))?
                as u64;

            let path_list = file_dict
                .get(b"path".as_slice())
                .and_then(|v| v.as_list())
                .ok_or(TorrentError::MissingField("file.path".into()))?;

            let path: Vec<String> = path_list
                .iter()
                .filter_map(|p| p.as_str().map(String::from))
                .collect();

            if path.is_empty() {
                return Err(TorrentError::InvalidStructure("empty file path".into()));
            }

            parsed_files.push(TorrentFile {
                path,
                length: file_length,
            });
        }

        Some(parsed_files)
    } else {
        None
    };

    // Optional fields
    let comment = root
        .dict_get("comment")
        .and_then(|v| v.as_str())
        .map(String::from);
    let created_by = root
        .dict_get("created by")
        .and_then(|v| v.as_str())
        .map(String::from);
    let creation_date = root.dict_get("creation date").and_then(|v| v.as_integer());

    Ok(TorrentMetainfo {
        announce,
        announce_list,
        name,
        piece_length,
        pieces,
        length,
        files,
        comment,
        created_by,
        creation_date,
        info_hash,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::bencode::{encode, BValue};

    use super::*;

    /// Helper to build a minimal single-file torrent as bencoded bytes.
    fn make_single_file_torrent(
        announce: &str,
        name: &str,
        piece_length: i64,
        length: i64,
    ) -> Vec<u8> {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(length));
        info.insert(b"name".to_vec(), BValue::String(name.as_bytes().to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(piece_length));
        // 1 piece = 20 bytes of SHA1
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xaa; 20]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(announce.as_bytes().to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        encode(&BValue::Dict(root))
    }

    /// Helper to build a multi-file torrent as bencoded bytes.
    fn make_multi_file_torrent(
        announce: &str,
        name: &str,
        piece_length: i64,
        files: &[(&[&str], i64)],
    ) -> Vec<u8> {
        let file_entries: Vec<BValue> = files
            .iter()
            .map(|(path, length)| {
                let mut file = BTreeMap::new();
                file.insert(b"length".to_vec(), BValue::Integer(*length));
                file.insert(
                    b"path".to_vec(),
                    BValue::List(
                        path.iter()
                            .map(|p| BValue::String(p.as_bytes().to_vec()))
                            .collect(),
                    ),
                );
                BValue::Dict(file)
            })
            .collect();

        let mut info = BTreeMap::new();
        info.insert(b"files".to_vec(), BValue::List(file_entries));
        info.insert(b"name".to_vec(), BValue::String(name.as_bytes().to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(piece_length));
        // Enough pieces for the total size
        let total: i64 = files.iter().map(|(_, l)| l).sum();
        let piece_count = (total as u64).div_ceil(piece_length as u64) as usize;
        info.insert(
            b"pieces".to_vec(),
            BValue::String(vec![0xbb; piece_count * 20]),
        );

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(announce.as_bytes().to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        encode(&BValue::Dict(root))
    }

    #[test]
    fn parse_single_file_torrent() {
        let data = make_single_file_torrent(
            "http://tracker.example.com/announce",
            "test.txt",
            262144,
            1024,
        );
        let meta = parse(&data).unwrap();

        assert_eq!(meta.announce, "http://tracker.example.com/announce");
        assert_eq!(meta.name, "test.txt");
        assert_eq!(meta.piece_length, 262144);
        assert_eq!(meta.length, Some(1024));
        assert!(meta.files.is_none());
        assert!(meta.is_single_file());
        assert!(!meta.is_multi_file());
        assert_eq!(meta.total_size(), 1024);
        assert_eq!(meta.piece_count(), 1);
        assert_eq!(meta.pieces.len(), 20);
    }

    #[test]
    fn parse_multi_file_torrent() {
        let files = &[
            (&["dir", "file1.txt"][..], 1000),
            (&["dir", "file2.txt"][..], 2000),
        ];
        let data = make_multi_file_torrent(
            "http://tracker.example.com/announce",
            "my_torrent",
            262144,
            files,
        );
        let meta = parse(&data).unwrap();

        assert_eq!(meta.name, "my_torrent");
        assert!(meta.length.is_none());
        assert!(meta.is_multi_file());
        assert!(!meta.is_single_file());
        assert_eq!(meta.total_size(), 3000);

        let parsed_files = meta.files.as_ref().unwrap();
        assert_eq!(parsed_files.len(), 2);
        assert_eq!(parsed_files[0].path, vec!["dir", "file1.txt"]);
        assert_eq!(parsed_files[0].length, 1000);
        assert_eq!(parsed_files[1].path, vec!["dir", "file2.txt"]);
        assert_eq!(parsed_files[1].length, 2000);
    }

    #[test]
    fn info_hash_is_correct() {
        let data = make_single_file_torrent(
            "http://tracker.example.com/announce",
            "test.txt",
            262144,
            1024,
        );
        let meta = parse(&data).unwrap();

        // Manually compute expected info_hash
        let root = decode(&data).unwrap();
        let info = root.dict_get("info").unwrap();
        let info_bytes = encode(info);
        let expected: [u8; 20] = Sha1::digest(&info_bytes).into();

        assert_eq!(meta.info_hash, expected);
    }

    #[test]
    fn parse_with_optional_fields() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(512));
        info.insert(b"name".to_vec(), BValue::String(b"test.bin".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(65536));
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xcc; 20]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(
            b"comment".to_vec(),
            BValue::String(b"This is a test torrent".to_vec()),
        );
        root.insert(
            b"created by".to_vec(),
            BValue::String(b"ratiomaster-test".to_vec()),
        );
        root.insert(b"creation date".to_vec(), BValue::Integer(1700000000));
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        let meta = parse(&data).unwrap();

        assert_eq!(meta.comment.as_deref(), Some("This is a test torrent"));
        assert_eq!(meta.created_by.as_deref(), Some("ratiomaster-test"));
        assert_eq!(meta.creation_date, Some(1700000000));
    }

    #[test]
    fn parse_with_announce_list() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(100));
        info.insert(b"name".to_vec(), BValue::String(b"test".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(65536));
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xdd; 20]));

        let announce_list = BValue::List(vec![
            BValue::List(vec![
                BValue::String(b"http://tracker1.example.com/announce".to_vec()),
                BValue::String(b"http://tracker2.example.com/announce".to_vec()),
            ]),
            BValue::List(vec![BValue::String(
                b"http://backup.example.com/announce".to_vec(),
            )]),
        ]);

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker1.example.com/announce".to_vec()),
        );
        root.insert(b"announce-list".to_vec(), announce_list);
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        let meta = parse(&data).unwrap();

        let tiers = meta.announce_list.unwrap();
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].len(), 2);
        assert_eq!(tiers[1].len(), 1);
    }

    #[test]
    fn parse_missing_announce_fails() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(100));
        info.insert(b"name".to_vec(), BValue::String(b"test".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(65536));
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xee; 20]));

        let mut root = BTreeMap::new();
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        assert!(matches!(parse(&data), Err(TorrentError::MissingField(_))));
    }

    #[test]
    fn parse_invalid_pieces_length_fails() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(100));
        info.insert(b"name".to_vec(), BValue::String(b"test".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(65536));
        // 15 bytes is not a multiple of 20
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xff; 15]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        assert!(matches!(
            parse(&data),
            Err(TorrentError::InvalidStructure(_))
        ));
    }

    #[test]
    fn parse_torrent_no_optional_fields() {
        let data = make_single_file_torrent(
            "http://tracker.example.com/announce",
            "bare.txt",
            262144,
            5000,
        );
        let meta = parse(&data).unwrap();
        assert!(meta.comment.is_none());
        assert!(meta.created_by.is_none());
        assert!(meta.creation_date.is_none());
        assert!(meta.announce_list.is_none());
    }

    #[test]
    fn parse_torrent_with_every_optional_field() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(10000));
        info.insert(b"name".to_vec(), BValue::String(b"full.txt".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(262144));
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xaa; 20]));

        let announce_list = BValue::List(vec![
            BValue::List(vec![BValue::String(
                b"http://tracker1.example.com/announce".to_vec(),
            )]),
            BValue::List(vec![BValue::String(
                b"http://tracker2.example.com/announce".to_vec(),
            )]),
        ]);

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker1.example.com/announce".to_vec()),
        );
        root.insert(b"announce-list".to_vec(), announce_list);
        root.insert(
            b"comment".to_vec(),
            BValue::String(b"A full torrent with all fields".to_vec()),
        );
        root.insert(
            b"created by".to_vec(),
            BValue::String(b"ratiomaster-test v1.0".to_vec()),
        );
        root.insert(b"creation date".to_vec(), BValue::Integer(1700000000));
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        let meta = parse(&data).unwrap();

        assert_eq!(meta.announce, "http://tracker1.example.com/announce");
        assert!(meta.announce_list.is_some());
        assert_eq!(meta.announce_list.as_ref().unwrap().len(), 2);
        assert_eq!(
            meta.comment.as_deref(),
            Some("A full torrent with all fields")
        );
        assert_eq!(meta.created_by.as_deref(), Some("ratiomaster-test v1.0"));
        assert_eq!(meta.creation_date, Some(1700000000));
        assert_eq!(meta.name, "full.txt");
        assert_eq!(meta.length, Some(10000));
    }

    #[test]
    fn parse_very_large_file_size() {
        // Multi-TB file: 5 TB = 5_497_558_138_880 bytes
        let size: i64 = 5_497_558_138_880;
        let data = make_single_file_torrent(
            "http://tracker.example.com/announce",
            "huge.iso",
            4194304, // 4MB pieces
            size,
        );
        let meta = parse(&data).unwrap();
        assert_eq!(meta.total_size(), size as u64);
        assert_eq!(meta.length, Some(size as u64));
    }

    #[test]
    fn parse_many_files_torrent() {
        // 120 files
        let files: Vec<(&[&str], i64)> = (0..120)
            .map(|i| {
                // We need stable references, so use leaked strings
                let name: &'static str = Box::leak(format!("file_{:04}.dat", i).into_boxed_str());
                let path: &'static [&'static str] =
                    Box::leak(vec!["data", name].into_boxed_slice());
                (path as &[&str], 1024i64 * (i + 1))
            })
            .collect();

        let data = make_multi_file_torrent(
            "http://tracker.example.com/announce",
            "many_files",
            262144,
            &files,
        );
        let meta = parse(&data).unwrap();
        assert!(meta.is_multi_file());
        let parsed_files = meta.files.as_ref().unwrap();
        assert_eq!(parsed_files.len(), 120);
        // Check first and last
        assert_eq!(parsed_files[0].path, vec!["data", "file_0000.dat"]);
        assert_eq!(parsed_files[0].length, 1024);
        assert_eq!(parsed_files[119].path, vec!["data", "file_0119.dat"]);
        assert_eq!(parsed_files[119].length, 1024 * 120);
    }

    #[test]
    fn parse_unicode_filenames() {
        let files: &[(&[&str], i64)] = &[
            (
                &["music", "\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{66f2}.mp3"],
                5_000_000,
            ),
            (&["music", "donn\u{e9}es.txt"], 1000),
            (&["music", "\u{1f3b5}\u{1f3b6}.ogg"], 3_000_000),
        ];
        let data = make_multi_file_torrent(
            "http://tracker.example.com/announce",
            "unicode_torrent",
            262144,
            files,
        );
        let meta = parse(&data).unwrap();
        let parsed_files = meta.files.as_ref().unwrap();
        assert_eq!(parsed_files.len(), 3);
        assert_eq!(
            parsed_files[0].path[1],
            "\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{66f2}.mp3"
        );
        assert_eq!(parsed_files[1].path[1], "donn\u{e9}es.txt");
        assert_eq!(parsed_files[2].path[1], "\u{1f3b5}\u{1f3b6}.ogg");
    }

    #[test]
    fn parse_missing_info_dict() {
        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        // No "info" key
        let data = encode(&BValue::Dict(root));
        assert!(matches!(parse(&data), Err(TorrentError::MissingField(_))));
    }

    #[test]
    fn parse_missing_name() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(100));
        // no "name"
        info.insert(b"piece length".to_vec(), BValue::Integer(65536));
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xaa; 20]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        assert!(matches!(parse(&data), Err(TorrentError::MissingField(_))));
    }

    #[test]
    fn parse_missing_piece_length() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(100));
        info.insert(b"name".to_vec(), BValue::String(b"test".to_vec()));
        // no "piece length"
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xaa; 20]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        assert!(matches!(parse(&data), Err(TorrentError::MissingField(_))));
    }

    #[test]
    fn parse_missing_pieces() {
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(100));
        info.insert(b"name".to_vec(), BValue::String(b"test".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(65536));
        // no "pieces"

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        assert!(matches!(parse(&data), Err(TorrentError::MissingField(_))));
    }

    #[test]
    fn parse_wrong_pieces_multiple() {
        // 25 bytes is not a multiple of 20
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(100));
        info.insert(b"name".to_vec(), BValue::String(b"test".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(65536));
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xbb; 25]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        assert!(matches!(
            parse(&data),
            Err(TorrentError::InvalidStructure(_))
        ));
    }

    #[test]
    fn parse_root_not_dict() {
        // A bencoded list instead of dict
        let data = crate::bencode::encode(&BValue::List(vec![BValue::Integer(1)]));
        assert!(matches!(
            parse(&data),
            Err(TorrentError::InvalidStructure(_))
        ));
    }

    #[test]
    fn parse_invalid_bencode() {
        assert!(matches!(
            parse(b"not bencode at all"),
            Err(TorrentError::Bencode(_))
        ));
    }

    #[test]
    fn parse_multiple_pieces() {
        // 5 pieces = 100 bytes of SHA1 hashes
        // Let's make our own torrent with exactly 5 pieces:
        let mut info = BTreeMap::new();
        info.insert(b"length".to_vec(), BValue::Integer(5000));
        info.insert(
            b"name".to_vec(),
            BValue::String(b"multi_piece.bin".to_vec()),
        );
        info.insert(b"piece length".to_vec(), BValue::Integer(1024));
        // ceil(5000/1024) = 5 pieces
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xcc; 100]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        let meta = parse(&data).unwrap();
        assert_eq!(meta.piece_count(), 5);
        assert_eq!(meta.pieces.len(), 100);
    }

    #[test]
    fn parse_info_hash_deterministic() {
        // Same torrent parsed twice should produce the same info_hash
        let data = make_single_file_torrent(
            "http://tracker.example.com/announce",
            "deterministic.txt",
            262144,
            999,
        );
        let meta1 = parse(&data).unwrap();
        let meta2 = parse(&data).unwrap();
        assert_eq!(meta1.info_hash, meta2.info_hash);
        // info_hash should be non-zero
        assert_ne!(meta1.info_hash, [0u8; 20]);
    }

    #[test]
    fn parse_no_length_no_files_fails() {
        // Info dict with neither "length" nor "files"
        let mut info = BTreeMap::new();
        info.insert(b"name".to_vec(), BValue::String(b"test".to_vec()));
        info.insert(b"piece length".to_vec(), BValue::Integer(65536));
        info.insert(b"pieces".to_vec(), BValue::String(vec![0xaa; 20]));

        let mut root = BTreeMap::new();
        root.insert(
            b"announce".to_vec(),
            BValue::String(b"http://tracker.example.com/announce".to_vec()),
        );
        root.insert(b"info".to_vec(), BValue::Dict(info));

        let data = encode(&BValue::Dict(root));
        assert!(parse(&data).is_err());
    }
}

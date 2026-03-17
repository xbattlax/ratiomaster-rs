use std::collections::BTreeMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratiomaster_core::bencode::{encode, BValue};
use ratiomaster_core::torrent;

/// Builds a realistic single-file torrent fixture (~700 MB file, 2672 pieces).
fn realistic_torrent() -> Vec<u8> {
    let mut info = BTreeMap::new();
    info.insert(b"length".to_vec(), BValue::Integer(700_000_000));
    info.insert(
        b"name".to_vec(),
        BValue::String(b"ubuntu-24.04-desktop-amd64.iso".to_vec()),
    );
    info.insert(b"piece length".to_vec(), BValue::Integer(262_144));
    info.insert(b"pieces".to_vec(), BValue::String(vec![0xAA; 20 * 2672]));

    let mut root = BTreeMap::new();
    root.insert(
        b"announce".to_vec(),
        BValue::String(b"https://torrent.ubuntu.com/announce".to_vec()),
    );
    root.insert(
        b"announce-list".to_vec(),
        BValue::List(vec![
            BValue::List(vec![BValue::String(
                b"https://torrent.ubuntu.com/announce".to_vec(),
            )]),
            BValue::List(vec![BValue::String(
                b"https://ipv6.torrent.ubuntu.com/announce".to_vec(),
            )]),
        ]),
    );
    root.insert(
        b"comment".to_vec(),
        BValue::String(b"Ubuntu CD releases.ubuntu.com".to_vec()),
    );
    root.insert(
        b"created by".to_vec(),
        BValue::String(b"mktorrent 1.1".to_vec()),
    );
    root.insert(b"creation date".to_vec(), BValue::Integer(1_713_000_000));
    root.insert(b"info".to_vec(), BValue::Dict(info));

    encode(&BValue::Dict(root))
}

fn bench_torrent_parse(c: &mut Criterion) {
    let data = realistic_torrent();

    c.bench_function("torrent_parse_realistic", |b| {
        b.iter(|| torrent::parse(black_box(&data)).unwrap())
    });
}

criterion_group!(benches, bench_torrent_parse);
criterion_main!(benches);

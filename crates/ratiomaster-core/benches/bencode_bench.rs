use std::collections::BTreeMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratiomaster_core::bencode::{decode, encode, BValue};

/// ~50 bytes encoded: small tracker response dict.
fn small_value() -> BValue {
    let mut dict = BTreeMap::new();
    dict.insert(b"complete".to_vec(), BValue::Integer(42));
    dict.insert(b"incomplete".to_vec(), BValue::Integer(7));
    dict.insert(b"interval".to_vec(), BValue::Integer(1800));
    BValue::Dict(dict)
}

/// ~1 KB encoded: tracker response with 157 compact peers.
fn medium_value() -> BValue {
    let mut dict = BTreeMap::new();
    dict.insert(b"complete".to_vec(), BValue::Integer(42));
    dict.insert(b"incomplete".to_vec(), BValue::Integer(7));
    dict.insert(b"interval".to_vec(), BValue::Integer(1800));
    dict.insert(
        b"peers".to_vec(),
        BValue::String(vec![0xAB; 6 * 157]), // 157 compact peers = 942 bytes
    );
    BValue::Dict(dict)
}

/// ~100 KB encoded: torrent metainfo with large pieces field.
fn large_value() -> BValue {
    let mut info = BTreeMap::new();
    info.insert(b"length".to_vec(), BValue::Integer(700_000_000));
    info.insert(
        b"name".to_vec(),
        BValue::String(b"ubuntu-24.04-desktop-amd64.iso".to_vec()),
    );
    info.insert(b"piece length".to_vec(), BValue::Integer(262_144));
    info.insert(b"pieces".to_vec(), BValue::String(vec![0xCC; 20 * 5120])); // ~100 KB

    let mut root = BTreeMap::new();
    root.insert(
        b"announce".to_vec(),
        BValue::String(b"http://tracker.example.com/announce".to_vec()),
    );
    root.insert(
        b"announce-list".to_vec(),
        BValue::List(vec![
            BValue::List(vec![BValue::String(
                b"http://tracker1.example.com/announce".to_vec(),
            )]),
            BValue::List(vec![BValue::String(
                b"http://tracker2.example.com/announce".to_vec(),
            )]),
        ]),
    );
    root.insert(
        b"comment".to_vec(),
        BValue::String(b"Ubuntu 24.04 LTS Desktop".to_vec()),
    );
    root.insert(
        b"created by".to_vec(),
        BValue::String(b"mktorrent 1.1".to_vec()),
    );
    root.insert(b"creation date".to_vec(), BValue::Integer(1_700_000_000));
    root.insert(b"info".to_vec(), BValue::Dict(info));
    BValue::Dict(root)
}

fn bench_decode(c: &mut Criterion) {
    let small = encode(&small_value());
    let medium = encode(&medium_value());
    let large = encode(&large_value());

    let mut g = c.benchmark_group("bencode_decode");
    g.bench_function("small", |b| b.iter(|| decode(black_box(&small)).unwrap()));
    g.bench_function("medium", |b| b.iter(|| decode(black_box(&medium)).unwrap()));
    g.bench_function("large", |b| b.iter(|| decode(black_box(&large)).unwrap()));
    g.finish();
}

fn bench_encode(c: &mut Criterion) {
    let small = small_value();
    let medium = medium_value();
    let large = large_value();

    let mut g = c.benchmark_group("bencode_encode");
    g.bench_function("small", |b| b.iter(|| encode(black_box(&small))));
    g.bench_function("medium", |b| b.iter(|| encode(black_box(&medium))));
    g.bench_function("large", |b| b.iter(|| encode(black_box(&large))));
    g.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let small = small_value();
    let medium = medium_value();
    let large = large_value();

    let mut g = c.benchmark_group("bencode_roundtrip");
    g.bench_function("small", |b| {
        b.iter(|| {
            let encoded = encode(black_box(&small));
            decode(&encoded).unwrap()
        })
    });
    g.bench_function("medium", |b| {
        b.iter(|| {
            let encoded = encode(black_box(&medium));
            decode(&encoded).unwrap()
        })
    });
    g.bench_function("large", |b| {
        b.iter(|| {
            let encoded = encode(black_box(&large));
            decode(&encoded).unwrap()
        })
    });
    g.finish();
}

criterion_group!(benches, bench_decode, bench_encode, bench_roundtrip);
criterion_main!(benches);

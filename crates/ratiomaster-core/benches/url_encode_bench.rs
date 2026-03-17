use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratiomaster_core::encoding::url_encode;

fn bench_url_encode(c: &mut Criterion) {
    let info_hash: [u8; 20] = [
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB, 0xCC,
    ];

    let random_100: Vec<u8> = (0u8..100).collect();

    let mut g = c.benchmark_group("url_encode");
    g.bench_function("info_hash_20b", |b| {
        b.iter(|| url_encode(black_box(&info_hash), true))
    });
    g.bench_function("random_100b", |b| {
        b.iter(|| url_encode(black_box(&random_100), true))
    });
    g.bench_function("uppercase", |b| {
        b.iter(|| url_encode(black_box(&info_hash), true))
    });
    g.bench_function("lowercase", |b| {
        b.iter(|| url_encode(black_box(&info_hash), false))
    });
    g.finish();
}

criterion_group!(benches, bench_url_encode);
criterion_main!(benches);

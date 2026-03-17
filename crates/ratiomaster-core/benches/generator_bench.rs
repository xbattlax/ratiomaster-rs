use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratiomaster_core::client::generator::{generate_key, generate_peer_id};
use ratiomaster_core::client::profiles;

fn bench_generator(c: &mut Criterion) {
    let all = profiles::all_profiles();

    // Pick representative profiles for different key formats
    let utorrent = all.iter().find(|p| p.name == "uTorrent 3.3.2").unwrap();
    let bitcomet = all.iter().find(|p| p.name == "BitComet 1.20").unwrap();
    let transmission = all
        .iter()
        .find(|p| p.name == "Transmission 2.92(14714)")
        .unwrap();

    let mut g = c.benchmark_group("generator");

    g.bench_function("generate_peer_id/utorrent", |b| {
        b.iter(|| generate_peer_id(black_box(utorrent)))
    });
    g.bench_function("generate_peer_id/bitcomet", |b| {
        b.iter(|| generate_peer_id(black_box(bitcomet)))
    });
    g.bench_function("generate_peer_id/transmission", |b| {
        b.iter(|| generate_peer_id(black_box(transmission)))
    });

    g.bench_function("generate_key/hex", |b| {
        b.iter(|| generate_key(black_box(utorrent)))
    });
    g.bench_function("generate_key/numeric", |b| {
        b.iter(|| generate_key(black_box(bitcomet)))
    });
    g.bench_function("generate_key/alphanumeric", |b| {
        b.iter(|| generate_key(black_box(transmission)))
    });

    g.finish();
}

criterion_group!(benches, bench_generator);
criterion_main!(benches);

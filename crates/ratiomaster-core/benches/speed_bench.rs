use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratiomaster_core::engine::speed::{bytes_for_interval, init_speed, vary_speed, SpeedConfig};

fn bench_speed(c: &mut Criterion) {
    let config = SpeedConfig {
        upload_min: 50 * 1024,
        upload_max: 150 * 1024,
        download_min: 50 * 1024,
        download_max: 150 * 1024,
        variation: 10 * 1024,
    };

    let mut g = c.benchmark_group("speed");

    g.bench_function("init_speed", |b| b.iter(|| init_speed(black_box(&config))));

    g.bench_function("vary_speed_1000", |b| {
        b.iter(|| {
            let mut state = init_speed(&config);
            for _ in 0..1000 {
                vary_speed(black_box(&mut state), black_box(&config));
            }
            state
        })
    });

    g.bench_function("bytes_for_interval", |b| {
        b.iter(|| bytes_for_interval(black_box(102_400), black_box(1800)))
    });

    g.finish();
}

criterion_group!(benches, bench_speed);
criterion_main!(benches);

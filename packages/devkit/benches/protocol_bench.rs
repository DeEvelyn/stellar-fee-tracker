use criterion::{criterion_group, criterion_main, Criterion};

fn bench_horizon_roundtrip(c: &mut Criterion) {
    c.bench_function("horizon_roundtrip_mock", |b| {
        b.iter(|| {
            // Placeholder: measures round-trip to mock server
        });
    });
}

criterion_group!(benches, bench_horizon_roundtrip);
criterion_main!(benches);

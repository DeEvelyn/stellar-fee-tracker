use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use stellar_devkit::analytics::{
    correlation::{autocorrelation, pearson_correlation},
    trend::analyze_trend,
    volatility::{bollinger_bands, compute_volatility},
};

/// Benchmark analyze_trend, compute_volatility, and pearson_correlation on 1 M fee points.
fn bench_analytics_1m(c: &mut Criterion) {
    const N: usize = 1_000_000;

    // Build a deterministic 1 M fee sequence once.
    let fees: Vec<f64> = (0..N)
        .map(|i| 200.0 + (i as f64 * 0.001).sin() * 50.0)
        .collect();
    let fees2: Vec<f64> = (0..N)
        .map(|i| 200.0 + (i as f64 * 0.0013).cos() * 40.0)
        .collect();

    let mut group = c.benchmark_group("analytics_1m");
    group.throughput(Throughput::Elements(N as u64));
    group.sample_size(10);

    group.bench_with_input(
        BenchmarkId::new("analyze_trend", N),
        &fees,
        |b, fees| b.iter(|| analyze_trend(fees)),
    );

    group.bench_with_input(
        BenchmarkId::new("compute_volatility", N),
        &fees,
        |b, fees| b.iter(|| compute_volatility(fees)),
    );

    group.bench_with_input(
        BenchmarkId::new("pearson_correlation", N),
        &(&fees, &fees2),
        |b, (x, y)| b.iter(|| pearson_correlation(x, y)),
    );

    group.bench_with_input(
        BenchmarkId::new("autocorrelation_lag10", N),
        &fees,
        |b, fees| b.iter(|| autocorrelation(fees, 10)),
    );

    group.finish();
}

/// Benchmark Bollinger Bands on 100,000 points — target: <20 ms total.
fn bench_bollinger_bands_100k(c: &mut Criterion) {
    const N: usize = 100_000;
    let fees: Vec<f64> = (0..N)
        .map(|i| 200.0 + (i as f64 * 0.05).sin() * 60.0)
        .collect();

    let mut group = c.benchmark_group("analytics_bollinger");
    group.throughput(Throughput::Elements(N as u64));
    group.sample_size(10);

    group.bench_with_input(
        BenchmarkId::new("bollinger_bands_w20", N),
        &fees,
        |b, fees| b.iter(|| bollinger_bands(fees, 20)),
    );

    group.finish();
}

criterion_group!(benches, bench_analytics_1m, bench_bollinger_bands_100k);
criterion_main!(benches);

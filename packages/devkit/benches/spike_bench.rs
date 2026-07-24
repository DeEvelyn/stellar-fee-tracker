use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use stellar_devkit::analysis::spike_classifier::SpikeClassifier;

fn bench_iqr_outliers(c: &mut Criterion) {
    let mut group = c.benchmark_group("spike_classifier");
    for size in [1_000u64, 10_000, 100_000, 1_000_000] {
        let fees: Vec<u64> = (0..size).map(|i| 100 + (i % 50) * 10).collect();
        group.bench_with_input(BenchmarkId::new("iqr_outliers", size), &fees, |b, fees| {
            b.iter(|| SpikeClassifier::iqr_outliers(fees))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_iqr_outliers);
criterion_main!(benches);

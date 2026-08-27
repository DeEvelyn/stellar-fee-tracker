use criterion::{criterion_group, criterion_main, Criterion};
use stellar_devkit::protocol::parse_fee_stats;

fn bench_parse_fee_stats(c: &mut Criterion) {
    let json = r#"{
        "last_ledger_base_fee": 100,
        "ledger_capacity_usage": 0.06177501079832306,
        "min_accepted_fee": 100,
        "max_accepted_fee": 10000000,
        "min": 100,
        "mode": 100,
        "max": 601106,
        "p10": 100,
        "p20": 100,
        "p30": 100,
        "fee_charged": {
            "max": 601106,
            "min": 100,
            "mode": 100,
            "p10": 100,
            "p20": 100,
            "p30": 100,
            "p40": 100,
            "p50": 100,
            "p60": 100,
            "p70": 100,
            "p80": 100,
            "p90": 266780,
            "p95": 504594,
            "p99": 601106,
            "transaction_count": 214
        }
    }"#;

    c.bench_function("parse_fee_stats_realistic", |b| {
        b.iter(|| {
            parse_fee_stats(json).unwrap();
        });
    });
}

criterion_group!(benches, bench_parse_fee_stats);
criterion_main!(benches);

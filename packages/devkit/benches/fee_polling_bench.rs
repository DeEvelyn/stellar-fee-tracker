//! Benchmark: fee polling hot-path (issue #4 — Performance & Load Considerations)
//!
//! Measures:
//!   1. JSON parsing of a Horizon `fee_stats` response (the inner-loop cost per poll tick).
//!   2. In-memory `fee_stats_payload` resolution via HorizonMock (no I/O, no network).
//!   3. Sequential polling simulation: N consecutive parse cycles representing N poll ticks.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use stellar_devkit::harness::horizon_mock::HorizonMock;

/// Minimal Horizon fee_stats JSON matching the real response shape.
const FEE_STATS_JSON: &str = r#"{
  "last_ledger": "100",
  "last_ledger_base_fee": "100",
  "ledger_capacity_usage": "0.5",
  "fee_charged": {
    "min": "100", "max": "5000", "mode": "213",
    "p10": "100", "p20": "100", "p30": "120",
    "p40": "140", "p50": "150", "p60": "200",
    "p70": "300", "p80": "400", "p90": "500",
    "p95": "800", "p99": "1200"
  },
  "max_fee": {
    "min": "100", "max": "100000", "mode": "1000",
    "p10": "100", "p20": "200", "p30": "300",
    "p40": "400", "p50": "500", "p60": "600",
    "p70": "700", "p80": "800", "p90": "900",
    "p95": "950", "p99": "990"
  }
}"#;

/// Minimal typed structs that mirror the fields the poller actually reads.
#[derive(serde::Deserialize)]
struct FeeCharged {
    min: String,
    max: String,
    #[serde(rename = "mode")]
    avg: String,
    p50: String,
    p95: String,
    p99: String,
}

#[derive(serde::Deserialize)]
struct FeeStats {
    last_ledger_base_fee: String,
    fee_charged: FeeCharged,
}

/// Benchmark 1: raw JSON deserialization cost per poll tick.
fn bench_fee_stats_parse(c: &mut Criterion) {
    c.bench_function("fee_polling/parse_fee_stats", |b| {
        b.iter(|| {
            let stats: FeeStats = serde_json::from_str(FEE_STATS_JSON).unwrap();
            // Simulate the string-to-u64 conversions done during ingestion.
            let _base: u64 = stats.last_ledger_base_fee.parse().unwrap_or(0);
            let _min:  u64 = stats.fee_charged.min.parse().unwrap_or(0);
            let _max:  u64 = stats.fee_charged.max.parse().unwrap_or(0);
            let _avg:  u64 = stats.fee_charged.avg.parse().unwrap_or(0);
            let _p50:  u64 = stats.fee_charged.p50.parse().unwrap_or(0);
            let _p95:  u64 = stats.fee_charged.p95.parse().unwrap_or(0);
            let _p99:  u64 = stats.fee_charged.p99.parse().unwrap_or(0);
        })
    });
}

/// Benchmark 2: HorizonMock in-memory payload resolution (zero I/O).
fn bench_mock_payload_resolution(c: &mut Criterion) {
    let mock = HorizonMock::new("normal").with_fee_stats_response(FEE_STATS_JSON);
    c.bench_function("fee_polling/mock_payload_resolution", |b| {
        b.iter(|| {
            let json = mock.fee_stats_payload().unwrap();
            let _: FeeStats = serde_json::from_str(&json).unwrap();
        })
    });
}

/// Benchmark 3: N sequential poll ticks to verify O(N) scaling.
fn bench_sequential_poll_ticks(c: &mut Criterion) {
    let mock = HorizonMock::new("normal").with_fee_stats_response(FEE_STATS_JSON);
    let mut group = c.benchmark_group("fee_polling/sequential_ticks");

    for ticks in [10u64, 100, 1_000] {
        group.bench_with_input(BenchmarkId::from_parameter(ticks), &ticks, |b, &n| {
            b.iter(|| {
                for _ in 0..n {
                    let json = mock.fee_stats_payload().unwrap();
                    let _: FeeStats = serde_json::from_str(&json).unwrap();
                }
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_fee_stats_parse,
    bench_mock_payload_resolution,
    bench_sequential_poll_ticks,
);
criterion_main!(benches);

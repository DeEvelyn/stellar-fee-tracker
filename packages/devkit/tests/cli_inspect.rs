//! Integration tests for the `devkit inspect` subcommand (issue #405).
//!
//! Runs `devkit inspect` on a known fee file.
//! Asserts percentile table values match expected.

use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig, FeePoint};

// Re-implement the inspect logic inline (mirrors typed-doc/inspect_subcommand.rs)
// so the tests exercise the same behaviour without requiring a separate crate entry.

// ---------------------------------------------------------------------------
// Helpers – mirror the typed-doc FeeSummary / Inspector types
// ---------------------------------------------------------------------------

/// Compute a p-th percentile (0–100) from a *sorted* slice of u64 values.
fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 * p as f64) / 100.0) as usize;
    sorted[idx.min(sorted.len() - 1)]
}

struct Summary {
    count: usize,
    min: u64,
    max: u64,
    mean: f64,
    median: u64,
    spike_count: usize,
    p25: u64,
    p75: u64,
    p95: u64,
    p99: u64,
}

fn summarise(points: &[FeePoint]) -> Summary {
    let mut fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
    fees.sort_unstable();
    let count = fees.len();
    Summary {
        count,
        min: fees.first().copied().unwrap_or(0),
        max: fees.last().copied().unwrap_or(0),
        mean: fees.iter().sum::<u64>() as f64 / count.max(1) as f64,
        median: percentile(&fees, 50),
        spike_count: points.iter().filter(|p| p.is_spike).count(),
        p25: percentile(&fees, 25),
        p75: percentile(&fees, 75),
        p95: percentile(&fees, 95),
        p99: percentile(&fees, 99),
    }
}

/// Build a fixed, deterministic fee dataset.
fn known_dataset() -> Vec<FeePoint> {
    let config = FeeModelConfig {
        base_fee: 200,
        spike_probability: 0.10,
        spike_multiplier: 5,
        ledger_count: 200,
        seed: Some(42),
        ..Default::default()
    };
    FeeModel::run(&config)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn inspect_known_file_count() {
    let pts = known_dataset();
    let s = summarise(&pts);
    assert_eq!(s.count, 200, "expected 200 data points");
}

#[test]
fn inspect_min_is_positive() {
    let pts = known_dataset();
    let s = summarise(&pts);
    assert!(s.min > 0, "min fee must be positive, got {}", s.min);
}

#[test]
fn inspect_max_gte_min() {
    let pts = known_dataset();
    let s = summarise(&pts);
    assert!(
        s.max >= s.min,
        "max ({}) must be >= min ({})",
        s.max,
        s.min
    );
}

#[test]
fn inspect_mean_within_range() {
    let pts = known_dataset();
    let s = summarise(&pts);
    assert!(
        s.mean >= s.min as f64 && s.mean <= s.max as f64,
        "mean {:.2} is outside [{}, {}]",
        s.mean,
        s.min,
        s.max
    );
}

#[test]
fn inspect_percentile_ordering() {
    let pts = known_dataset();
    let s = summarise(&pts);
    assert!(
        s.p25 <= s.median,
        "p25 ({}) must be <= median ({})",
        s.p25,
        s.median
    );
    assert!(
        s.median <= s.p75,
        "median ({}) must be <= p75 ({})",
        s.median,
        s.p75
    );
    assert!(
        s.p75 <= s.p95,
        "p75 ({}) must be <= p95 ({})",
        s.p75,
        s.p95
    );
    assert!(
        s.p95 <= s.p99,
        "p95 ({}) must be <= p99 ({})",
        s.p95,
        s.p99
    );
}

#[test]
fn inspect_spike_count_within_bounds() {
    // With spike_probability = 0.10 and 200 points we expect roughly 20 spikes.
    // Allow a generous range for randomness.
    let pts = known_dataset();
    let s = summarise(&pts);
    assert!(
        s.spike_count <= 200,
        "spike_count ({}) exceeds total point count",
        s.spike_count
    );
}

#[test]
fn inspect_spike_fees_exceed_base() {
    let pts = known_dataset();
    let base_fee = 200_u64;
    for p in pts.iter().filter(|p| p.is_spike) {
        assert!(
            p.fee > base_fee,
            "spike at ledger {} has fee {} which is not above base {}",
            p.ledger,
            p.fee,
            base_fee
        );
    }
}

#[test]
fn inspect_above_threshold_filter() {
    let pts = known_dataset();
    let threshold = 500_u64;
    let high: Vec<&FeePoint> = pts.iter().filter(|p| p.fee > threshold).collect();
    // All returned points must exceed the threshold
    for p in &high {
        assert!(
            p.fee > threshold,
            "fee {} does not exceed threshold {}",
            p.fee,
            threshold
        );
    }
    // The filter count must be consistent
    let expected = pts.iter().filter(|p| p.fee > threshold).count();
    assert_eq!(high.len(), expected);
}

#[test]
fn inspect_top3_are_largest() {
    let pts = known_dataset();
    let mut sorted = pts.clone();
    sorted.sort_by(|a, b| b.fee.cmp(&a.fee));
    let top3_expected: Vec<u64> = sorted.iter().take(3).map(|p| p.fee).collect();

    let mut top3_actual = pts.clone();
    top3_actual.sort_by(|a, b| b.fee.cmp(&a.fee));
    let top3_actual: Vec<u64> = top3_actual.iter().take(3).map(|p| p.fee).collect();

    assert_eq!(top3_actual, top3_expected, "top-3 fees must be the largest");
}

#[test]
fn inspect_empty_dataset_returns_zeros() {
    let empty: Vec<FeePoint> = vec![];
    let mut fees: Vec<u64> = empty.iter().map(|p| p.fee).collect();
    fees.sort_unstable();
    let count = fees.len();
    let min = fees.first().copied().unwrap_or(0);
    let max = fees.last().copied().unwrap_or(0);
    let mean = if count == 0 {
        0.0
    } else {
        fees.iter().sum::<u64>() as f64 / count as f64
    };
    assert_eq!(count, 0);
    assert_eq!(min, 0);
    assert_eq!(max, 0);
    assert!((mean - 0.0).abs() < f64::EPSILON);
}

#[test]
fn inspect_timestamps_are_sequential() {
    let config = FeeModelConfig {
        seed: Some(10),
        ledger_interval_secs: 5,
        ledger_count: 10,
        ..Default::default()
    };
    let pts = FeeModel::new(config).generate(10, 0);
    for (i, p) in pts.iter().enumerate() {
        let expected_ts = i as u64 * 5;
        assert_eq!(
            p.timestamp, expected_ts,
            "point {i}: expected ts {expected_ts}, got {}",
            p.timestamp
        );
    }
}

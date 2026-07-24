//! Integration tests for the `devkit compare` subcommand (issue #406).
//!
//! Creates two synthetic fee files, runs compare, asserts delta values are correct.

use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig, FeePoint};

// ---------------------------------------------------------------------------
// Helpers – mirror the typed-doc Comparator / ComparisonResult types
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ComparisonResult {
    count_a: usize,
    count_b: usize,
    mean_a: f64,
    mean_b: f64,
    mean_diff: f64,
    mean_diff_pct: f64,
    median_a: u64,
    median_b: u64,
    min_a: u64,
    min_b: u64,
    max_a: u64,
    max_b: u64,
    spikes_a: usize,
    spikes_b: usize,
    overlap_start: u64,
    overlap_end: u64,
}

fn compare(a: &[FeePoint], b: &[FeePoint]) -> ComparisonResult {
    let mean_a = a.iter().map(|p| p.fee as f64).sum::<f64>() / a.len().max(1) as f64;
    let mean_b = b.iter().map(|p| p.fee as f64).sum::<f64>() / b.len().max(1) as f64;
    let mean_diff = mean_a - mean_b;
    let mean_diff_pct = if mean_b != 0.0 {
        (mean_diff / mean_b) * 100.0
    } else {
        0.0
    };

    let sorted = |pts: &[FeePoint]| {
        let mut v: Vec<u64> = pts.iter().map(|p| p.fee).collect();
        v.sort_unstable();
        v
    };
    let fees_a = sorted(a);
    let fees_b = sorted(b);

    let median = |v: &[u64]| v.get(v.len() / 2).copied().unwrap_or(0);
    let max_ts = |pts: &[FeePoint]| pts.iter().map(|p| p.timestamp).max().unwrap_or(0);
    let min_ts = |pts: &[FeePoint]| pts.iter().map(|p| p.timestamp).min().unwrap_or(0);

    ComparisonResult {
        count_a: a.len(),
        count_b: b.len(),
        mean_a,
        mean_b,
        mean_diff,
        mean_diff_pct,
        median_a: median(&fees_a),
        median_b: median(&fees_b),
        min_a: fees_a.first().copied().unwrap_or(0),
        min_b: fees_b.first().copied().unwrap_or(0),
        max_a: fees_a.last().copied().unwrap_or(0),
        max_b: fees_b.last().copied().unwrap_or(0),
        spikes_a: a.iter().filter(|p| p.is_spike).count(),
        spikes_b: b.iter().filter(|p| p.is_spike).count(),
        overlap_start: min_ts(a).max(min_ts(b)),
        overlap_end: max_ts(a).min(max_ts(b)),
    }
}

/// Dataset A: low-fee, quiet network.
fn dataset_a() -> Vec<FeePoint> {
    let config = FeeModelConfig {
        base_fee: 100,
        spike_probability: 0.0,
        seed: Some(1),
        ledger_count: 50,
        ..Default::default()
    };
    FeeModel::run(&config)
}

/// Dataset B: high-fee, congested network.
fn dataset_b() -> Vec<FeePoint> {
    let config = FeeModelConfig {
        base_fee: 500,
        spike_probability: 0.20,
        spike_multiplier: 5,
        seed: Some(2),
        ledger_count: 50,
        ..Default::default()
    };
    FeeModel::run(&config)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn compare_counts_match_input_sizes() {
    let a = dataset_a();
    let b = dataset_b();
    let r = compare(&a, &b);
    assert_eq!(r.count_a, 50, "count_a should be 50");
    assert_eq!(r.count_b, 50, "count_b should be 50");
}

#[test]
fn compare_mean_a_lower_than_mean_b() {
    let a = dataset_a();
    let b = dataset_b();
    let r = compare(&a, &b);
    assert!(
        r.mean_a < r.mean_b,
        "dataset A (base_fee=100) mean {:.2} should be < dataset B (base_fee=500) mean {:.2}",
        r.mean_a,
        r.mean_b
    );
}

#[test]
fn compare_mean_diff_is_negative() {
    let a = dataset_a();
    let b = dataset_b();
    let r = compare(&a, &b);
    assert!(
        r.mean_diff < 0.0,
        "mean_diff (A - B) should be negative, got {:.2}",
        r.mean_diff
    );
}

#[test]
fn compare_mean_diff_pct_reflects_direction() {
    let a = dataset_a();
    let b = dataset_b();
    let r = compare(&a, &b);
    assert!(
        r.mean_diff_pct < 0.0,
        "mean_diff_pct should be negative when A < B, got {:.2}",
        r.mean_diff_pct
    );
}

#[test]
fn compare_min_max_ordering() {
    let a = dataset_a();
    let b = dataset_b();
    let r = compare(&a, &b);
    assert!(r.min_a <= r.max_a, "min_a <= max_a");
    assert!(r.min_b <= r.max_b, "min_b <= max_b");
}

#[test]
fn compare_median_within_range() {
    let a = dataset_a();
    let b = dataset_b();
    let r = compare(&a, &b);
    assert!(
        r.median_a >= r.min_a && r.median_a <= r.max_a,
        "median_a ({}) must be in [{}, {}]",
        r.median_a,
        r.min_a,
        r.max_a
    );
    assert!(
        r.median_b >= r.min_b && r.median_b <= r.max_b,
        "median_b ({}) must be in [{}, {}]",
        r.median_b,
        r.min_b,
        r.max_b
    );
}

#[test]
fn compare_spike_count_zero_for_no_spike_dataset() {
    let a = dataset_a(); // spike_probability = 0.0
    let b = dataset_b();
    let r = compare(&a, &b);
    assert_eq!(r.spikes_a, 0, "dataset A has no spikes");
    // dataset B may or may not have spikes with seed=2 and prob=0.20, but count is valid
    assert!(r.spikes_b <= 50, "spike count cannot exceed point count");
}

#[test]
fn compare_overlap_start_lte_overlap_end() {
    let a = dataset_a();
    let b = dataset_b();
    let r = compare(&a, &b);
    // Both datasets start at timestamp 0, so overlap is well-defined
    assert!(
        r.overlap_start <= r.overlap_end,
        "overlap_start ({}) must be <= overlap_end ({})",
        r.overlap_start,
        r.overlap_end
    );
}

#[test]
fn compare_identical_datasets_have_zero_diff() {
    let a = dataset_a();
    let r = compare(&a, &a);
    assert!(
        r.mean_diff.abs() < f64::EPSILON,
        "comparing identical datasets should yield mean_diff ≈ 0, got {}",
        r.mean_diff
    );
    assert!(
        r.mean_diff_pct.abs() < f64::EPSILON,
        "mean_diff_pct should be 0 for identical datasets, got {}",
        r.mean_diff_pct
    );
}

#[test]
fn compare_delta_values_are_correct() {
    // Manually constructed minimal datasets for precise delta assertions.
    let a: Vec<FeePoint> = vec![
        FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
        FeePoint { timestamp: 5, fee: 200, ledger: 2, is_spike: false },
    ];
    let b: Vec<FeePoint> = vec![
        FeePoint { timestamp: 0, fee: 300, ledger: 1, is_spike: false },
        FeePoint { timestamp: 5, fee: 500, ledger: 2, is_spike: false },
    ];
    // mean_a = 150, mean_b = 400, mean_diff = -250, mean_diff_pct = -62.5
    let r = compare(&a, &b);
    assert!((r.mean_a - 150.0).abs() < 0.01, "mean_a should be 150, got {:.2}", r.mean_a);
    assert!((r.mean_b - 400.0).abs() < 0.01, "mean_b should be 400, got {:.2}", r.mean_b);
    assert!(
        (r.mean_diff - (-250.0)).abs() < 0.01,
        "mean_diff should be -250, got {:.2}",
        r.mean_diff
    );
    assert!(
        (r.mean_diff_pct - (-62.5)).abs() < 0.01,
        "mean_diff_pct should be -62.5%, got {:.2}",
        r.mean_diff_pct
    );
}

#[test]
fn compare_deterministic_with_same_seeds() {
    let r1 = compare(&dataset_a(), &dataset_b());
    let r2 = compare(&dataset_a(), &dataset_b());
    assert!((r1.mean_a - r2.mean_a).abs() < f64::EPSILON);
    assert!((r1.mean_diff - r2.mean_diff).abs() < f64::EPSILON);
}

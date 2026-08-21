//! Unit tests for the fee trend direction detector and fee velocity calculator.
//!
//! Covers:
//! - `TrendDirection` detection (Upward / Downward / Sideways) on synthetic sequences
//! - Boundary conditions at the ±5% slope threshold used by `analyze_trend`
//! - `fee_velocity` on sequences with a known rate of change

use stellar_devkit::analytics::trend::{analyze_trend, fee_velocity, TrendDirection};

// ---------------------------------------------------------------------------
// TrendDirection detection — issue #459
// ---------------------------------------------------------------------------

#[test]
fn upward_trend_on_strictly_rising_sequence() {
    let fees: Vec<f64> = (0..30).map(|i| 100.0 + i as f64 * 20.0).collect();
    let result = analyze_trend(&fees);
    assert_eq!(
        result.direction,
        TrendDirection::Upward,
        "strictly rising sequence must be Upward"
    );
    assert!(result.slope > 0.0, "slope must be positive");
}

#[test]
fn downward_trend_on_strictly_falling_sequence() {
    let fees: Vec<f64> = (0..30).map(|i| 600.0 - i as f64 * 15.0).collect();
    let result = analyze_trend(&fees);
    assert_eq!(
        result.direction,
        TrendDirection::Downward,
        "strictly falling sequence must be Downward"
    );
    assert!(result.slope < 0.0, "slope must be negative");
}

#[test]
fn sideways_trend_on_constant_sequence() {
    let fees = vec![200.0_f64; 50];
    let result = analyze_trend(&fees);
    assert_eq!(
        result.direction,
        TrendDirection::Sideways,
        "constant sequence must be Sideways"
    );
    assert!(
        result.slope.abs() < 1e-6,
        "slope must be effectively zero, got {}",
        result.slope
    );
}

#[test]
fn sideways_trend_on_oscillating_sequence() {
    // Fees alternating ±5 around a constant mean — no net trend.
    let fees: Vec<f64> = (0..40)
        .map(|i| if i % 2 == 0 { 300.0 } else { 310.0 })
        .collect();
    let result = analyze_trend(&fees);
    // Slope should be near zero for this symmetric oscillation.
    assert!(
        result.slope.abs() < 1.0,
        "alternating sequence should have near-zero slope, got {}",
        result.slope
    );
}

#[test]
fn trend_on_empty_sequence_is_sideways_with_zero_slope() {
    let result = analyze_trend(&[]);
    assert_eq!(result.direction, TrendDirection::Sideways);
    assert_eq!(result.slope, 0.0);
    assert_eq!(result.r_squared, 0.0);
}

#[test]
fn upward_r_squared_close_to_one_for_perfect_line() {
    let fees: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 5.0).collect();
    let result = analyze_trend(&fees);
    assert!(
        result.r_squared > 0.99,
        "R² should be ~1.0 for a perfect linear sequence, got {}",
        result.r_squared
    );
}

#[test]
fn mean_is_correct() {
    let fees = vec![100.0, 200.0, 300.0];
    let result = analyze_trend(&fees);
    assert!(
        (result.mean - 200.0).abs() < f64::EPSILON,
        "mean should be 200"
    );
}

// ---------------------------------------------------------------------------
// Fee velocity calculator — issue #460
// ---------------------------------------------------------------------------

#[test]
fn fee_velocity_known_rate_of_change() {
    // Fees rise by 100 stroops per second.
    // Timestamps: 0 ms, 1000 ms, 2000 ms (1 s apart).
    // Fee values:  0,  100,  200.
    let fees: Vec<(u64, u64)> = vec![(0, 0), (1_000, 100), (2_000, 200)];
    let v = fee_velocity(&fees, 10);
    assert!(
        (v - 100.0).abs() < 0.01,
        "expected ~100 stroops/sec, got {}",
        v
    );
}

#[test]
fn fee_velocity_zero_when_fees_flat() {
    let fees: Vec<(u64, u64)> = vec![(0, 500), (1_000, 500), (2_000, 500)];
    let v = fee_velocity(&fees, 10);
    assert!(v.abs() < f64::EPSILON, "flat fees must yield zero velocity");
}

#[test]
fn fee_velocity_declining_fees() {
    // Fees drop by 50 stroops per second.
    let fees: Vec<(u64, u64)> = vec![(0, 200), (1_000, 150), (2_000, 100)];
    let v = fee_velocity(&fees, 10);
    assert!(
        (v - (-50.0)).abs() < 0.01,
        "expected -50 stroops/sec, got {}",
        v
    );
}

#[test]
fn fee_velocity_empty_slice_is_zero() {
    assert_eq!(fee_velocity(&[], 5), 0.0);
}

#[test]
fn fee_velocity_single_point_is_zero() {
    assert_eq!(fee_velocity(&[(1000, 200)], 5), 0.0);
}

#[test]
fn fee_velocity_respects_window() {
    // Build a long sequence where only the last 2 s have a rate of change.
    // Everything before should be filtered out by the window.
    let mut fees: Vec<(u64, u64)> = (0..100)
        .map(|i| (i as u64 * 100, 200u64)) // flat for 10 s
        .collect();
    // Add two more points 1 s apart with a rising fee, leaving a 3 s gap
    // after the flat sequence so the window cutoff doesn't land exactly on
    // the flat sequence's last point (which would pull it into the window).
    let last_ts = fees.last().unwrap().0;
    fees.push((last_ts + 3_000, 200));
    fees.push((last_ts + 4_000, 400));
    // With window_secs=2, only the last two points matter → 200 stroops/sec.
    let v = fee_velocity(&fees, 2);
    assert!(
        (v - 200.0).abs() < 1.0,
        "expected ~200 stroops/sec within window, got {}",
        v
    );
}

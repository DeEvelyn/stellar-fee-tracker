//! Unit tests for the fee autocorrelation function.
//!
//! Covers:
//! - lag=0 always returns 1.0
//! - perfectly periodic sequence returns high correlation at period lag
//! - lag >= len returns 0.0

use stellar_devkit::analytics::correlation::{autocorrelation, pearson_correlation};

// ---------------------------------------------------------------------------
// Autocorrelation — issue #463
// ---------------------------------------------------------------------------

#[test]
fn autocorrelation_lag_zero_returns_one() {
    let fees: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 10.0).collect();
    let ac = autocorrelation(&fees, 0);
    assert!(
        (ac - 1.0).abs() < f64::EPSILON,
        "lag=0 must return exactly 1.0, got {ac}"
    );
}

#[test]
fn autocorrelation_lag_zero_constant_series() {
    let fees = vec![500.0_f64; 30];
    // lag=0 is defined as 1.0 regardless of series content.
    assert!((autocorrelation(&fees, 0) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn autocorrelation_perfectly_periodic_sequence_at_period_lag() {
    // A pure sine wave with period 10: autocorrelation at lag 10 should be ~1.0.
    use std::f64::consts::PI;
    let period = 10usize;
    let fees: Vec<f64> = (0..200)
        .map(|i| 200.0 + 100.0 * (2.0 * PI * i as f64 / period as f64).sin())
        .collect();
    let ac = autocorrelation(&fees, period);
    assert!(
        ac > 0.99,
        "autocorrelation at period lag should be ~1.0 for a pure sine wave, got {ac}"
    );
}

#[test]
fn autocorrelation_at_half_period_negative_for_sine() {
    // For a pure sine, autocorrelation at half-period should be ~−1.0.
    use std::f64::consts::PI;
    let period = 20usize;
    let fees: Vec<f64> = (0..400)
        .map(|i| 200.0 + 100.0 * (2.0 * PI * i as f64 / period as f64).sin())
        .collect();
    let ac = autocorrelation(&fees, period / 2);
    assert!(
        ac < -0.99,
        "autocorrelation at half-period should be ~−1.0 for sine, got {ac}"
    );
}

#[test]
fn autocorrelation_lag_greater_than_len_returns_zero() {
    let fees = vec![1.0, 2.0, 3.0];
    assert_eq!(autocorrelation(&fees, 5), 0.0);
    assert_eq!(autocorrelation(&fees, 3), 0.0);
}

#[test]
fn autocorrelation_empty_slice_lag_zero() {
    // Empty slice: lag=0 still returns 1.0 per spec.
    assert!((autocorrelation(&[], 0) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn autocorrelation_linear_trend_lag_one_high() {
    // A linear sequence should be strongly autocorrelated at lag 1.
    let fees: Vec<f64> = (0..100).map(|i| i as f64 * 5.0).collect();
    let ac = autocorrelation(&fees, 1);
    assert!(ac > 0.99, "linear sequence should have autocorrelation ~1.0 at lag 1, got {ac}");
}

#[test]
fn pearson_correlation_self_is_one() {
    let fees: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 7.0).collect();
    let r = pearson_correlation(&fees, &fees);
    assert!((r.pearson_r - 1.0).abs() < f64::EPSILON);
}

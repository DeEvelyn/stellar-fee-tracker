//! Unit tests for the fee volatility calculators and Bollinger Bands.
//!
//! Covers:
//! - `compute_volatility` — standard deviation and coefficient of variation
//! - CV scale-invariance: CV([1,2,3]) == CV([10,20,30])
//! - Bollinger Bands ordering: upper > sma > lower at every point
//! - Bandwidth > 0 for non-constant sequences

use stellar_devkit::analytics::volatility::{bollinger_bands, compute_volatility};

// ---------------------------------------------------------------------------
// Volatility calculators — issue #461
// ---------------------------------------------------------------------------

#[test]
fn std_dev_on_known_distribution() {
    // For values [2, 4, 4, 4, 5, 5, 7, 9], population std dev ≈ 2.0
    let fees = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    let v = compute_volatility(&fees);
    assert!(
        (v.standard_deviation - 2.0).abs() < 0.01,
        "expected std_dev ≈ 2.0, got {}",
        v.standard_deviation
    );
}

#[test]
fn std_dev_zero_for_constant_sequence() {
    let fees = vec![100.0_f64; 20];
    let v = compute_volatility(&fees);
    assert!(
        v.standard_deviation < f64::EPSILON,
        "constant sequence must have zero std_dev"
    );
}

#[test]
fn cv_is_scale_invariant() {
    let small: Vec<f64> = vec![1.0, 2.0, 3.0];
    let large: Vec<f64> = vec![10.0, 20.0, 30.0];
    let cv_small = compute_volatility(&small).coefficient_of_variation;
    let cv_large = compute_volatility(&large).coefficient_of_variation;
    assert!(
        (cv_small - cv_large).abs() < 1e-10,
        "CV must be scale-invariant: {} vs {}",
        cv_small,
        cv_large
    );
}

#[test]
fn cv_zero_for_constant_sequence() {
    let fees = vec![250.0_f64; 10];
    let v = compute_volatility(&fees);
    assert!(
        v.coefficient_of_variation < f64::EPSILON,
        "constant sequence must have zero CV"
    );
}

#[test]
fn volatility_max_and_min_correct() {
    let fees = vec![50.0, 100.0, 200.0, 75.0, 150.0];
    let v = compute_volatility(&fees);
    assert_eq!(v.max, 200.0);
    assert_eq!(v.min, 50.0);
    assert_eq!(v.range, 150.0);
}

#[test]
fn volatility_empty_slice_returns_zeros() {
    let v = compute_volatility(&[]);
    assert_eq!(v.standard_deviation, 0.0);
    assert_eq!(v.coefficient_of_variation, 0.0);
    assert_eq!(v.max, 0.0);
    assert_eq!(v.min, 0.0);
}

// ---------------------------------------------------------------------------
// Bollinger Bands — issue #462
// ---------------------------------------------------------------------------

#[test]
fn bollinger_upper_gt_sma_gt_lower_for_non_constant_sequence() {
    let fees: Vec<f64> = (0..50)
        .map(|i| 100.0 + (i as f64 * 7.3).sin() * 30.0)
        .collect();
    let bands = bollinger_bands(&fees, 10);
    // For any point where std_dev > 0 (non-constant window), upper > sma > lower.
    for b in &bands {
        if b.bandwidth > f64::EPSILON {
            assert!(
                b.upper_band > b.sma,
                "upper_band ({}) must be > sma ({})",
                b.upper_band,
                b.sma
            );
            assert!(
                b.sma > b.lower_band,
                "sma ({}) must be > lower_band ({})",
                b.sma,
                b.lower_band
            );
        }
    }
}

#[test]
fn bollinger_bandwidth_positive_for_non_constant() {
    let fees: Vec<f64> = (0..30).map(|i| 100.0 + i as f64 * 5.0).collect();
    let bands = bollinger_bands(&fees, 5);
    // After the first window fills (index >= window-1), bandwidth must be > 0.
    for b in bands.iter().skip(4) {
        assert!(
            b.bandwidth > 0.0,
            "bandwidth must be > 0 for non-constant window, got {}",
            b.bandwidth
        );
    }
}

#[test]
fn bollinger_bandwidth_zero_for_constant_sequence() {
    let fees = vec![200.0_f64; 20];
    let bands = bollinger_bands(&fees, 5);
    for b in &bands {
        assert!(
            b.bandwidth < f64::EPSILON,
            "constant sequence must have zero bandwidth, got {}",
            b.bandwidth
        );
    }
}

#[test]
fn bollinger_count_equals_input_length() {
    let fees: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let bands = bollinger_bands(&fees, 20);
    assert_eq!(bands.len(), fees.len());
}

#[test]
fn bollinger_sma_correct_at_full_window() {
    // First 5 values: 0, 1, 2, 3, 4 → SMA = 2.0 at index 4 with window=5.
    let fees: Vec<f64> = (0..10).map(|i| i as f64).collect();
    let bands = bollinger_bands(&fees, 5);
    assert!(
        (bands[4].sma - 2.0).abs() < 1e-10,
        "SMA at index 4 should be 2.0, got {}",
        bands[4].sma
    );
}

#[test]
fn bollinger_bands_empty_input_returns_empty() {
    let bands = bollinger_bands(&[], 10);
    assert!(bands.is_empty());
}

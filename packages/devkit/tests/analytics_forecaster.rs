//! Unit tests for the linear fee forecaster.
//!
//! Covers:
//! - Flat sequence forecasts flat values
//! - Rising sequence forecasts values higher than the last observation
//! - Falling sequence forecasts values lower than the last observation

use stellar_devkit::analytics::forecaster::{forecast_linear, forecast_holt};

// ---------------------------------------------------------------------------
// Linear forecaster — issue #464
// ---------------------------------------------------------------------------

#[test]
fn flat_sequence_forecasts_flat() {
    let fees = vec![100.0_f64; 20];
    let predictions = forecast_linear(&fees, 5);
    assert_eq!(predictions.len(), 5);
    for &p in &predictions {
        assert!(
            (p - 100.0).abs() < 0.01,
            "flat series must forecast ~100.0, got {p}"
        );
    }
}

#[test]
fn rising_sequence_forecasts_higher_than_last() {
    let fees: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 10.0).collect();
    let last = *fees.last().unwrap();
    let predictions = forecast_linear(&fees, 5);
    assert_eq!(predictions.len(), 5);
    for &p in &predictions {
        assert!(
            p > last,
            "rising sequence must forecast values above last={last}, got {p}"
        );
    }
}

#[test]
fn falling_sequence_forecasts_lower_than_last() {
    let fees: Vec<f64> = (0..20).map(|i| 500.0 - i as f64 * 10.0).collect();
    let last = *fees.last().unwrap();
    let predictions = forecast_linear(&fees, 5);
    for &p in &predictions {
        assert!(
            p < last,
            "falling sequence must forecast values below last={last}, got {p}"
        );
    }
}

#[test]
fn forecast_linear_returns_correct_horizon_length() {
    let fees: Vec<f64> = (0..10).map(|i| i as f64).collect();
    assert_eq!(forecast_linear(&fees, 0).len(), 0);
    assert_eq!(forecast_linear(&fees, 1).len(), 1);
    assert_eq!(forecast_linear(&fees, 10).len(), 10);
}

#[test]
fn forecast_linear_perfect_line_extrapolates_exactly() {
    // fees = [0, 5, 10, 15, 20] → next step should be ~25.
    let fees: Vec<f64> = (0..5).map(|i| i as f64 * 5.0).collect();
    let predictions = forecast_linear(&fees, 1);
    assert!(
        (predictions[0] - 25.0).abs() < 0.01,
        "linear extrapolation of [0,5,10,15,20] should give ~25, got {}",
        predictions[0]
    );
}

#[test]
fn forecast_linear_empty_slice() {
    let predictions = forecast_linear(&[], 3);
    assert_eq!(predictions.len(), 3);
    for &p in &predictions {
        assert_eq!(p, 0.0, "empty slice should forecast 0.0");
    }
}

// ---------------------------------------------------------------------------
// Holt's double exponential smoothing
// ---------------------------------------------------------------------------

#[test]
fn holt_rising_sequence_forecasts_higher_than_last() {
    let fees: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 10.0).collect();
    let last = *fees.last().unwrap();
    let predictions = forecast_holt(&fees, 5, 0.3, 0.1);
    for &p in &predictions {
        assert!(p > last, "Holt forecast must exceed last value for rising series, got {p}");
    }
}

#[test]
fn holt_flat_sequence_stays_near_last_value() {
    let fees = vec![200.0_f64; 30];
    let predictions = forecast_holt(&fees, 5, 0.3, 0.1);
    for &p in &predictions {
        assert!(
            (p - 200.0).abs() < 1.0,
            "Holt forecast of flat series should stay near 200, got {p}"
        );
    }
}

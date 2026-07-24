/// Issue #371: Unit tests for outlier detector
///
/// Assert Z-score method flags known outliers.
/// Assert clean sequences score below threshold.
///
/// File to touch: packages/devkit/tests/data_quality_validator.rs
/// (created as dedicated outlier test file for isolation)

use stellar_fee_tracker::outliers::{detect_outliers_zscore, OutlierResult};

// ── Clean sequences — no false positives ────────────────────────────────────

#[test]
fn test_uniform_sequence_no_outliers() {
    let values = vec![100.0; 20];
    let result: Vec<OutlierResult> = detect_outliers_zscore(&values, 3.0);
    let flagged: Vec<_> = result.iter().filter(|r| r.is_outlier).collect();
    assert!(flagged.is_empty(), "uniform values should not have outliers");
}

#[test]
fn test_linear_sequence_no_outliers() {
    let values: Vec<f64> = (0..20).map(|i| i as f64).collect();
    let result = detect_outliers_zscore(&values, 3.0);
    let flagged: Vec<_> = result.iter().filter(|r| r.is_outlier).collect();
    assert!(flagged.is_empty(), "linear sequence should not have outliers");
}

#[test]
fn test_small_sequence_below_minimum() {
    let values = vec![10.0, 12.0];
    let result = detect_outliers_zscore(&values, 3.0);
    // Too few samples — all should be non-outlier
    assert!(result.iter().all(|r| !r.is_outlier));
}

// ── Known outliers detected ─────────────────────────────────────────────────

#[test]
fn test_single_spike_detected() {
    let mut values = vec![50.0; 20];
    values[5] = 5000.0; // spike
    let result = detect_outliers_zscore(&values, 3.0);
    assert!(result[5].is_outlier, "spike at index 5 should be flagged");
    assert_eq!(result[5].z_score.abs(), result[5].z_score); // positive z-score
}

#[test]
fn test_single_dip_detected() {
    let mut values = vec![100.0; 15];
    values[10] = 0.5; // dip
    let result = detect_outliers_zscore(&values, 3.0);
    assert!(result[10].is_outlier, "dip at index 10 should be flagged");
}

#[test]
fn test_multiple_outliers_flagged() {
    let mut values = vec![50.0; 30];
    values[3] = 9999.0;
    values[20] = 8888.0;
    let result = detect_outliers_zscore(&values, 3.0);
    let flagged: Vec<_> = result.iter().filter(|r| r.is_outlier).collect();
    assert_eq!(flagged.len(), 2, "expected 2 outliers, found {}", flagged.len());
}

// ── Threshold sensitivity ────────────────────────────────────────────────────

#[test]
fn test_tight_threshold_flags_more() {
    let mut values = vec![100.0; 10];
    values[4] = 500.0;
    let loose = detect_outliers_zscore(&values, 3.0);
    let tight = detect_outliers_zscore(&values, 1.5);
    let loose_count = loose.iter().filter(|r| r.is_outlier).count();
    let tight_count = tight.iter().filter(|r| r.is_outlier).count();
    assert!(tight_count >= loose_count, "tighter threshold should flag >= outliers");
}

// ── Edge cases ───────────────────────────────────────────────────────────────

#[test]
fn test_all_identical_values_no_outliers() {
    let values = vec![42.0; 100];
    let result = detect_outliers_zscore(&values, 3.0);
    assert!(result.iter().all(|r| !r.is_outlier));
}

#[test]
fn test_single_element() {
    let values = vec![1.0];
    let result = detect_outliers_zscore(&values, 3.0);
    assert_eq!(result.len(), 1);
    assert!(!result[0].is_outlier);
}

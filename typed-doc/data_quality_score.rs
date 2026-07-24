/// Issue #372: Unit tests for quality score calculator
///
/// Assert perfect data scores 1.0.
/// Assert data with 10% invalid records scores < 0.95.
///
/// File to create: packages/devkit/tests/data_quality_score.rs
/// (mapped to packages/core/tests/data_quality_score.rs in current layout)

use stellar_fee_tracker::quality::{compute_quality_score, QualityInput, QualityScore};

// ── Perfect data ─────────────────────────────────────────────────────────────

#[test]
fn test_perfect_data_scores_one() {
    let input = QualityInput {
        total_records: 1000,
        valid_records: 1000,
        gaps: 0,
        outliers: 0,
        missing_fields: 0,
    };
    let score = compute_quality_score(&input);
    assert!((score.overall - 1.0).abs() < f64::EPSILON);
    assert!((score.validity - 1.0).abs() < f64::EPSILON);
    assert!((score.completeness - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_single_record_perfect() {
    let input = QualityInput {
        total_records: 1,
        valid_records: 1,
        gaps: 0,
        outliers: 0,
        missing_fields: 0,
    };
    let score = compute_quality_score(&input);
    assert!((score.overall - 1.0).abs() < f64::EPSILON);
}

// ── Degraded data ────────────────────────────────────────────────────────────

#[test]
fn test_ten_percent_invalid_scores_below_threshold() {
    let input = QualityInput {
        total_records: 1000,
        valid_records: 900,   // 10% invalid
        gaps: 0,
        outliers: 0,
        missing_fields: 0,
    };
    let score = compute_quality_score(&input);
    assert!(score.overall < 0.95, "expected < 0.95, got {}", score.overall);
    assert!((score.validity - 0.9).abs() < 0.01);
}

#[test]
fn test_fifty_percent_invalid_low_score() {
    let input = QualityInput {
        total_records: 100,
        valid_records: 50,
        gaps: 0,
        outliers: 0,
        missing_fields: 0,
    };
    let score = compute_quality_score(&input);
    assert!(score.overall < 0.6);
    assert!((score.validity - 0.5).abs() < 0.01);
}

// ── Gap penalties ────────────────────────────────────────────────────────────

#[test]
fn test_gaps_reduce_completeness() {
    let no_gaps = QualityInput { total_records: 100, valid_records: 100, gaps: 0, outliers: 0, missing_fields: 0 };
    let with_gaps = QualityInput { total_records: 100, valid_records: 100, gaps: 5, outliers: 0, missing_fields: 0 };

    let score_no = compute_quality_score(&no_gaps);
    let score_yes = compute_quality_score(&with_gaps);

    assert!(score_yes.completeness < score_no.completeness);
    assert!(score_yes.overall < score_no.overall);
}

// ── Outlier penalties ───────────────────────────────────────────────────────

#[test]
fn test_outliers_reduce_consistency() {
    let clean = QualityInput { total_records: 100, valid_records: 100, gaps: 0, outliers: 0, missing_fields: 0 };
    let dirty = QualityInput { total_records: 100, valid_records: 100, gaps: 0, outliers: 10, missing_fields: 0 };

    let score_clean = compute_quality_score(&clean);
    let score_dirty = compute_quality_score(&dirty);

    assert!(score_dirty.overall < score_clean.overall);
}

// ── Mixed degradation ────────────────────────────────────────────────────────

#[test]
fn test_combined_degradation_compounds() {
    let input = QualityInput {
        total_records: 1000,
        valid_records: 920,
        gaps: 3,
        outliers: 5,
        missing_fields: 2,
    };
    let score = compute_quality_score(&input);
    assert!(score.overall < 1.0);
    assert!(score.validity > 0.0 && score.validity < 1.0);
    assert!(score.completeness > 0.0 && score.completeness < 1.0);
}

// ── Edge cases ───────────────────────────────────────────────────────────────

#[test]
fn test_zero_total_records_scores_zero() {
    let input = QualityInput { total_records: 0, valid_records: 0, gaps: 0, outliers: 0, missing_fields: 0 };
    let score = compute_quality_score(&input);
    assert!((score.overall - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_missing_fields_penalty() {
    let input = QualityInput { total_records: 100, valid_records: 100, gaps: 0, outliers: 0, missing_fields: 15 };
    let score = compute_quality_score(&input);
    assert!(score.overall < 1.0);
    assert!(score.completeness < 1.0);
}

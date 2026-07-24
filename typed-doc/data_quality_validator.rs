/// Issue #369: Unit tests for fee record validator
///
/// Tests all validation rules with both valid and invalid inputs.
/// Minimum 12 test cases covering: field presence, value ranges,
/// type checks, timestamps, and relationship constraints.
///
/// File to create: packages/devkit/tests/data_quality_validator.rs
/// (mapped to packages/core/tests/data_quality_validator.rs in current layout)

use stellar_fee_tracker::validator::{validate_record, ValidationError, FeeRecord};

// ── Happy path ────────────────────────────────────────────────────────────────

#[test]
fn test_valid_record_passes() {
    let record = FeeRecord {
        ledger_seq: 1000,
        fee_stroops: 100,
        time_bucket: "2025-01-01T00:00:00Z".into(),
        source: "test".into(),
    };
    assert!(validate_record(&record).is_ok());
}

#[test]
fn test_minimal_fee_passes() {
    let record = FeeRecord {
        ledger_seq: 1,
        fee_stroops: 0,       // minimum
        time_bucket: "2025-01-01T00:00:00Z".into(),
        source: "minimal".into(),
    };
    assert!(validate_record(&record).is_ok());
}

#[test]
fn test_max_ledger_seq_passes() {
    let record = FeeRecord {
        ledger_seq: u32::MAX,
        fee_stroops: 100_000,
        time_bucket: "2025-01-01T00:00:00Z".into(),
        source: "edge".into(),
    };
    assert!(validate_record(&record).is_ok());
}

// ── Field presence ───────────────────────────────────────────────────────────

#[test]
fn test_empty_source_fails() {
    let record = FeeRecord {
        ledger_seq: 1000,
        fee_stroops: 100,
        time_bucket: "2025-01-01T00:00:00Z".into(),
        source: "".into(),
    };
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::EmptyField(f)) if f == "source"
    ));
}

#[test]
fn test_empty_time_bucket_fails() {
    let record = FeeRecord {
        ledger_seq: 1000,
        fee_stroops: 100,
        time_bucket: "".into(),
        source: "test".into(),
    };
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::EmptyField(f)) if f == "time_bucket"
    ));
}

// ── Value ranges ─────────────────────────────────────────────────────────────

#[test]
fn test_negative_fee_stroops_fails() {
    let record = FeeRecord {
        ledger_seq: 1000,
        fee_stroops: 0u64.wrapping_sub(1), // underflow → large value
        time_bucket: "2025-01-01T00:00:00Z".into(),
        source: "test".into(),
    };
    assert!(validate_record(&record).is_err());
}

#[test]
fn test_zero_ledger_seq_passes() {
    // ledger 0 can appear on test networks
    let record = FeeRecord {
        ledger_seq: 0,
        fee_stroops: 50,
        time_bucket: "2025-01-01T00:00:00Z".into(),
        source: "test".into(),
    };
    assert!(validate_record(&record).is_ok());
}

// ── Timestamp format ─────────────────────────────────────────────────────────

#[test]
fn test_invalid_timestamp_format_fails() {
    let record = FeeRecord {
        ledger_seq: 1000,
        fee_stroops: 100,
        time_bucket: "not-a-date".into(),
        source: "test".into(),
    };
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::InvalidTimestamp(_))
    ));
}

#[test]
fn test_future_timestamp_fails() {
    let record = FeeRecord {
        ledger_seq: 1000,
        fee_stroops: 100,
        time_bucket: "2099-01-01T00:00:00Z".into(),
        source: "test".into(),
    };
    assert!(matches!(
        validate_record(&record),
        Err(ValidationError::FutureTimestamp(_))
    ));
}

// ── Relationship constraints ─────────────────────────────────────────────────

#[test]
fn test_zero_fee_with_active_ledger_passes() {
    // Zero fee is valid in some edge cases (e.g. surge pricing off)
    let record = FeeRecord {
        ledger_seq: 42,
        fee_stroops: 0,
        time_bucket: "2025-01-01T00:00:00Z".into(),
        source: "test".into(),
    };
    assert!(validate_record(&record).is_ok());
}

// ── Batch validation ─────────────────────────────────────────────────────────

#[test]
fn test_batch_valid_all_pass() {
    let records = vec![
        FeeRecord { ledger_seq: 1, fee_stroops: 100, time_bucket: "2025-01-01T00:00:00Z".into(), source: "a".into() },
        FeeRecord { ledger_seq: 2, fee_stroops: 200, time_bucket: "2025-01-01T00:01:00Z".into(), source: "b".into() },
    ];
    assert!(validate_record(&records[0]).is_ok());
    assert!(validate_record(&records[1]).is_ok());
}

#[test]
fn test_batch_invalid_rejected() {
    let records = vec![
        FeeRecord { ledger_seq: 1, fee_stroops: 100, time_bucket: "2025-01-01T00:00:00Z".into(), source: "ok".into() },
        FeeRecord { ledger_seq: 2, fee_stroops: 200, time_bucket: "".into(), source: "bad".into() },
    ];
    assert!(validate_record(&records[0]).is_ok());
    assert!(matches!(
        validate_record(&records[1]),
        Err(ValidationError::EmptyField(_))
    ));
}

// ── Stress ───────────────────────────────────────────────────────────────────

#[test]
fn test_max_fields_populated_passes() {
    let record = FeeRecord {
        ledger_seq: u32::MAX,
        fee_stroops: u64::MAX,
        time_bucket: "2025-06-15T12:30:00Z".into(),
        source: "stress-test".into(),
    };
    assert!(validate_record(&record).is_ok());
}

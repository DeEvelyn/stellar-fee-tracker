use stellar_devkit::protocol::parse_fee_stats;

fn minimal_valid_json() -> &'static str {
    r#"{"last_ledger_base_fee": 100, "ledger_capacity_usage": 0.5}"#
}

#[test]
fn parse_minimal_valid() {
    let stats = parse_fee_stats(minimal_valid_json()).unwrap();
    assert_eq!(stats.last_ledger_base_fee, 100);
    assert!((stats.ledger_capacity_usage - 0.5).abs() < f64::EPSILON);
}

#[test]
fn parse_full_json() {
    let json = r#"{
        "last_ledger_base_fee": 200,
        "ledger_capacity_usage": 0.75,
        "min": 100,
        "mode": 200,
        "max": 500,
        "p10": 100,
        "p20": 150,
        "p30": 200,
        "min_accepted_fee": 100,
        "max_accepted_fee": 1000,
        "transaction_count_estimate": 500
    }"#;
    let stats = parse_fee_stats(json).unwrap();
    assert_eq!(stats.last_ledger_base_fee, 200);
    assert_eq!(stats.min, Some(100));
    assert_eq!(stats.max, Some(500));
}

#[test]
fn parse_with_fee_charged() {
    let json = r#"{
        "last_ledger_base_fee": 100,
        "ledger_capacity_usage": 0.3,
        "fee_charged": {
            "min": 100,
            "mode": 200,
            "max": 300,
            "p10": 100,
            "p50": 200,
            "p90": 300,
            "p99": 500
        }
    }"#;
    let stats = parse_fee_stats(json).unwrap();
    let fc = stats.fee_charged.unwrap();
    assert_eq!(fc.p10, Some(100));
    assert_eq!(fc.p99, Some(500));
}

#[test]
fn parse_missing_optional_fields() {
    let json = r#"{"last_ledger_base_fee": 100, "ledger_capacity_usage": 0.5}"#;
    let stats = parse_fee_stats(json).unwrap();
    assert!(stats.fee_charged.is_none());
    assert!(stats.min.is_none());
}

#[test]
fn parse_invalid_json_fails() {
    assert!(parse_fee_stats("not json").is_err());
}

#[test]
fn parse_empty_object_fails() {
    assert!(parse_fee_stats("{}").is_err());
}

#[test]
fn parse_base_fee_too_low_fails() {
    let json = r#"{"last_ledger_base_fee": 10, "ledger_capacity_usage": 0.5}"#;
    assert!(parse_fee_stats(json).is_err());
}

#[test]
fn parse_capacity_over_one_fails() {
    let json = r#"{"last_ledger_base_fee": 100, "ledger_capacity_usage": 2.0}"#;
    assert!(parse_fee_stats(json).is_err());
}

#[test]
fn parse_negative_capacity_fails() {
    let json = r#"{"last_ledger_base_fee": 100, "ledger_capacity_usage": -0.5}"#;
    assert!(parse_fee_stats(json).is_err());
}

#[test]
fn parse_non_monotonic_percentiles_fails() {
    let json = r#"{
        "last_ledger_base_fee": 100,
        "ledger_capacity_usage": 0.5,
        "fee_charged": {
            "p10": 300,
            "p50": 200,
            "p90": 100,
            "p99": 50
        }
    }"#;
    assert!(parse_fee_stats(json).is_err());
}

#[test]
fn parse_realistic_horizon_response() {
    let json = r#"{
        "last_ledger_base_fee": 100,
        "ledger_capacity_usage": 0.06177501079832306,
        "min_accepted_fee": 100,
        "max_accepted_fee": 10000000,
        "min": 100,
        "mode": 100,
        "max": 601106,
        "p10": 100,
        "p20": 100,
        "p30": 100,
        "fee_charged": {
            "max": 601106,
            "min": 100,
            "mode": 100,
            "p10": 100,
            "p20": 100,
            "p30": 100,
            "p40": 100,
            "p50": 100,
            "p60": 100,
            "p70": 100,
            "p80": 100,
            "p90": 266780,
            "p95": 504594,
            "p99": 601106,
            "transaction_count": 214
        },
        "max_fee": {
            "max": 601106,
            "min": 100,
            "mode": 100,
            "p10": 100,
            "p20": 100,
            "p30": 100,
            "p40": 100,
            "p50": 100,
            "p60": 100,
            "p70": 100,
            "p80": 100,
            "p90": 266780,
            "p95": 504594,
            "p99": 601106,
            "transaction_count": 214
        }
    }"#;
    let stats = parse_fee_stats(json).unwrap();
    assert_eq!(stats.last_ledger_base_fee, 100);
    assert!(stats.fee_charged.is_some());
    assert!(stats.max_fee.is_some());
}

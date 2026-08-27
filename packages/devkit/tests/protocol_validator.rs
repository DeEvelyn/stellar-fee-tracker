use stellar_devkit::protocol::fee_stats::HorizonFeeStats;
use stellar_devkit::protocol::parser::validate_fee_stats;

fn valid_stats() -> HorizonFeeStats {
    HorizonFeeStats {
        last_ledger_base_fee: 100,
        ledger_capacity_usage: 0.5,
        min: Some(100),
        mode: Some(200),
        max: Some(300),
        p10: Some(100),
        p20: Some(150),
        p30: Some(200),
        min_accepted_fee: None,
        max_accepted_fee: None,
        transaction_count_estimate: None,
        fee_charged: None,
        max_fee: None,
    }
}

#[test]
fn valid_stats_passes() {
    assert!(validate_fee_stats(&valid_stats()).is_ok());
}

#[test]
fn base_fee_below_100_fails() {
    let mut stats = valid_stats();
    stats.last_ledger_base_fee = 50;
    assert!(validate_fee_stats(&stats).is_err());
}

#[test]
fn capacity_usage_out_of_range_fails() {
    let mut stats = valid_stats();
    stats.ledger_capacity_usage = 1.5;
    assert!(validate_fee_stats(&stats).is_err());
}

#[test]
fn negative_capacity_usage_fails() {
    let mut stats = valid_stats();
    stats.ledger_capacity_usage = -0.1;
    assert!(validate_fee_stats(&stats).is_err());
}

#[test]
fn capacity_usage_boundary_zero_ok() {
    let mut stats = valid_stats();
    stats.ledger_capacity_usage = 0.0;
    assert!(validate_fee_stats(&stats).is_ok());
}

#[test]
fn capacity_usage_boundary_one_ok() {
    let mut stats = valid_stats();
    stats.ledger_capacity_usage = 1.0;
    assert!(validate_fee_stats(&stats).is_ok());
}

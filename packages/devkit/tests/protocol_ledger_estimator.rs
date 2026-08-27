use chrono::Utc;
use stellar_devkit::protocol::estimate_close_time;

#[test]
fn estimate_returns_future_for_target_greater_than_current() {
    let estimate = estimate_close_time(100, 110, 5000);
    assert!(estimate > Utc::now());
}

#[test]
fn estimate_proportional_to_ledger_distance() {
    let short = estimate_close_time(100, 101, 5000);
    let medium = estimate_close_time(100, 110, 5000);
    let long = estimate_close_time(100, 1000, 5000);
    assert!(medium > short);
    assert!(long > medium);
}

#[test]
fn estimate_same_ledger_returns_now_or_future() {
    let estimate = estimate_close_time(100, 100, 5000);
    assert!(estimate >= Utc::now() - chrono::Duration::seconds(1));
}

#[test]
fn estimate_scales_with_avg_close_time() {
    let fast = estimate_close_time(100, 110, 1000);
    let slow = estimate_close_time(100, 110, 10000);
    assert!(slow > fast);
}

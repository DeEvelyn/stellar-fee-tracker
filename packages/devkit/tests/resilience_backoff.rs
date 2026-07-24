use std::time::Duration;
use stellar_devkit::resilience::backoff::linear_backoff;

#[test]
fn linear_growth() {
    let d0 = linear_backoff(0, 100, 10_000);
    let d1 = linear_backoff(1, 100, 10_000);
    let d2 = linear_backoff(2, 100, 10_000);
    let d5 = linear_backoff(5, 100, 10_000);
    assert_eq!(d0, Duration::from_millis(100));
    assert_eq!(d1, Duration::from_millis(100));
    assert_eq!(d2, Duration::from_millis(200));
    assert_eq!(d5, Duration::from_millis(500));
}

#[test]
fn caps_at_max() {
    let d = linear_backoff(200, 100, 5_000);
    assert_eq!(d, Duration::from_millis(5_000));
}

#[test]
fn attempt_zero_returns_base() {
    let d = linear_backoff(0, 250, 10_000);
    assert_eq!(d, Duration::from_millis(250));
}

#[test]
fn large_attempt_does_not_overflow() {
    let d = linear_backoff(u32::MAX, 100, 10_000);
    assert_eq!(d, Duration::from_millis(10_000));
}

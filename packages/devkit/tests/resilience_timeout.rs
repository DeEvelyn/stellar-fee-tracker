use std::time::Duration;

use stellar_devkit::resilience::timeout::{with_timeout, TimeoutError};

#[tokio::test]
async fn fast_operation_completes() {
    let result = with_timeout(Duration::from_secs(1), || async { 42 }).await;
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn slow_operation_times_out() {
    let result = with_timeout(Duration::from_millis(10), || async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        42
    })
    .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.elapsed, Duration::from_millis(10));
}

#[tokio::test]
async fn timeout_error_display() {
    let err = TimeoutError {
        elapsed: Duration::from_millis(500),
    };
    assert!(err.to_string().contains("500ms"));
}

#[tokio::test]
async fn exact_boundary_completes() {
    let result = with_timeout(Duration::from_millis(100), || async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        "done"
    })
    .await;
    assert_eq!(result.unwrap(), "done");
}

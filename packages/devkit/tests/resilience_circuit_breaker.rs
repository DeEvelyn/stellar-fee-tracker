use std::time::Duration;

use stellar_devkit::resilience::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState,
};

#[tokio::test]
async fn closed_to_open_after_threshold_failures() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        ..Default::default()
    });
    assert_eq!(cb.state().await, CircuitState::Closed);
    cb.record_failure().await;
    assert_eq!(cb.state().await, CircuitState::Closed);
    cb.record_failure().await;
    assert_eq!(cb.state().await, CircuitState::Closed);
    cb.record_failure().await;
    assert_eq!(cb.state().await, CircuitState::Open);
    assert!(!cb.allow_request().await);
}

#[tokio::test]
async fn open_to_half_open_after_duration() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 1,
        open_duration: Duration::from_millis(50),
        ..Default::default()
    });
    cb.record_failure().await;
    assert_eq!(cb.state().await, CircuitState::Open);
    assert!(!cb.allow_request().await);
    tokio::time::sleep(Duration::from_millis(60)).await;
    assert!(cb.allow_request().await);
    assert_eq!(cb.state().await, CircuitState::HalfOpen);
}

#[tokio::test]
async fn half_open_to_closed_after_successes() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 1,
        success_threshold: 2,
        open_duration: Duration::from_millis(10),
        ..Default::default()
    });
    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    cb.allow_request().await;
    assert_eq!(cb.state().await, CircuitState::HalfOpen);
    cb.record_success().await;
    assert_eq!(cb.state().await, CircuitState::HalfOpen);
    cb.record_success().await;
    assert_eq!(cb.state().await, CircuitState::Closed);
}

#[tokio::test]
async fn half_open_failure_reopens() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 1,
        open_duration: Duration::from_millis(10),
        ..Default::default()
    });
    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    cb.allow_request().await;
    assert_eq!(cb.state().await, CircuitState::HalfOpen);
    cb.record_failure().await;
    assert_eq!(cb.state().await, CircuitState::Open);
}

#[tokio::test]
async fn success_resets_failure_count_in_closed() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        ..Default::default()
    });
    cb.record_failure().await;
    cb.record_failure().await;
    cb.record_success().await;
    assert_eq!(cb.state().await, CircuitState::Closed);
    cb.record_failure().await;
    assert_eq!(cb.state().await, CircuitState::Closed);
    cb.record_failure().await;
    assert_eq!(cb.state().await, CircuitState::Closed);
}

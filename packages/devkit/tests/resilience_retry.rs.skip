use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use stellar_devkit::resilience::{retry, RetryConfig};

#[tokio::test]
async fn success_on_first_attempt_skips_retries() {
    let config = RetryConfig {
        max_attempts: 3,
        initial_backoff_ms: 100,
    };
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = call_count.clone();

    let result = retry(
        &config,
        move || {
            let cc = call_count_clone.clone();
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>("ok".to_string())
            }
        },
        |_| {},
    )
    .await;

    assert_eq!(result.unwrap(), "ok");
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn failure_exhausts_max_attempts() {
    let config = RetryConfig {
        max_attempts: 3,
        initial_backoff_ms: 100,
    };
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = call_count.clone();

    let result: Result<String, String> = retry(
        &config,
        move || {
            let cc = call_count_clone.clone();
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                Err("fail".to_string())
            }
        },
        |_| {},
    )
    .await;

    assert_eq!(result.unwrap_err(), "fail");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn correct_attempt_count_passed_to_backoff() {
    let config = RetryConfig {
        max_attempts: 5,
        initial_backoff_ms: 100,
    };
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = call_count.clone();
    let backoff_attempts = Arc::new(std::sync::Mutex::new(Vec::new()));
    let backoff_attempts_clone = backoff_attempts.clone();

    let result: Result<String, String> = retry(
        &config,
        move || {
            let cc = call_count_clone.clone();
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                Err("fail".to_string())
            }
        },
        move |attempt| {
            backoff_attempts_clone.lock().unwrap().push(attempt);
        },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(call_count.load(Ordering::SeqCst), 5);
    assert_eq!(*backoff_attempts.lock().unwrap(), vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn succeeds_after_retries() {
    let config = RetryConfig {
        max_attempts: 3,
        initial_backoff_ms: 100,
    };
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = call_count.clone();

    let result = retry(
        &config,
        move || {
            let cc = call_count_clone.clone();
            async move {
                let attempt = cc.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err("fail".to_string())
                } else {
                    Ok("ok".to_string())
                }
            }
        },
        |_| {},
    )
    .await;

    assert_eq!(result.unwrap(), "ok");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

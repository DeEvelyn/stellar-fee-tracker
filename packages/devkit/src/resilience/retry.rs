use std::future::Future;
use std::time::Duration;

use super::circuit_breaker::CircuitBreaker;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff_ms: u64,
}

/// Execute an async closure with retries guarded by a circuit breaker.
///
/// If the circuit breaker is open, the operation is skipped and the last error
/// is returned immediately. On each failure, the circuit breaker is notified
/// and a fixed backoff delay is applied before the next attempt.
pub async fn retry_with_circuit_breaker<T, E, F, Fut>(
    retry_config: RetryConfig,
    cb: &CircuitBreaker,
    f: F,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut last_err = None;
    for attempt in 0..retry_config.max_attempts {
        if !cb.allow_request().await {
            return Err(last_err.unwrap_or_else(|| {
                unreachable!("first iteration always allows request")
            }));
        }

        match f().await {
            Ok(val) => {
                cb.record_success().await;
                return Ok(val);
            }
            Err(e) => {
                cb.record_failure().await;
                last_err = Some(e);
                if attempt + 1 < retry_config.max_attempts {
                    tokio::time::sleep(Duration::from_millis(retry_config.backoff_ms)).await;
                }
            }
        }
    }
    Err(last_err.unwrap())
}

#[cfg(test)]
mod tests {
    use super::super::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
    use super::{retry_with_circuit_breaker, RetryConfig};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn success_on_first_attempt() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let config = RetryConfig {
            max_attempts: 3,
            backoff_ms: 1,
        };
        let result = retry_with_circuit_breaker(config, &cb, || async {
            Ok::<_, String>("ok")
        })
        .await;
        assert_eq!(result.unwrap(), "ok");
    }

    #[tokio::test]
    async fn retries_after_failure() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();
        let config = RetryConfig {
            max_attempts: 5,
            backoff_ms: 1,
        };
        let result = retry_with_circuit_breaker(config, &cb, || {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err("fail".to_string())
                } else {
                    Ok("ok")
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), "ok");
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            open_duration: Duration::from_secs(60),
            ..Default::default()
        });
        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();
        let config = RetryConfig {
            max_attempts: 10,
            backoff_ms: 1,
        };
        let result: Result<(), String> = retry_with_circuit_breaker(config, &cb, || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err("fail".to_string())
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert_eq!(cb.state().await, CircuitState::Open);
    }
}

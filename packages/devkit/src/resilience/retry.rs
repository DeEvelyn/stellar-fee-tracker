use std::future::Future;

use super::backoff::{BackoffStrategy, compute_delay};

/// Configuration for a retry executor.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts (including the first call).
    pub max_attempts: u32,
    /// Backoff strategy between retries.
    pub backoff: BackoffStrategy,
}

/// Execute an async closure with automatic retries on failure.
///
/// Returns `Ok(T)` on the first successful invocation, or `Err(E)` after
/// `max_attempts` failures.
pub async fn retry<T, E, F, Fut>(config: RetryConfig, f: F) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut last_err = None;

    for attempt in 0..config.max_attempts {
        match f().await {
            Ok(val) => return Ok(val),
            Err(err) => {
                last_err = Some(err);
                if attempt + 1 < config.max_attempts {
                    let delay = compute_delay(&config.backoff, attempt);
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    Err(last_err.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn succeeds_on_first_try() {
        let config = RetryConfig {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed { delay_ms: 1 },
        };
        let result = retry(config, || async { Ok::<_, &str>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn retries_then_succeeds() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let a = attempts.clone();
        let config = RetryConfig {
            max_attempts: 5,
            backoff: BackoffStrategy::Fixed { delay_ms: 1 },
        };
        let result = retry(config, || {
            let a = a.clone();
            async move {
                if a.fetch_add(1, Ordering::SeqCst) < 2 {
                    Err("fail")
                } else {
                    Ok(99)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 99);
    }

    #[tokio::test]
    async fn exhausts_attempts() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let a = attempts.clone();
        let config = RetryConfig {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed { delay_ms: 1 },
        };
        let result = retry(config, || {
            let a = a.clone();
            async move {
                a.fetch_add(1, Ordering::SeqCst);
                Err::<(), &str>("always fail")
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}

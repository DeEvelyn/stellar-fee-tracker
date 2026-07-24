use std::future::Future;
use std::time::Duration;

use super::backoff::{exponential_backoff, linear_backoff};

/// Backoff strategy used between retry attempts.
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Exponential: `min(base_ms * 2^attempt, max_ms)` with optional jitter.
    Exponential { base_ms: u64, max_ms: u64, jitter: bool },
    /// Linear: `min(base_ms * attempt, max_ms)`.
    Linear { base_ms: u64, max_ms: u64 },
    /// Fixed delay between retries.
    Fixed(Duration),
}

/// Configuration for the retry executor.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
}

/// Execute an async closure with automatic retries on failure.
///
/// Calls `f()` up to `config.max_attempts` times. Returns `Ok(T)` on the first
/// successful invocation, or `Err(E)` after all attempts are exhausted.
pub async fn retry<T, E, F, Fut>(config: RetryConfig, f: F) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut last_err = None;
    for attempt in 0..config.max_attempts {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < config.max_attempts {
                    let delay = compute_delay(&config.backoff, attempt);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    Err(last_err.unwrap())
}

fn compute_delay(strategy: &BackoffStrategy, attempt: u32) -> Duration {
    match strategy {
        BackoffStrategy::Exponential { base_ms, max_ms, jitter } => {
            exponential_backoff(attempt, *base_ms, *max_ms, *jitter)
        }
        BackoffStrategy::Linear { base_ms, max_ms } => {
            linear_backoff(attempt, *base_ms, *max_ms)
        }
        BackoffStrategy::Fixed(d) => *d,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn success_on_first_attempt_skips_retries() {
        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();
        let config = RetryConfig {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed(Duration::from_millis(1)),
        };
        let result = retry(config, || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>("ok")
            }
        })
        .await;
        assert_eq!(result.unwrap(), "ok");
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn failure_exhausts_max_attempts() {
        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();
        let config = RetryConfig {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed(Duration::from_millis(1)),
        };
        let result: Result<(), String> = retry(config, || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err("fail".to_string())
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn succeeds_after_initial_failures() {
        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();
        let config = RetryConfig {
            max_attempts: 5,
            backoff: BackoffStrategy::Fixed(Duration::from_millis(1)),
        };
        let result = retry(config, || {
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
}

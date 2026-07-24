use std::future::Future;

/// Configuration for retry behaviour.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
        }
    }
}

/// Execute an async closure with retry logic.
///
/// On each failure the `backoff_fn` is called with the current attempt number
/// (1-based) so callers can sleep / log before the next attempt.
pub async fn retry<F, Fut, T, E, B>(
    config: &RetryConfig,
    mut op: F,
    mut backoff_fn: B,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    B: FnMut(u32),
{
    let mut attempt = 0u32;
    let mut last_err = None;

    while attempt < config.max_attempts {
        match op().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_err = Some(e);
                attempt += 1;
                if attempt < config.max_attempts {
                    backoff_fn(attempt);
                }
            }
        }
    }

    Err(last_err.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn success_on_first_attempt_skips_retries() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 100,
        };
        let mut backoff_calls = 0u32;

        let result = retry(
            &config,
            || async { Ok::<_, String>("ok".to_string()) },
            |_| {
                backoff_calls += 1;
            },
        )
        .await;

        assert_eq!(result.unwrap(), "ok");
        assert_eq!(backoff_calls, 0);
    }

    #[tokio::test]
    async fn failure_exhausts_max_attempts() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff_ms: 100,
        };

        let result: Result<String, String> = retry(
            &config,
            || async { Err("fail".to_string()) },
            |_| {},
        )
        .await;

        assert_eq!(result.unwrap_err(), "fail");
    }

    #[tokio::test]
    async fn correct_attempt_count_passed_to_backoff() {
        let config = RetryConfig {
            max_attempts: 4,
            initial_backoff_ms: 100,
        };
        let mut attempts_seen = Vec::new();

        let result: Result<String, String> = retry(
            &config,
            || async { Err("fail".to_string()) },
            |attempt| {
                attempts_seen.push(attempt);
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(attempts_seen, vec![1, 2, 3]);
    }
}

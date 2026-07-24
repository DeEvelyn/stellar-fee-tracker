use std::future::Future;
use std::time::Duration;

/// Error returned when an operation exceeds the specified timeout.
#[derive(Debug)]
pub struct TimeoutError {
    pub elapsed: Duration,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operation timed out after {:?}", self.elapsed)
    }
}

impl std::error::Error for TimeoutError {}

/// Wrap an async operation with a configurable timeout.
///
/// Returns `Err(TimeoutError)` if the operation does not complete within `duration`.
pub async fn with_timeout<T, F, Fut>(duration: Duration, f: F) -> Result<T, TimeoutError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    match tokio::time::timeout(duration, f()).await {
        Ok(val) => Ok(val),
        Err(_elapsed) => Err(TimeoutError { elapsed: duration }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}

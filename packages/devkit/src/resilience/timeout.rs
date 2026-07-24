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

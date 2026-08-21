use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Concurrency limiter that restricts the number of simultaneous operations.
#[derive(Clone)]
pub struct Bulkhead {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

/// Error returned when the bulkhead is at capacity in fail-fast mode.
#[derive(Debug)]
pub struct BulkheadFull;

impl std::fmt::Display for BulkheadFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bulkhead at capacity")
    }
}

impl std::error::Error for BulkheadFull {}

impl Bulkhead {
    /// Create a new bulkhead with the given maximum concurrency.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// The configured maximum concurrency.
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Acquire a permit in queue mode (waits until a slot opens).
    pub async fn acquire(&self) -> OwnedSemaphorePermit {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore closed unexpectedly")
    }

    /// Try to acquire a permit in fail-fast mode (returns None if full).
    pub fn try_acquire(&self) -> Option<OwnedSemaphorePermit> {
        self.semaphore.clone().try_acquire_owned().ok()
    }
}

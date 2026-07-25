//! Async sandbox runner.

use std::future::Future;
use crate::sandbox::environment::{SandboxEnv, Scenario, SandboxResult};

/// Execute a closure against a fresh [`SandboxEnv`] for the given [`Scenario`].
///
/// ```rust,ignore
/// use stellar_devkit::sandbox::runner::run;
/// use stellar_devkit::sandbox::environment::Scenario;
///
/// let count = run(Scenario::Normal, |env| async move { env.len() }).await.unwrap();
/// assert_eq!(count, 10_000);
/// ```
pub async fn run<F, Fut, T>(scenario: Scenario, f: F) -> SandboxResult<T>
where
    F: FnOnce(SandboxEnv) -> Fut,
    Fut: Future<Output = T>,
{
    let env = SandboxEnv::new();
    let _ = scenario; // env is pre-seeded; scenario can be used to filter if needed
    Ok(f(env).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_returns_record_count() {
        let count = run(Scenario::Normal, |env| async move { env.len() })
            .await
            .unwrap();
        assert_eq!(count, 10_000);
    }

    #[tokio::test]
    async fn run_congested_scenario() {
        let count = run(Scenario::Congested, |env| async move { env.len() })
            .await
            .unwrap();
        assert_eq!(count, 10_000);
    }
}

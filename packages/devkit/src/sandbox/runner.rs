//! Synchronous test runner for the sandbox environment.

use std::time::Duration as StdDuration;

use crate::sandbox::environment::SandboxEnv;

/// Runs `f` with a freshly-seeded [`SandboxEnv`] and returns its result.
pub fn run<F, T>(f: F) -> T
where
    F: FnOnce(&SandboxEnv) -> T,
{
    let env = SandboxEnv::new();
    f(&env)
}

/// Collects timing and record-count metadata for a sandbox run.
pub struct ResultCollector {
    duration: StdDuration,
    records_processed: usize,
}

impl ResultCollector {
    pub fn new() -> Self {
        Self {
            duration: StdDuration::ZERO,
            records_processed: 0,
        }
    }

    pub fn record_duration(&mut self, d: StdDuration) {
        self.duration = d;
    }

    pub fn duration(&self) -> StdDuration {
        self.duration
    }

    pub fn set_records_processed(&mut self, n: usize) {
        self.records_processed = n;
    }

    pub fn records_processed(&self) -> usize {
        self.records_processed
    }
}

impl Default for ResultCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_receives_seeded_env() {
        let count = run(|env| env.len());
        assert_eq!(count, 10_000);
    }
}

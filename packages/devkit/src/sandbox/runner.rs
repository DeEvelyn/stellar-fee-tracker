//! Synchronous test runner for the sandbox environment.

use crate::sandbox::environment::SandboxEnv;

/// Runs `f` with a freshly-seeded [`SandboxEnv`] and returns its result.
pub fn run<F, T>(f: F) -> T
where
    F: FnOnce(&SandboxEnv) -> T,
{
    let env = SandboxEnv::new();
    f(&env)
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

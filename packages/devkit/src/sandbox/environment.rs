//! In-memory fee database for the sandbox environment.

use crate::sandbox::fixtures;

/// Network scenario/regime enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    Normal,
    Rising,
    Congested,
    Spike,
}

/// Result type for sandbox operations.
pub type SandboxResult<T> = Result<T, String>;

/// A single fee observation.
#[derive(Debug, Clone)]
pub struct FeeRecord {
    pub timestamp_ms: u64,
    pub fee_stroops: u64,
    pub scenario: Scenario,
}

/// In-memory fee database pre-seeded with 10,000 records.
pub struct SandboxEnv {
    records: Vec<FeeRecord>,
}

impl SandboxEnv {
    pub fn new() -> Self {
        let mut env = Self { records: Vec::with_capacity(10_000) };
        env.seed();
        env
    }

    pub fn seed(&mut self) {
        self.records.clear();
        const ANCHOR_MS: u64 = 1_753_315_200_000;
        const DAY_MS: u64 = 86_400_000;
        let start = ANCHOR_MS.saturating_sub(DAY_MS);
        let per: u64 = 2_500;
        let seg = DAY_MS / 4;
        for (scenario, quarter) in [
            (Scenario::Normal, 0u64),
            (Scenario::Rising, 1),
            (Scenario::Congested, 2),
            (Scenario::Spike, 3),
        ] {
            let seg_start = start + quarter * seg;
            let interval = seg / per;
            for i in 0..per {
                let fee_stroops = match scenario {
                    Scenario::Normal    => 100 + i * 400 / per,
                    Scenario::Rising    => 400 + i * 1_100 / per,
                    Scenario::Congested => 1_000 + i * 4_000 / per,
                    Scenario::Spike     => if i % 50 == 0 { 50_000 } else { 100 + (i % 50) * 4 },
                };
                self.records.push(FeeRecord {
                    timestamp_ms: seg_start + i * interval,
                    fee_stroops,
                    scenario,
                });
            }
        }
    }

    pub fn reset(&mut self) { self.seed(); }
    pub fn records(&self) -> &[FeeRecord] { &self.records }
    pub fn len(&self) -> usize { self.records.len() }
    pub fn is_empty(&self) -> bool { self.records.is_empty() }

    pub fn from_normal_fixture() -> Self {
        let records = fixtures::normal_network()
            .into_iter()
            .map(|(timestamp_ms, fee_stroops)| FeeRecord { timestamp_ms, fee_stroops, scenario: Scenario::Normal })
            .collect();
        Self { records }
    }
}

impl Default for SandboxEnv {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seed_produces_10k_records() { assert_eq!(SandboxEnv::new().len(), 10_000); }
    #[test]
    fn reset_works() {
        let mut env = SandboxEnv::new();
        env.reset();
        assert_eq!(env.len(), 10_000);
    }
}

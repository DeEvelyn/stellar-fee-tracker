//! In-memory fee database for the sandbox environment.
//!
//! [`SandboxEnv`] holds 10,000 synthetic fee records spanning a 24-hour window
//! covering Normal, Rising, Congested, and Spike network scenarios.
//! No external database dependencies are required.

use crate::sandbox::fixtures;

/// Network regime represented by a block of fee records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// Quiet network — fees 100–500 stroops.
    Normal,
    /// Fees gradually climbing above the normal band.
    Rising,
    /// Sustained high demand; fees significantly elevated.
    Congested,
    /// Short-lived burst; fees spike then recover.
    Spike,
}

/// A single fee observation stored in the sandbox database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeRecord {
    /// Milliseconds since the Unix epoch.
    pub timestamp_ms: u64,
    /// Fee amount in stroops.
    pub fee_stroops: u64,
    /// Network regime this record belongs to.
    pub regime: Regime,
}

/// In-memory fee database pre-seeded with 10,000 [`FeeRecord`]s spanning 24 hours.
///
/// Records are partitioned evenly across four [`Regime`] scenarios so tests can
/// exercise the full range of network behaviour without hitting live infrastructure.
///
/// # Example
/// ```rust
/// use stellar_devkit::sandbox::environment::SandboxEnv;
///
/// let mut env = SandboxEnv::new();
/// assert_eq!(env.records().len(), 10_000);
/// env.reset();
/// assert_eq!(env.records().len(), 10_000);
/// ```
pub struct SandboxEnv {
    records: Vec<FeeRecord>,
}

impl SandboxEnv {
    /// Creates a new [`SandboxEnv`] and seeds it with 10,000 records.
    pub fn new() -> Self {
        let mut env = Self {
            records: Vec::with_capacity(10_000),
        };
        env.seed();
        env
    }

    /// Seeds the database with 10,000 fee records spanning 24 hours.
    pub fn seed(&mut self) {
        self.records.clear();
        const ANCHOR_MS: u64 = 1_753_315_200_000;
        const DAY_MS: u64 = 86_400_000;
        let start_ms = ANCHOR_MS.saturating_sub(DAY_MS);
        let per_regime: u64 = 2_500;
        let regime_duration_ms = DAY_MS / 4;

        let scenarios: &[(Regime, u64)] = &[
            (Regime::Normal, 0),
            (Regime::Rising, 1),
            (Regime::Congested, 2),
            (Regime::Spike, 3),
        ];

        for &(regime, quarter) in scenarios {
            let seg_start = start_ms + quarter * regime_duration_ms;
            let interval_ms = regime_duration_ms / per_regime;
            for i in 0..per_regime {
                let timestamp_ms = seg_start + i * interval_ms;
                let fee_stroops = match regime {
                    Regime::Normal => 100 + (i * 400 / per_regime),
                    Regime::Rising => 400 + (i * 1_100 / per_regime),
                    Regime::Congested => 1_000 + (i * 4_000 / per_regime),
                    Regime::Spike => {
                        if i % 50 == 0 {
                            50_000
                        } else {
                            100 + (i % 50) * 4
                        }
                    }
                };
                self.records.push(FeeRecord {
                    timestamp_ms,
                    fee_stroops,
                    regime,
                });
            }
        }
    }

    /// Discards all records and re-seeds the database from scratch.
    pub fn reset(&mut self) {
        self.seed();
    }

    /// Returns a read-only slice of all fee records.
    pub fn records(&self) -> &[FeeRecord] {
        &self.records
    }

    /// Returns a mutable reference to the underlying record list.
    pub fn records_mut(&mut self) -> &mut Vec<FeeRecord> {
        &mut self.records
    }

    /// Returns only the records belonging to the given [`Regime`].
    pub fn records_for_regime(&self, regime: Regime) -> Vec<&FeeRecord> {
        self.records.iter().filter(|r| r.regime == regime).collect()
    }

    /// Total number of fee records currently in the database.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns `true` if the database contains no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Creates a [`SandboxEnv`] seeded from the normal-network fixture.
    pub fn from_normal_fixture() -> Self {
        let raw = fixtures::normal_network();
        let records = raw
            .into_iter()
            .map(|(timestamp_ms, fee_stroops)| FeeRecord {
                timestamp_ms,
                fee_stroops,
                regime: Regime::Normal,
            })
            .collect();
        Self { records }
    }
}

impl Default for SandboxEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_produces_10k_records() {
        let env = SandboxEnv::new();
        assert_eq!(env.len(), 10_000);
    }

    #[test]
    fn each_regime_has_2500_records() {
        let env = SandboxEnv::new();
        for regime in [
            Regime::Normal,
            Regime::Rising,
            Regime::Congested,
            Regime::Spike,
        ] {
            assert_eq!(env.records_for_regime(regime).len(), 2_500);
        }
    }

    #[test]
    fn reset_restores_10k_records() {
        let mut env = SandboxEnv::new();
        env.records_mut().clear();
        env.reset();
        assert_eq!(env.len(), 10_000);
    }

    #[test]
    fn from_normal_fixture_has_10k_records() {
        let env = SandboxEnv::from_normal_fixture();
        assert_eq!(env.len(), 10_000);
    }
}

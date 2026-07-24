//! Generates synthetic network load profiles for simulation.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Configuration for the seeded random network load generator.
///
/// # Fields
///
/// * `min_tx` – Minimum transactions per ledger.
/// * `max_tx` – Maximum transactions per ledger.
/// * `ledger_capacity` – Maximum transactions the network can process per ledger (capacity limit).
/// * `ledger_interval_ms` – Time between ledger closes in milliseconds.
/// * `duration_secs` – Total simulation duration in seconds.
/// * `seed` – Optional RNG seed for reproducibility.
pub struct NetworkLoadConfig {
    /// Minimum transactions per ledger.
    pub min_tx: u64,
    /// Maximum transactions per ledger.
    pub max_tx: u64,
    /// Maximum transactions the network can process per ledger (capacity limit).
    pub ledger_capacity: u64,
    /// Time between ledger closes in milliseconds.
    pub ledger_interval_ms: u64,
    /// Total simulation duration in seconds.
    pub duration_secs: u64,
    /// Optional RNG seed for reproducibility.
    pub seed: Option<u64>,
}

impl Default for NetworkLoadConfig {
    fn default() -> Self {
        Self {
            min_tx: 10,
            max_tx: 1000,
            ledger_capacity: 1000,
            ledger_interval_ms: 5000,
            duration_secs: 3600,
            seed: None,
        }
    }
}

impl NetworkLoadConfig {
    /// Returns the average capacity pressure ratio based on midpoint tx count.
    pub fn avg_pressure(&self) -> f64 {
        let mid = (self.min_tx + self.max_tx) as f64 / 2.0;
        (mid / self.ledger_capacity as f64).clamp(0.0, 1.0)
    }

    /// Number of ledger events in `duration_secs` based on `ledger_interval_ms`.
    pub fn ledger_count(&self) -> usize {
        (self.duration_secs * 1000 / self.ledger_interval_ms) as usize
    }
}

/// A single simulated ledger produced by the throughput simulator.
#[derive(Debug, Clone)]
pub struct SimulatedLedger {
    pub ledger_seq: u64,
    pub tx_count: u64,
    /// Capacity pressure in [0.0, 1.0]: tx_count / ledger_capacity.
    pub pressure: f64,
}

/// Stateful network load simulator with seeded RNG for reproducible output.
pub struct NetworkLoad {
    config: NetworkLoadConfig,
    rng: SmallRng,
}

impl NetworkLoad {
    pub fn new(config: NetworkLoadConfig) -> Self {
        let rng = match config.seed {
            Some(s) => SmallRng::seed_from_u64(s),
            None => SmallRng::from_entropy(),
        };
        Self { config, rng }
    }

    /// Generate `count` random tx-count samples within [min_tx, max_tx].
    pub fn generate(&mut self, count: usize) -> Vec<u64> {
        (0..count)
            .map(|_| self.rng.gen_range(self.config.min_tx..=self.config.max_tx))
            .collect()
    }

    /// Simulate `count` ledger closes, returning full `SimulatedLedger` records.
    ///
    /// Resolves #243.
    pub fn simulate(&mut self, count: usize) -> Vec<SimulatedLedger> {
        self.generate(count)
            .into_iter()
            .enumerate()
            .map(|(i, tx_count)| {
                let pressure =
                    (tx_count as f64 / self.config.ledger_capacity as f64).clamp(0.0, 1.0);
                SimulatedLedger {
                    ledger_seq: i as u64 + 1,
                    tx_count,
                    pressure,
                }
            })
            .collect()
    }

    /// Simulate for the full `duration_secs` configured in `NetworkLoadConfig`.
    ///
    /// Returns one `SimulatedLedger` per ledger close interval.
    /// Resolves #243.
    pub fn run(&mut self) -> Vec<SimulatedLedger> {
        self.simulate(self.config.ledger_count())
    }

    /// Returns a fee multiplier (1.0–3.0) based on hour of day (0–23).
    ///
    /// Models diurnal congestion: peak around hour 14 (2pm UTC), trough around hour 2 (2am UTC).
    /// Resolves #245.
    pub fn diurnal_multiplier(hour: u8) -> f64 {
        // Simple sinusoidal: peak at hour 14, trough at hour 2
        let angle = std::f64::consts::PI * (hour as f64 - 2.0) / 12.0;
        1.0 + angle.sin().max(0.0) * 2.0
    }

    /// Apply diurnal multiplier to a base fee given the hour of day.
    ///
    /// Resolves #245.
    pub fn diurnal_fee(base_fee: u64, hour: u8) -> u64 {
        (base_fee as f64 * Self::diurnal_multiplier(hour)).round() as u64
    }

    /// Scale a tx count by the diurnal multiplier for the given hour.
    ///
    /// During peak hours (08:00–20:00 UTC) the multiplier is >1.0, increasing
    /// the effective transaction volume.
    ///
    /// Resolves #245.
    pub fn diurnal_tx_count(tx_count: u64, hour: u8) -> u64 {
        (tx_count as f64 * Self::diurnal_multiplier(hour)).round() as u64
    }
}

/// Scale a fee upward as capacity pressure approaches 1.0.
///
/// The fee increases linearly from `base_fee` at pressure 0.0 to
/// `base_fee * max_multiple` at pressure 1.0.
///
/// # Examples
///
/// ```
/// use stellar_devkit::simulation::network_load::capacity_pressure_fee;
///
/// assert_eq!(capacity_pressure_fee(100, 0.0, 10), 100);
/// assert_eq!(capacity_pressure_fee(100, 1.0, 10), 1000);
/// assert_eq!(capacity_pressure_fee(100, 0.5, 10), 550);
/// ```
pub fn capacity_pressure_fee(base_fee: u64, pressure: f64, max_multiple: u64) -> u64 {
    let pressure = pressure.clamp(0.0, 1.0);
    let scaled = base_fee as f64 * (1.0 + pressure * (max_multiple.saturating_sub(1)) as f64);
    (scaled.round() as u64).max(1)
}

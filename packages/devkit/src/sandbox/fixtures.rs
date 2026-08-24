//! Fixture generators for the sandbox.
//!
//! Each function returns a [`Vec`] of `(timestamp_ms, fee_stroops)` tuples
//! representing a particular network condition. All fixtures are fully
//! deterministic — no RNG seed is required.

use std::f64::consts::PI;

/// Generates 10,000 fee records representing a quiet Stellar testnet session.
///
/// Fees are clustered in the **100–500 stroop** range with gentle sinusoidal
/// variation. No spike events are included. Timestamps are spaced ~8.64 seconds
/// apart spanning exactly 24 hours.
pub fn normal_network() -> Vec<(u64, u64)> {
    const ANCHOR_MS: u64 = 1_753_315_200_000;
    const COUNT: usize = 10_000;
    const DAY_MS: u64 = 86_400_000;
    let interval_ms = DAY_MS / COUNT as u64;

    (0..COUNT)
        .map(|i| {
            let timestamp_ms = ANCHOR_MS + i as u64 * interval_ms;
            let fi = i as f64;
            let wave1 = 120.0 * (2.0 * PI * fi / 137.0).sin();
            let wave2 = 80.0 * (2.0 * PI * fi / 89.0).sin();
            let raw = 200.0 + wave1 + wave2;
            let fee = (raw.round() as i64).clamp(100, 500) as u64;
            (timestamp_ms, fee)
        })
        .collect()
}

/// Generates 10,000 fee records representing a congested Stellar testnet session.
///
/// Fees are clustered in the **10,000–80,000 stroop** range with linearly
/// increasing variation. Timestamps are spaced ~8.64 seconds apart spanning
/// exactly 24 hours.
pub fn congested_network() -> Vec<(u64, u64)> {
    const ANCHOR_MS: u64 = 1_753_315_200_000;
    const COUNT: usize = 10_000;
    const DAY_MS: u64 = 86_400_000;
    let interval_ms = DAY_MS / COUNT as u64;

    (0..COUNT)
        .map(|i| {
            let timestamp_ms = ANCHOR_MS + i as u64 * interval_ms;
            let fee = 10_000 + (i as u64 * 70_000 / COUNT as u64);
            (timestamp_ms, fee)
        })
        .collect()
}

/// Generates 10,000 fee records representing a volatile Stellar network.
///
/// Most fees are very low (~10 stroops), but ~10% spike to ~80,000, producing
/// a high coefficient of variation. Timestamps are spaced ~8.64 seconds apart
/// spanning 24 hours.
pub fn volatile_network() -> Vec<(u64, u64)> {
    const ANCHOR_MS: u64 = 1_753_315_200_000;
    const COUNT: usize = 10_000;
    const DAY_MS: u64 = 86_400_000;
    let interval_ms = DAY_MS / COUNT as u64;

    (0..COUNT)
        .map(|i| {
            let timestamp_ms = ANCHOR_MS + i as u64 * interval_ms;
            let fee = if i % 10 == 0 { 80_000 } else { 10 };
            (timestamp_ms, fee)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_network_count() {
        assert_eq!(normal_network().len(), 10_000);
    }

    #[test]
    fn normal_network_fees_in_range() {
        for (_, fee) in normal_network() {
            assert!((100..=500).contains(&fee), "fee out of range: {fee}");
        }
    }

    #[test]
    fn normal_network_timestamps_ascending() {
        let records = normal_network();
        for w in records.windows(2) {
            assert!(w[0].0 < w[1].0);
        }
    }

    #[test]
    fn congested_network_count() {
        assert_eq!(congested_network().len(), 10_000);
    }

    #[test]
    fn congested_network_fees_in_range() {
        for (_, fee) in congested_network() {
            assert!((10_000..=80_000).contains(&fee), "fee out of range: {fee}");
        }
    }

    #[test]
    fn volatile_network_count() {
        assert_eq!(volatile_network().len(), 10_000);
    }
}

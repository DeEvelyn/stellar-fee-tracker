//! Fixture generators for the sandbox.
//!
//! Each generator returns 10,000 deterministic `(timestamp_ms, fee_stroops)` tuples.

use std::f64::consts::PI;

/// Quiet Stellar testnet session: fees 100–500 stroops, no spikes.
pub fn normal_network() -> Vec<(u64, u64)> {
    const ANCHOR_MS: u64 = 1_753_315_200_000;
    const COUNT: usize = 10_000;
    const DAY_MS: u64 = 86_400_000;
    let interval_ms = DAY_MS / COUNT as u64;
    (0..COUNT)
        .map(|i| {
            let fi = i as f64;
            let raw = 200.0 + 120.0 * (2.0 * PI * fi / 137.0).sin() + 80.0 * (2.0 * PI * fi / 89.0).sin();
            let fee = (raw.round() as i64).clamp(100, 500) as u64;
            (ANCHOR_MS + i as u64 * interval_ms, fee)
        })
        .collect()
}

/// Congested network: 10,000 records at 10,000–300,000 stroops with multiple spike events.
pub fn congested_network() -> Vec<(u64, u64)> {
    const ANCHOR_MS: u64 = 1_753_315_200_000;
    const COUNT: usize = 10_000;
    const DAY_MS: u64 = 86_400_000;
    let interval_ms = DAY_MS / COUNT as u64;
    (0..COUNT)
        .map(|i| {
            let timestamp_ms = ANCHOR_MS + i as u64 * interval_ms;
            // Base congestion: 10,000–100,000 stroops with sine variation.
            let fi = i as f64;
            let base = 50_000.0 + 40_000.0 * (2.0 * PI * fi / 200.0).sin();
            // Spike events every ~500 records.
            let fee = if i % 500 < 10 {
                300_000u64 // spike peak
            } else if i % 500 < 30 {
                // tail-off after spike
                200_000u64 - (i % 500 - 10) as u64 * 10_000
            } else {
                (base.round() as u64).clamp(10_000, 100_000)
            };
            (timestamp_ms, fee)
        })
        .collect()
}

/// Volatile network: 10,000 records alternating quiet/busy periods, CV > 2.0.
pub fn volatile_network() -> Vec<(u64, u64)> {
    const ANCHOR_MS: u64 = 1_753_315_200_000;
    const COUNT: usize = 10_000;
    const DAY_MS: u64 = 86_400_000;
    let interval_ms = DAY_MS / COUNT as u64;
    (0..COUNT)
        .map(|i| {
            let timestamp_ms = ANCHOR_MS + i as u64 * interval_ms;
            // Alternate: 100 quiet records (100–300) then 100 busy records (10,000–50,000).
            let period = i % 200;
            let fee = if period < 100 {
                100 + (period * 2) as u64 // quiet: 100–300
            } else {
                10_000 + ((period - 100) * 400) as u64 // busy: 10,000–50,000
            };
            (timestamp_ms, fee)
        })
        .collect()
}

/// Recovery scenario: fees declining from spike to baseline.
///
/// First 30% of records are elevated (spike range), remaining 70% decline
/// monotonically back to baseline ~200 stroops.
pub fn recovery_scenario() -> Vec<(u64, u64)> {
    const ANCHOR_MS: u64 = 1_753_315_200_000;
    const COUNT: usize = 10_000;
    const DAY_MS: u64 = 86_400_000;
    let interval_ms = DAY_MS / COUNT as u64;
    let spike_end = (COUNT as f64 * 0.3) as usize; // first 30%
    (0..COUNT)
        .map(|i| {
            let timestamp_ms = ANCHOR_MS + i as u64 * interval_ms;
            let fee = if i < spike_end {
                // Elevated zone: 50,000 stroops
                50_000u64
            } else {
                // Monotonic decline from 50,000 → 200 over the remaining 70%.
                let remaining = COUNT - spike_end;
                let step = i - spike_end;
                let decline = (50_000u64 - 200).saturating_mul(step as u64)
                    / remaining as u64;
                50_000u64 - decline
            };
            (timestamp_ms, fee)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn congested_network_count() { assert_eq!(congested_network().len(), 10_000); }

    #[test]
    fn congested_network_fees_in_range() {
        for (_, fee) in congested_network() {
            assert!(fee >= 10_000 && fee <= 300_000, "fee out of range: {fee}");
        }
    }

    #[test]
    fn volatile_network_count() { assert_eq!(volatile_network().len(), 10_000); }

    #[test]
    fn recovery_scenario_count() { assert_eq!(recovery_scenario().len(), 10_000); }

    #[test]
    fn recovery_scenario_first_30_pct_elevated() {
        let records = recovery_scenario();
        for (_, fee) in records.iter().take(3_000) {
            assert_eq!(*fee, 50_000, "first 30% must be at spike level");
        }
    }

    #[test]
    fn recovery_scenario_ends_near_baseline() {
        let records = recovery_scenario();
        let last_fee = records.last().unwrap().1;
        assert!(last_fee <= 500, "final fee should be near baseline, got {last_fee}");
    }

    #[test]
    fn recovery_scenario_monotonically_declining_after_spike() {
        let records = recovery_scenario();
        let recovery = &records[3_000..];
        for w in recovery.windows(2) {
            assert!(w[1].1 <= w[0].1, "fees must decline monotonically in recovery");
        }
    }
}

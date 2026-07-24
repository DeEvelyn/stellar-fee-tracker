use stellar_devkit::simulation::fee_model::FeeCurve;
use stellar_devkit::simulation::network_load::{NetworkLoad, NetworkLoadConfig};

/// Assert that the fee curve's 50th percentile increases monotonically as
/// capacity_usage approaches 1.0.
#[test]
fn fee_percentiles_increase_with_pressure() {
    let base_fee = 100;
    let max_fee = 10_000;
    let pressures = [0.0, 0.25, 0.5, 0.75, 1.0];
    let mut prev_p50 = 0u64;
    for &pressure in &pressures {
        let curve = FeeCurve::from_pressure(base_fee, max_fee, pressure, 1);
        let p50: u64 = curve.fee_charged.p50.parse().unwrap();
        assert!(
            p50 >= prev_p50,
            "p50 decreased from {prev_p50} to {p50} at pressure {pressure}"
        );
        prev_p50 = p50;
    }
}

/// Assert p99 also increases monotonically with pressure.
#[test]
fn fee_p99_increases_with_pressure() {
    let base_fee = 100;
    let max_fee = 10_000;
    let pressures = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let mut prev_p99 = 0u64;
    for &pressure in &pressures {
        let curve = FeeCurve::from_pressure(base_fee, max_fee, pressure, 1);
        let p99: u64 = curve.fee_charged.p99.parse().unwrap();
        assert!(
            p99 >= prev_p99,
            "p99 decreased from {prev_p99} to {p99} at pressure {pressure}"
        );
        prev_p99 = p99;
    }
}

/// Assert that every percentile in the fee curve is monotonic w.r.t. pressure.
#[test]
fn all_percentiles_monotonic_with_pressure() {
    let base_fee = 100;
    let max_fee = 10_000;
    let pressures = [0.0, 0.3, 0.6, 1.0];

    let mut prev: Vec<u64> = Vec::new();
    for &pressure in &pressures {
        let curve = FeeCurve::from_pressure(base_fee, max_fee, pressure, 1);
        let pct_values: Vec<u64> = [
            &curve.fee_charged.min,
            &curve.fee_charged.p10,
            &curve.fee_charged.p20,
            &curve.fee_charged.p30,
            &curve.fee_charged.p40,
            &curve.fee_charged.p50,
            &curve.fee_charged.p60,
            &curve.fee_charged.p70,
            &curve.fee_charged.p80,
            &curve.fee_charged.p90,
            &curve.fee_charged.p95,
            &curve.fee_charged.p99,
            &curve.fee_charged.max,
        ]
        .iter()
        .map(|s| s.parse::<u64>().unwrap())
        .collect();

        if !prev.is_empty() {
            for (i, (&cur, &prev_val)) in pct_values.iter().zip(prev.iter()).enumerate() {
                assert!(
                    cur >= prev_val,
                    "percentile at index {i} decreased from {prev_val} to {cur} when pressure moved to {pressure}"
                );
            }
        }
        prev = pct_values;
    }
}

/// Assert that avg_pressure increases when max_tx is increased.
#[test]
fn avg_pressure_increases_with_max_tx() {
    let low = NetworkLoadConfig {
        min_tx: 10,
        max_tx: 100,
        ..Default::default()
    };
    let high = NetworkLoadConfig {
        min_tx: 10,
        max_tx: 900,
        ..Default::default()
    };
    assert!(
        high.avg_pressure() > low.avg_pressure(),
        "avg_pressure should increase with max_tx"
    );
}

/// Assert that simulated ledger pressure is monotonic in tx_count.
#[test]
fn simulated_ledger_pressure_increases_with_tx_count() {
    let config = NetworkLoadConfig {
        min_tx: 1,
        max_tx: 1000,
        ledger_capacity: 1000,
        seed: Some(42),
        ..Default::default()
    };
    let mut sim = NetworkLoad::new(config);
    let ledgers = sim.simulate(100);

    for window in ledgers.windows(2) {
        // pressure must be monotonic in tx_count (same ledger capacity)
        if window[1].tx_count > window[0].tx_count {
            assert!(
                window[1].pressure >= window[0].pressure,
                "pressure decreased when tx_count increased"
            );
        }
    }
}

/// Assert every ledger pressure is within [0.0, 1.0].
#[test]
fn simulated_pressure_in_bounds() {
    let config = NetworkLoadConfig {
        seed: Some(7),
        ..Default::default()
    };
    let mut sim = NetworkLoad::new(config);
    let ledgers = sim.simulate(200);
    for l in &ledgers {
        assert!(
            (0.0..=1.0).contains(&l.pressure),
            "pressure {} out of [0,1] at ledger {}",
            l.pressure,
            l.ledger_seq
        );
    }
}

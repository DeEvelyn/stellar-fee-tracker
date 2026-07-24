//! End-to-end test: simulate fees → classify spikes → export CSV.
use stellar_devkit::{
    analysis::spike_classifier::SpikeClassifier,
    simulation::fee_model::{FeeModel, FeeModelConfig},
};

#[test]
fn e2e_simulate_classify_export() {
    let config = FeeModelConfig {
        base_fee: 100,
        spike_probability: 0.20,
        spike_multiplier: 10,
        ledger_count: 200,
        seed: Some(42),
        ..Default::default()
    };
    let records = FeeModel::run(&config);
    assert_eq!(records.len(), 200, "expected 200 ledger records");

    let fees: Vec<u64> = records.iter().map(|r| r.fee).collect();
    let outliers = SpikeClassifier::iqr_outliers(&fees);
    assert!(
        !outliers.is_empty(),
        "expected spikes at 20% probability over 200 ledgers"
    );

    // Build CSV in memory
    let mut csv = String::from("ledger,fee,timestamp\n");
    for r in &records {
        csv.push_str(&format!("{},{},{}\n", r.ledger, r.fee, r.timestamp));
    }
    assert!(csv.starts_with("ledger,fee,timestamp\n"));
    assert_eq!(csv.lines().count(), 201, "header + 200 data rows");

    for &idx in &outliers {
        assert!(idx < records.len(), "outlier index out of bounds");
    }
}

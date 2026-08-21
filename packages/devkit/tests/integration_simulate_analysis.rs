use stellar_devkit::analysis::percentile::Percentile;
use stellar_devkit::analysis::spike_classifier::{SpikeClassifier, SpikeSeverity};
use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig};

#[test]
fn test_simulate_to_analysis_pipeline() {
    let config = FeeModelConfig::default();
    let mut model = FeeModel::new(config);
    let points = model.generate(10_000, 0);

    assert_eq!(points.len(), 10_000);

    let mut values: Vec<u64> = points.iter().map(|p| p.fee).collect();
    values.sort_unstable();

    let p50 = Percentile::nearest_rank(&values, 50);
    let p95 = Percentile::nearest_rank(&values, 95);

    assert!(p50 > 0, "p50 should be positive");
    assert!(p95 >= p50, "p95 should be >= p50");

    let baseline = FeeModelConfig::default().base_fee;
    let classifier = SpikeClassifier::new(baseline);
    let spike_count = points
        .iter()
        .filter(|p| {
            matches!(
                classifier.classify(p.fee),
                SpikeSeverity::High | SpikeSeverity::Critical
            )
        })
        .count();

    println!(
        "Analyzed {} fees: p50={}, p95={}, spikes={}",
        points.len(),
        p50,
        p95,
        spike_count
    );

    assert!(
        spike_count < points.len() / 2,
        "Spike count should be less than half of total"
    );
}

use stellar_devkit::analysis::percentile::percentile_nearest;
use stellar_devkit::sandbox::fixtures::*;

#[test]
fn test_normal_fixture_has_no_spikes() {
    let fees = normal_network();
    assert!(!fees.is_empty());

    let mut values: Vec<u64> = fees.iter().map(|(_, f)| *f).collect();
    values.sort();
    let p95 = percentile_nearest(&values, 95);

    // Normal fixture should have reasonable p95 (below 50,000 stroops)
    assert!(
        p95 < 50_000,
        "Normal fixture p95 {} should be below 50,000",
        p95
    );
}

#[test]
fn test_congested_fixture_high_fees() {
    let fees = congested_network();
    assert!(!fees.is_empty());

    let mut values: Vec<u64> = fees.iter().map(|(_, f)| *f).collect();
    values.sort();
    let p95 = percentile_nearest(&values, 95);

    // Congested fixture should have p95 > 50,000
    assert!(
        p95 > 50_000,
        "Congested fixture p95 {} should be above 50,000",
        p95
    );
}

#[test]
fn test_volatile_fixture_high_cv() {
    let fees = volatile_network();
    assert!(!fees.is_empty());

    let values: Vec<u64> = fees.iter().map(|(_, f)| *f).collect();
    let mean = values.iter().sum::<u64>() as f64 / values.len() as f64;
    let variance = values
        .iter()
        .map(|v| (*v as f64 - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    let std_dev = variance.sqrt();
    let cv = std_dev / mean;

    // Volatile fixture should have CV > 2.0
    assert!(cv > 2.0, "Volatile fixture CV {} should be above 2.0", cv);
}

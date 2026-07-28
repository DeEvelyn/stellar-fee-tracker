use stellar_devkit::sandbox::environment::SandboxEnv;
use stellar_devkit::analytics::volatility::bollinger_bands;

#[test]
fn test_sandbox_to_analytics_pipeline() {
    let env = SandboxEnv::new();
    let records = env.records();

    assert!(!records.is_empty(), "Sandbox should have records");

    let values: Vec<f64> = records.iter()
        .map(|r| r.fee_stroops as f64)
        .collect();

    let bands = bollinger_bands(&values, 20);

    assert!(!bands.is_empty(), "Bands should not be empty");

    let mut within_bands = 0;
    let mut outside_bands = 0;

    for (i, &value) in values.iter().enumerate() {
        if i < bands.len() {
            if value >= bands[i].lower_band && value <= bands[i].upper_band {
                within_bands += 1;
            } else {
                outside_bands += 1;
            }
        }
    }

    println!("Bollinger Bands analysis: {} within, {} outside", within_bands, outside_bands);

    assert!(
        within_bands > outside_bands,
        "Most points should be within Bollinger Bands"
    );
}

use stellar_devkit::harness::scenarios;

#[test]
fn load_normal_scenario_parses_without_error() {
    let path = std::path::Path::new("src/harness/scenarios/normal.json");
    let result = scenarios::load_scenario(path);
    assert!(result.is_ok(), "normal.json failed to load: {:?}", result);
}

#[test]
fn load_congested_scenario_parses_without_error() {
    let path = std::path::Path::new("src/harness/scenarios/congested.json");
    let result = scenarios::load_scenario(path);
    assert!(
        result.is_ok(),
        "congested.json failed to load: {:?}",
        result
    );
}

#[test]
fn load_spike_scenario_parses_without_error() {
    let path = std::path::Path::new("src/harness/scenarios/spike.json");
    let result = scenarios::load_scenario(path);
    assert!(result.is_ok(), "spike.json failed to load: {:?}", result);
}

#[test]
fn load_recovery_scenario_parses_without_error() {
    let path = std::path::Path::new("src/harness/scenarios/recovery.json");
    let result = scenarios::load_scenario(path);
    assert!(result.is_ok(), "recovery.json failed to load: {:?}", result);
}

#[test]
fn all_bundled_scenarios_have_required_fields() {
    let names = ["normal", "congested", "spike", "recovery"];
    for name in &names {
        let path = std::path::PathBuf::from(format!("src/harness/scenarios/{}.json", name));
        let scenario = scenarios::load_scenario(&path)
            .unwrap_or_else(|e| panic!("{}.json failed validation: {}", name, e));
        assert!(
            !scenario.scenario.is_empty(),
            "{}: scenario name empty",
            name
        );
        assert!(
            !scenario.fee_stats.last_ledger.is_empty(),
            "{}: last_ledger empty",
            name
        );
        assert!(
            !scenario.fee_stats.fee_charged.p50.is_empty(),
            "{}: fee_charged.p50 empty",
            name
        );
        assert!(
            !scenario.fee_stats.max_fee.p50.is_empty(),
            "{}: max_fee.p50 empty",
            name
        );
    }
}

#[test]
fn missing_file_returns_error() {
    let path = std::path::Path::new("src/harness/scenarios/nonexistent.json");
    assert!(scenarios::load_scenario(path).is_err());
}

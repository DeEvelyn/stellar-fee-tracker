use chrono::Duration;
use stellar_devkit::sandbox::scenario::{Fixture, Scenario};

#[test]
fn test_builder_produces_correct_scenario_config() {
    let scenario = Scenario::builder()
        .fixture(Fixture::Congested)
        .duration(Duration::seconds(7200))
        .build();

    assert_eq!(scenario.duration, Duration::seconds(7200));
}

#[test]
fn test_builder_default_values() {
    let scenario = Scenario::builder().build();

    assert!(scenario.spike_at.is_none(), "Default should have no spike");
}

#[test]
fn test_spike_injection_at_correct_offset() {
    let scenario = Scenario::builder()
        .fixture(Fixture::Normal)
        .duration(Duration::seconds(3600))
        .inject_spike_at(1800, 50_000)
        .build();

    assert!(scenario.spike_at.is_some(), "Should have spike configured");
    let (offset, fee) = scenario.spike_at.unwrap();
    assert_eq!(offset, 1800);
    assert_eq!(fee, 50_000);
}

#[test]
fn test_builder_chaining() {
    let scenario = Scenario::builder()
        .fixture(Fixture::Spike)
        .duration(Duration::seconds(1800))
        .inject_spike_at(900, 100_000)
        .build();

    assert_eq!(scenario.duration, Duration::seconds(1800));
    assert_eq!(scenario.spike_at, Some((900, 100_000)));
}

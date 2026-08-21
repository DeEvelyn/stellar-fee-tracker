use std::time::Instant;
use stellar_devkit::sandbox::runner::*;

#[test]
fn test_runner_executes_closure() {
    let mut executed = false;
    run(|_env| {
        executed = true;
    });
    assert!(executed, "Runner should execute the closure");
}

#[test]
fn test_runner_provides_non_empty_env() {
    run(|env| {
        let records = env.records();
        assert!(!records.is_empty(), "Sandbox env should have records");
    });
}

#[test]
fn test_result_collector_captures_duration() {
    let start = Instant::now();
    let mut collector = ResultCollector::new();

    // Simulate some work
    std::thread::sleep(std::time::Duration::from_millis(10));

    collector.record_duration(start.elapsed());
    assert!(
        collector.duration().as_millis() >= 10,
        "Should capture duration"
    );
}

#[test]
fn test_result_collector_captures_record_count() {
    let mut collector = ResultCollector::new();
    collector.set_records_processed(100);
    assert_eq!(collector.records_processed(), 100);
}

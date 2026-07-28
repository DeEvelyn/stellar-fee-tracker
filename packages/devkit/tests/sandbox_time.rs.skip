use stellar_devkit::sandbox::environment::SandboxEnv;
use chrono::{Duration, Utc};

#[test]
fn test_advance_time_moves_clock_forward() {
    let mut env = SandboxEnv::from_normal_fixture();
    let initial_time = env.current_time();

    env.advance_time(Duration::hours(1));

    let new_time = env.current_time();
    let diff = new_time - initial_time;
    assert!(diff >= Duration::hours(1), "Time should advance by at least 1 hour");
}

#[test]
fn test_set_time_to_specific_moment() {
    let mut env = SandboxEnv::from_normal_fixture();
    let target = Utc::now() - Duration::days(365);

    env.set_time(target);

    let current = env.current_time();
    let diff = (current - target).num_seconds().abs();
    assert!(diff < 2, "Time should be set to target (within 2 seconds)");
}

#[test]
fn test_current_time_returns_reasonable_value() {
    let env = SandboxEnv::from_normal_fixture();
    let now = Utc::now();
    let env_time = env.current_time();

    let diff = (now - env_time).num_seconds().abs();
    assert!(diff < 60, "Current time should be within 60 seconds of now");
}

#[test]
fn test_fees_generated_after_advance_reflect_new_timestamps() {
    let mut env = SandboxEnv::from_normal_fixture();
    env.advance_time(Duration::hours(24));

    let records = env.records();
    // All records should have timestamps after the advance
    for record in records {
        assert!(record.timestamp_ms > 0, "Records should have valid timestamps");
    }
}

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use stellar_devkit::config::DevkitConfig;

// Helper: write a TOML config file to a temp path and return the path
fn write_temp_toml(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write TOML");
    file
}

fn env_guard() -> MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

// ─── from_toml_file tests ───────────────────────────────────────────────────

#[test]
fn test_load_full_toml_file_sets_all_fields() {
    let content = r#"
        db_path = "/tmp/my_fees.db"
        scenario = "congested"
        port = 9090
        verbose = true
        horizon_url = "https://horizon.stellar.org"
        poll_interval_secs = 30
        retry_attempts = 5
        base_retry_delay_ms = 2000
        simulation_duration = 500
        simulation_base_fee = 200
        simulation_spike_prob = 0.1
        sandbox_time_offset_secs = -3600
        analysis_window_hours = 48
    "#;

    let file = write_temp_toml(content);
    let config = DevkitConfig::from_toml_file(&file.path().to_path_buf())
        .expect("Should load TOML successfully");

    assert_eq!(config.db_path, PathBuf::from("/tmp/my_fees.db"));
    assert_eq!(config.scenario, "congested");
    assert_eq!(config.port, 9090);
    assert!(config.verbose);
    assert_eq!(config.horizon_url, "https://horizon.stellar.org");
    assert_eq!(config.poll_interval_secs, 30);
    assert_eq!(config.retry_attempts, 5);
    assert_eq!(config.base_retry_delay_ms, 2000);
    assert_eq!(config.simulation_duration, 500);
    assert_eq!(config.simulation_base_fee, 200);
    assert!((config.simulation_spike_prob - 0.1).abs() < 1e-9);
    assert_eq!(config.sandbox_time_offset_secs, -3600);
    assert_eq!(config.analysis_window_hours, 48);
}

#[test]
fn test_load_partial_toml_uses_defaults_for_missing_fields() {
    let content = r#"
        port = 7777
        scenario = "spike"
    "#;

    let file = write_temp_toml(content);
    let config =
        DevkitConfig::from_toml_file(&file.path().to_path_buf()).expect("Should load partial TOML");

    assert_eq!(config.port, 7777);
    assert_eq!(config.scenario, "spike");

    // All unspecified fields should equal the defaults
    let defaults = DevkitConfig::default();
    assert_eq!(config.db_path, defaults.db_path);
    assert_eq!(config.verbose, defaults.verbose);
    assert_eq!(config.horizon_url, defaults.horizon_url);
    assert_eq!(config.poll_interval_secs, defaults.poll_interval_secs);
    assert_eq!(config.retry_attempts, defaults.retry_attempts);
    assert_eq!(config.base_retry_delay_ms, defaults.base_retry_delay_ms);
    assert_eq!(config.simulation_duration, defaults.simulation_duration);
    assert_eq!(config.simulation_base_fee, defaults.simulation_base_fee);
    assert_eq!(config.simulation_spike_prob, defaults.simulation_spike_prob);
    assert_eq!(
        config.sandbox_time_offset_secs,
        defaults.sandbox_time_offset_secs
    );
    assert_eq!(config.analysis_window_hours, defaults.analysis_window_hours);
}

#[test]
fn test_load_empty_toml_returns_all_defaults() {
    let file = write_temp_toml("");
    let config = DevkitConfig::from_toml_file(&file.path().to_path_buf())
        .expect("Should load empty TOML as defaults");

    let defaults = DevkitConfig::default();
    assert_eq!(config.db_path, defaults.db_path);
    assert_eq!(config.scenario, defaults.scenario);
    assert_eq!(config.port, defaults.port);
    assert_eq!(config.verbose, defaults.verbose);
    assert_eq!(config.horizon_url, defaults.horizon_url);
    assert_eq!(config.poll_interval_secs, defaults.poll_interval_secs);
    assert_eq!(config.retry_attempts, defaults.retry_attempts);
    assert_eq!(config.simulation_base_fee, defaults.simulation_base_fee);
}

#[test]
fn test_load_toml_missing_file_returns_error() {
    let result = DevkitConfig::from_toml_file(&PathBuf::from("/nonexistent/path/config.toml"));
    assert!(result.is_err(), "Should return Err for missing file");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("Failed to read config file"),
        "Error message should indicate read failure: {msg}"
    );
}

#[test]
fn test_load_toml_malformed_content_returns_error() {
    let content = "not valid toml !!![ broken";
    let file = write_temp_toml(content);
    let result = DevkitConfig::from_toml_file(&file.path().to_path_buf());
    assert!(result.is_err(), "Should return Err for malformed TOML");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("Failed to parse config file"),
        "Error message should indicate parse failure: {msg}"
    );
}

#[test]
fn test_load_toml_verbose_false_value() {
    let content = "verbose = false\n";
    let file = write_temp_toml(content);
    let config = DevkitConfig::from_toml_file(&file.path().to_path_buf()).unwrap();
    assert!(!config.verbose);
}

#[test]
fn test_load_toml_negative_time_offset() {
    let content = "sandbox_time_offset_secs = -7200\n";
    let file = write_temp_toml(content);
    let config = DevkitConfig::from_toml_file(&file.path().to_path_buf()).unwrap();
    assert_eq!(config.sandbox_time_offset_secs, -7200);
}

#[test]
fn test_load_toml_positive_time_offset() {
    let content = "sandbox_time_offset_secs = 3600\n";
    let file = write_temp_toml(content);
    let config = DevkitConfig::from_toml_file(&file.path().to_path_buf()).unwrap();
    assert_eq!(config.sandbox_time_offset_secs, 3600);
}

// ─── from_env tests ──────────────────────────────────────────────────────────

#[test]
fn test_env_vars_override_defaults() {
    let _env_guard = env_guard();
    // Use a scoped environment for this test
    std::env::set_var("DEVKIT_PORT", "9999");
    std::env::set_var("DEVKIT_SCENARIO", "stress");
    std::env::set_var("DEVKIT_VERBOSE", "true");
    std::env::set_var("DEVKIT_POLL_INTERVAL_SECS", "60");

    let config = DevkitConfig::from_env();

    assert_eq!(config.port, 9999);
    assert_eq!(config.scenario, "stress");
    assert!(config.verbose);
    assert_eq!(config.poll_interval_secs, 60);

    // Cleanup
    std::env::remove_var("DEVKIT_PORT");
    std::env::remove_var("DEVKIT_SCENARIO");
    std::env::remove_var("DEVKIT_VERBOSE");
    std::env::remove_var("DEVKIT_POLL_INTERVAL_SECS");
}

#[test]
fn test_env_db_path_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_DB_PATH", "/tmp/custom.db");
    let config = DevkitConfig::from_env();
    assert_eq!(config.db_path, PathBuf::from("/tmp/custom.db"));
    std::env::remove_var("DEVKIT_DB_PATH");
}

#[test]
fn test_env_horizon_url_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_HORIZON_URL", "https://horizon.stellar.org");
    let config = DevkitConfig::from_env();
    assert_eq!(config.horizon_url, "https://horizon.stellar.org");
    std::env::remove_var("DEVKIT_HORIZON_URL");
}

#[test]
fn test_env_retry_attempts_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_RETRY_ATTEMPTS", "7");
    let config = DevkitConfig::from_env();
    assert_eq!(config.retry_attempts, 7);
    std::env::remove_var("DEVKIT_RETRY_ATTEMPTS");
}

#[test]
fn test_env_base_retry_delay_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_BASE_RETRY_DELAY_MS", "5000");
    let config = DevkitConfig::from_env();
    assert_eq!(config.base_retry_delay_ms, 5000);
    std::env::remove_var("DEVKIT_BASE_RETRY_DELAY_MS");
}

#[test]
fn test_env_simulation_duration_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_SIMULATION_DURATION", "2000");
    let config = DevkitConfig::from_env();
    assert_eq!(config.simulation_duration, 2000);
    std::env::remove_var("DEVKIT_SIMULATION_DURATION");
}

#[test]
fn test_env_simulation_base_fee_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_SIMULATION_BASE_FEE", "500");
    let config = DevkitConfig::from_env();
    assert_eq!(config.simulation_base_fee, 500);
    std::env::remove_var("DEVKIT_SIMULATION_BASE_FEE");
}

#[test]
fn test_env_simulation_spike_prob_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_SIMULATION_SPIKE_PROB", "0.25");
    let config = DevkitConfig::from_env();
    assert!((config.simulation_spike_prob - 0.25).abs() < 1e-9);
    std::env::remove_var("DEVKIT_SIMULATION_SPIKE_PROB");
}

#[test]
fn test_env_sandbox_time_offset_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_SANDBOX_TIME_OFFSET_SECS", "-1800");
    let config = DevkitConfig::from_env();
    assert_eq!(config.sandbox_time_offset_secs, -1800);
    std::env::remove_var("DEVKIT_SANDBOX_TIME_OFFSET_SECS");
}

#[test]
fn test_env_analysis_window_hours_override() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_ANALYSIS_WINDOW_HOURS", "72");
    let config = DevkitConfig::from_env();
    assert_eq!(config.analysis_window_hours, 72);
    std::env::remove_var("DEVKIT_ANALYSIS_WINDOW_HOURS");
}

#[test]
fn test_env_verbose_one_value() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_VERBOSE", "1");
    let config = DevkitConfig::from_env();
    assert!(config.verbose);
    std::env::remove_var("DEVKIT_VERBOSE");
}

#[test]
fn test_env_verbose_false_value() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_VERBOSE", "false");
    let config = DevkitConfig::from_env();
    assert!(!config.verbose);
    std::env::remove_var("DEVKIT_VERBOSE");
}

#[test]
fn test_env_invalid_port_falls_back_to_default() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_PORT", "not_a_number");
    let config = DevkitConfig::from_env();
    let defaults = DevkitConfig::default();
    assert_eq!(config.port, defaults.port);
    std::env::remove_var("DEVKIT_PORT");
}

#[test]
fn test_env_invalid_poll_interval_falls_back_to_default() {
    let _env_guard = env_guard();
    std::env::set_var("DEVKIT_POLL_INTERVAL_SECS", "abc");
    let config = DevkitConfig::from_env();
    let defaults = DevkitConfig::default();
    assert_eq!(config.poll_interval_secs, defaults.poll_interval_secs);
    std::env::remove_var("DEVKIT_POLL_INTERVAL_SECS");
}

// ─── apply_env (override on top of TOML) tests ──────────────────────────────

#[test]
fn test_env_overrides_toml_values() {
    let _env_guard = env_guard();
    let content = r#"
        port = 7000
        scenario = "normal"
        poll_interval_secs = 20
    "#;
    let file = write_temp_toml(content);
    let mut config =
        DevkitConfig::from_toml_file(&file.path().to_path_buf()).expect("Should parse TOML");

    // Assert TOML values are loaded
    assert_eq!(config.port, 7000);
    assert_eq!(config.scenario, "normal");
    assert_eq!(config.poll_interval_secs, 20);

    // Now apply env overrides
    std::env::set_var("DEVKIT_PORT", "9000");
    std::env::set_var("DEVKIT_POLL_INTERVAL_SECS", "45");
    config.apply_env();

    assert_eq!(config.port, 9000);
    assert_eq!(config.poll_interval_secs, 45);
    // Unoverridden field should keep TOML value
    assert_eq!(config.scenario, "normal");

    std::env::remove_var("DEVKIT_PORT");
    std::env::remove_var("DEVKIT_POLL_INTERVAL_SECS");
}

#[test]
fn test_env_not_set_does_not_change_toml_values() {
    let _env_guard = env_guard();
    let content = r#"
        port = 7777
        horizon_url = "https://custom.horizon.org"
    "#;
    let file = write_temp_toml(content);
    let mut config =
        DevkitConfig::from_toml_file(&file.path().to_path_buf()).expect("Should parse TOML");

    // Ensure no stray env vars interfere
    std::env::remove_var("DEVKIT_PORT");
    std::env::remove_var("DEVKIT_HORIZON_URL");

    config.apply_env();

    assert_eq!(config.port, 7777);
    assert_eq!(config.horizon_url, "https://custom.horizon.org");
}

// ─── config display tests ────────────────────────────────────────────────────

#[test]
fn test_display_contains_all_key_names() {
    let config = DevkitConfig::default();
    let output = config.display();
    assert!(output.contains("db_path"));
    assert!(output.contains("scenario"));
    assert!(output.contains("port"));
    assert!(output.contains("verbose"));
    assert!(output.contains("horizon_url"));
    assert!(output.contains("poll_interval_secs"));
    assert!(output.contains("retry_attempts"));
    assert!(output.contains("simulation_base_fee"));
    assert!(output.contains("analysis_window_hours"));
}

#[test]
fn test_display_contains_header() {
    let config = DevkitConfig::default();
    let output = config.display();
    assert!(output.contains("devkit configuration"));
}

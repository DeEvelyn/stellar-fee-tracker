use std::path::PathBuf;

use stellar_devkit::config::{ConfigValidator, DevkitConfig, ValidationSeverity};

/// Helper to create a valid baseline config
fn valid_config() -> DevkitConfig {
    DevkitConfig {
        db_path: PathBuf::from("test.db"),
        scenario: "normal".to_string(),
        port: 8090,
        verbose: false,
        horizon_url: "https://horizon-testnet.stellar.org".to_string(),
        poll_interval_secs: 10,
        retry_attempts: 3,
        base_retry_delay_ms: 1000,
        simulation_duration: 1000,
        simulation_base_fee: 100,
        simulation_spike_prob: 0.05,
        sandbox_time_offset_secs: 0,
        analysis_window_hours: 24,
        overrides: Default::default(),
    }
}

#[test]
fn test_valid_config_passes() {
    let config = valid_config();
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid(), "Valid config should pass validation");
    assert_eq!(result.errors().len(), 0);
}

#[test]
fn test_default_config_is_valid() {
    let config = DevkitConfig::default();
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid(), "Default config should be valid");
}

// Port validation tests
#[test]
fn test_zero_port_produces_error() {
    let mut config = valid_config();
    config.port = 0;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, "port");
    assert!(errors[0].message.contains("greater than 0"));
}

#[test]
fn test_privileged_port_produces_warning() {
    let mut config = valid_config();
    config.port = 80;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid(), "Privileged port should be valid with warning");
    let warnings = result.warnings();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].field, "port");
    assert!(warnings[0].message.contains("privileged"));
}

#[test]
fn test_port_1024_produces_no_warning() {
    let mut config = valid_config();
    config.port = 1024;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    assert_eq!(result.warnings().len(), 0);
}

// Database path validation tests
#[test]
fn test_empty_db_path_produces_error() {
    let mut config = valid_config();
    config.db_path = PathBuf::from("");
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "db_path"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("Database path is empty")));
}

// Horizon URL validation tests
#[test]
fn test_empty_horizon_url_produces_error() {
    let mut config = valid_config();
    config.horizon_url = String::new();
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "horizon_url"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("Horizon URL is empty")));
}

#[test]
fn test_invalid_horizon_url_scheme_produces_error() {
    let mut config = valid_config();
    config.horizon_url = "ftp://invalid.com".to_string();
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "horizon_url"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("does not start with http")));
}

#[test]
fn test_horizon_url_with_spaces_produces_error() {
    let mut config = valid_config();
    config.horizon_url = "https://invalid url.com".to_string();
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "horizon_url"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("contains spaces")));
}

#[test]
fn test_valid_http_horizon_url_passes() {
    let mut config = valid_config();
    config.horizon_url = "http://localhost:8000".to_string();
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
}

#[test]
fn test_valid_https_horizon_url_passes() {
    let mut config = valid_config();
    config.horizon_url = "https://horizon.stellar.org".to_string();
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
}

// Poll interval validation tests
#[test]
fn test_zero_poll_interval_produces_error() {
    let mut config = valid_config();
    config.poll_interval_secs = 0;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "poll_interval_secs"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must be greater than 0")));
}

#[test]
fn test_aggressive_poll_interval_produces_warning() {
    let mut config = valid_config();
    config.poll_interval_secs = 2;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    let warnings = result.warnings();
    assert!(warnings.iter().any(|w| w.field == "poll_interval_secs"));
    assert!(warnings.iter().any(|w| w.message.contains("aggressive")));
}

#[test]
fn test_poll_interval_5_produces_no_warning() {
    let mut config = valid_config();
    config.poll_interval_secs = 5;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    assert_eq!(result.warnings().len(), 0);
}

// Retry configuration validation tests
#[test]
fn test_zero_retry_attempts_produces_error() {
    let mut config = valid_config();
    config.retry_attempts = 0;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "retry_attempts"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must be greater than 0")));
}

#[test]
fn test_high_retry_attempts_produces_warning() {
    let mut config = valid_config();
    config.retry_attempts = 15;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    let warnings = result.warnings();
    assert!(warnings.iter().any(|w| w.field == "retry_attempts"));
    assert!(warnings.iter().any(|w| w.message.contains("very high")));
}

#[test]
fn test_retry_attempts_10_produces_no_warning() {
    let mut config = valid_config();
    config.retry_attempts = 10;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    // Should have no warnings for retry_attempts
    assert!(!result
        .warnings()
        .iter()
        .any(|w| w.field == "retry_attempts"));
}

#[test]
fn test_zero_base_retry_delay_produces_error() {
    let mut config = valid_config();
    config.base_retry_delay_ms = 0;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "base_retry_delay_ms"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must be greater than 0")));
}

#[test]
fn test_high_base_retry_delay_produces_warning() {
    let mut config = valid_config();
    config.base_retry_delay_ms = 35_000;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    let warnings = result.warnings();
    assert!(warnings.iter().any(|w| w.field == "base_retry_delay_ms"));
    assert!(warnings.iter().any(|w| w.message.contains("very high")));
}

// Simulation configuration validation tests
#[test]
fn test_zero_simulation_duration_produces_error() {
    let mut config = valid_config();
    config.simulation_duration = 0;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "simulation_duration"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must be greater than 0")));
}

#[test]
fn test_zero_simulation_base_fee_produces_error() {
    let mut config = valid_config();
    config.simulation_base_fee = 0;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "simulation_base_fee"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must be greater than 0")));
}

#[test]
fn test_spike_prob_below_zero_produces_error() {
    let mut config = valid_config();
    config.simulation_spike_prob = -0.1;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "simulation_spike_prob"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("out of range [0.0, 1.0]")));
}

#[test]
fn test_spike_prob_above_one_produces_error() {
    let mut config = valid_config();
    config.simulation_spike_prob = 1.5;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "simulation_spike_prob"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("out of range [0.0, 1.0]")));
}

#[test]
fn test_spike_prob_zero_is_valid() {
    let mut config = valid_config();
    config.simulation_spike_prob = 0.0;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
}

#[test]
fn test_spike_prob_one_is_valid() {
    let mut config = valid_config();
    config.simulation_spike_prob = 1.0;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
}

// Analysis configuration validation tests
#[test]
fn test_zero_analysis_window_produces_error() {
    let mut config = valid_config();
    config.analysis_window_hours = 0;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.iter().any(|e| e.field == "analysis_window_hours"));
    assert!(errors
        .iter()
        .any(|e| e.message.contains("must be greater than 0")));
}

#[test]
fn test_large_analysis_window_produces_warning() {
    let mut config = valid_config();
    config.analysis_window_hours = 200; // More than 7 days
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    let warnings = result.warnings();
    assert!(warnings
        .iter()
        .any(|w| w.field == "analysis_window_hours"));
    assert!(warnings.iter().any(|w| w.message.contains("exceeds 7 days")));
}

#[test]
fn test_analysis_window_168_produces_no_warning() {
    let mut config = valid_config();
    config.analysis_window_hours = 168; // Exactly 7 days
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    // Should have no warnings for analysis_window_hours
    assert!(!result
        .warnings()
        .iter()
        .any(|w| w.field == "analysis_window_hours"));
}

// Scenario validation tests
#[test]
fn test_valid_scenario_normal_passes() {
    let mut config = valid_config();
    config.scenario = "normal".to_string();
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
}

#[test]
fn test_valid_scenario_congested_passes() {
    let mut config = valid_config();
    config.scenario = "congested".to_string();
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
}

#[test]
fn test_valid_scenario_spike_passes() {
    let mut config = valid_config();
    config.scenario = "spike".to_string();
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
}

#[test]
fn test_unknown_scenario_produces_warning() {
    let mut config = valid_config();
    config.scenario = "unknown_scenario".to_string();
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    let warnings = result.warnings();
    assert!(warnings.iter().any(|w| w.field == "scenario"));
    assert!(warnings
        .iter()
        .any(|w| w.message.contains("Unknown scenario")));
}

// Multiple errors test
#[test]
fn test_multiple_errors_all_reported() {
    let mut config = valid_config();
    config.port = 0;
    config.horizon_url = String::new();
    config.poll_interval_secs = 0;
    let result = ConfigValidator::validate(&config);
    assert!(!result.is_valid());
    let errors = result.errors();
    assert!(errors.len() >= 3);
    assert!(errors.iter().any(|e| e.field == "port"));
    assert!(errors.iter().any(|e| e.field == "horizon_url"));
    assert!(errors.iter().any(|e| e.field == "poll_interval_secs"));
}

// Validation result tests
#[test]
fn test_validation_result_has_issues() {
    let mut config = valid_config();
    config.port = 0;
    let result = ConfigValidator::validate(&config);
    assert!(result.has_issues());
}

#[test]
fn test_validation_result_display_contains_errors() {
    let mut config = valid_config();
    config.port = 0;
    config.horizon_url = "".to_string();
    let result = ConfigValidator::validate(&config);
    let display = result.display();
    assert!(display.contains("ERROR"));
    assert!(display.contains("port"));
    assert!(display.contains("horizon_url"));
    assert!(display.contains("2 error(s)"));
}

#[test]
fn test_validation_result_display_contains_warnings() {
    let mut config = valid_config();
    config.port = 80;
    config.poll_interval_secs = 2;
    let result = ConfigValidator::validate(&config);
    let display = result.display();
    assert!(display.contains("WARN"));
    assert!(display.contains("0 error(s), 2 warning(s)"));
}

#[test]
fn test_validation_result_display_no_issues() {
    let config = valid_config();
    let result = ConfigValidator::validate(&config);
    let display = result.display();
    assert!(display.contains("All checks passed"));
}

// Edge case tests
#[test]
fn test_config_with_only_warnings_is_valid() {
    let mut config = valid_config();
    config.port = 80; // Warning, not error
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    assert!(result.has_issues());
    assert_eq!(result.errors().len(), 0);
    assert!(result.warnings().len() > 0);
}

#[test]
fn test_boundary_values_are_valid() {
    let mut config = valid_config();
    config.simulation_spike_prob = 0.0;
    config.poll_interval_secs = 5;
    config.retry_attempts = 10;
    config.analysis_window_hours = 168;
    let result = ConfigValidator::validate(&config);
    assert!(result.is_valid());
    assert_eq!(result.errors().len(), 0);
}

use super::DevkitConfig;

/// Severity level for a validation issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

/// A single validation issue found during config validation.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

/// Result of validating a DevkitConfig.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Returns true if there are no error-level issues.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|i| i.severity == ValidationSeverity::Error)
    }

    /// Returns true if there are any issues (errors or warnings).
    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }

    /// Return only error-level issues.
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect()
    }

    /// Return only warning-level issues.
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Warning)
            .collect()
    }

    /// Display the validation result as a formatted report.
    pub fn display(&self) -> String {
        let mut out = String::new();
        out.push_str("Configuration Validation\n");
        out.push_str("========================\n");

        if self.issues.is_empty() {
            out.push_str("All checks passed.\n");
            return out;
        }

        for issue in &self.issues {
            let prefix = match issue.severity {
                ValidationSeverity::Error => "ERROR",
                ValidationSeverity::Warning => "WARN ",
            };
            out.push_str(&format!(
                "[{}] {}: {}\n",
                prefix, issue.field, issue.message
            ));
        }

        out.push_str(&format!(
            "\n{} error(s), {} warning(s)\n",
            self.errors().len(),
            self.warnings().len()
        ));
        out
    }
}

/// Configuration validator for DevkitConfig.
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate the given configuration and return all issues found.
    pub fn validate(config: &DevkitConfig) -> ValidationResult {
        let mut issues = Vec::new();

        Self::validate_port(config, &mut issues);
        Self::validate_db_path(config, &mut issues);
        Self::validate_horizon_url(config, &mut issues);
        Self::validate_poll_interval(config, &mut issues);
        Self::validate_retry_config(config, &mut issues);
        Self::validate_simulation_config(config, &mut issues);
        Self::validate_analysis_config(config, &mut issues);
        Self::validate_scenario(config, &mut issues);

        ValidationResult { issues }
    }

    fn validate_port(config: &DevkitConfig, issues: &mut Vec<ValidationIssue>) {
        if config.port == 0 {
            issues.push(ValidationIssue {
                field: "port".to_string(),
                message: "Port must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        } else if config.port < 1024 {
            issues.push(ValidationIssue {
                field: "port".to_string(),
                message: format!(
                    "Port {} is in the privileged range (1-1024). Consider using a port >= 1024.",
                    config.port
                ),
                severity: ValidationSeverity::Warning,
            });
        }
    }

    fn validate_db_path(config: &DevkitConfig, issues: &mut Vec<ValidationIssue>) {
        let path = &config.db_path;
        if path.to_string_lossy().is_empty() {
            issues.push(ValidationIssue {
                field: "db_path".to_string(),
                message: "Database path is empty".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                issues.push(ValidationIssue {
                    field: "db_path".to_string(),
                    message: format!("Parent directory does not exist: {}", parent.display()),
                    severity: ValidationSeverity::Warning,
                });
            }
        }
    }

    fn validate_horizon_url(config: &DevkitConfig, issues: &mut Vec<ValidationIssue>) {
        let url = &config.horizon_url;
        if url.is_empty() {
            issues.push(ValidationIssue {
                field: "horizon_url".to_string(),
                message: "Horizon URL is empty".to_string(),
                severity: ValidationSeverity::Error,
            });
            return;
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            issues.push(ValidationIssue {
                field: "horizon_url".to_string(),
                message: format!(
                    "Horizon URL '{}' does not start with http:// or https://",
                    url
                ),
                severity: ValidationSeverity::Error,
            });
        }

        if url.contains(' ') {
            issues.push(ValidationIssue {
                field: "horizon_url".to_string(),
                message: "Horizon URL contains spaces".to_string(),
                severity: ValidationSeverity::Error,
            });
        }
    }

    fn validate_poll_interval(config: &DevkitConfig, issues: &mut Vec<ValidationIssue>) {
        if config.poll_interval_secs == 0 {
            issues.push(ValidationIssue {
                field: "poll_interval_secs".to_string(),
                message: "Poll interval must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        } else if config.poll_interval_secs < 5 {
            issues.push(ValidationIssue {
                field: "poll_interval_secs".to_string(),
                message: format!(
                    "Poll interval of {}s is very aggressive. Consider >= 5s to avoid rate limiting.",
                    config.poll_interval_secs
                ),
                severity: ValidationSeverity::Warning,
            });
        }
    }

    fn validate_retry_config(config: &DevkitConfig, issues: &mut Vec<ValidationIssue>) {
        if config.retry_attempts == 0 {
            issues.push(ValidationIssue {
                field: "retry_attempts".to_string(),
                message: "Retry attempts must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        } else if config.retry_attempts > 10 {
            issues.push(ValidationIssue {
                field: "retry_attempts".to_string(),
                message: format!(
                    "Retry attempts of {} is very high. Consider <= 10.",
                    config.retry_attempts
                ),
                severity: ValidationSeverity::Warning,
            });
        }

        if config.base_retry_delay_ms == 0 {
            issues.push(ValidationIssue {
                field: "base_retry_delay_ms".to_string(),
                message: "Base retry delay must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        } else if config.base_retry_delay_ms > 30_000 {
            issues.push(ValidationIssue {
                field: "base_retry_delay_ms".to_string(),
                message: format!(
                    "Base retry delay of {}ms is very high. Consider <= 30000ms.",
                    config.base_retry_delay_ms
                ),
                severity: ValidationSeverity::Warning,
            });
        }
    }

    fn validate_simulation_config(config: &DevkitConfig, issues: &mut Vec<ValidationIssue>) {
        if config.simulation_duration == 0 {
            issues.push(ValidationIssue {
                field: "simulation_duration".to_string(),
                message: "Simulation duration must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        if config.simulation_base_fee == 0 {
            issues.push(ValidationIssue {
                field: "simulation_base_fee".to_string(),
                message: "Simulation base fee must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        }

        if config.simulation_spike_prob < 0.0 || config.simulation_spike_prob > 1.0 {
            issues.push(ValidationIssue {
                field: "simulation_spike_prob".to_string(),
                message: format!(
                    "Spike probability {} is out of range [0.0, 1.0]",
                    config.simulation_spike_prob
                ),
                severity: ValidationSeverity::Error,
            });
        }
    }

    fn validate_analysis_config(config: &DevkitConfig, issues: &mut Vec<ValidationIssue>) {
        if config.analysis_window_hours == 0 {
            issues.push(ValidationIssue {
                field: "analysis_window_hours".to_string(),
                message: "Analysis window must be greater than 0".to_string(),
                severity: ValidationSeverity::Error,
            });
        } else if config.analysis_window_hours > 168 {
            issues.push(ValidationIssue {
                field: "analysis_window_hours".to_string(),
                message: format!(
                    "Analysis window of {} hours exceeds 7 days (168h). Data freshness may be poor.",
                    config.analysis_window_hours
                ),
                severity: ValidationSeverity::Warning,
            });
        }
    }

    fn validate_scenario(config: &DevkitConfig, issues: &mut Vec<ValidationIssue>) {
        let valid_scenarios = ["normal", "congested", "spike", "calm", "stress"];
        if !valid_scenarios.contains(&config.scenario.as_str()) {
            issues.push(ValidationIssue {
                field: "scenario".to_string(),
                message: format!(
                    "Unknown scenario '{}'. Valid scenarios: {}",
                    config.scenario,
                    valid_scenarios.join(", ")
                ),
                severity: ValidationSeverity::Warning,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> DevkitConfig {
        DevkitConfig::default()
    }

    #[test]
    fn valid_default_config_passes() {
        let config = default_config();
        let result = ConfigValidator::validate(&config);
        assert!(result.is_valid(), "Default config should be valid");
    }

    #[test]
    fn zero_port_is_error() {
        let mut config = default_config();
        config.port = 0;
        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid());
        assert!(result.errors().iter().any(|i| i.field == "port"));
    }

    #[test]
    fn privileged_port_is_warning() {
        let mut config = default_config();
        config.port = 80;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_valid());
        assert!(result.warnings().iter().any(|i| i.field == "port"));
    }

    #[test]
    fn empty_horizon_url_is_error() {
        let mut config = default_config();
        config.horizon_url = String::new();
        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid());
        assert!(result.errors().iter().any(|i| i.field == "horizon_url"));
    }

    #[test]
    fn invalid_horizon_url_is_error() {
        let mut config = default_config();
        config.horizon_url = "ftp://invalid".to_string();
        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid());
        assert!(result.errors().iter().any(|i| i.field == "horizon_url"));
    }

    #[test]
    fn zero_poll_interval_is_error() {
        let mut config = default_config();
        config.poll_interval_secs = 0;
        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid());
        assert!(result
            .errors()
            .iter()
            .any(|i| i.field == "poll_interval_secs"));
    }

    #[test]
    fn aggressive_poll_interval_is_warning() {
        let mut config = default_config();
        config.poll_interval_secs = 2;
        let result = ConfigValidator::validate(&config);
        assert!(result.is_valid());
        assert!(result
            .warnings()
            .iter()
            .any(|i| i.field == "poll_interval_secs"));
    }

    #[test]
    fn invalid_spike_prob_is_error() {
        let mut config = default_config();
        config.simulation_spike_prob = 1.5;
        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid());
        assert!(result
            .errors()
            .iter()
            .any(|i| i.field == "simulation_spike_prob"));
    }

    #[test]
    fn negative_spike_prob_is_error() {
        let mut config = default_config();
        config.simulation_spike_prob = -0.1;
        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid());
        assert!(result
            .errors()
            .iter()
            .any(|i| i.field == "simulation_spike_prob"));
    }

    #[test]
    fn unknown_scenario_is_warning() {
        let mut config = default_config();
        config.scenario = "unknown_scenario".to_string();
        let result = ConfigValidator::validate(&config);
        assert!(result.is_valid());
        assert!(result.warnings().iter().any(|i| i.field == "scenario"));
    }

    #[test]
    fn validation_result_display_shows_errors() {
        let mut config = default_config();
        config.port = 0;
        config.horizon_url = String::new();
        let result = ConfigValidator::validate(&config);
        let display = result.display();
        assert!(display.contains("ERROR"));
        assert!(display.contains("2 error(s)"));
    }

    #[test]
    fn validation_result_has_issues() {
        let mut config = default_config();
        config.port = 0;
        let result = ConfigValidator::validate(&config);
        assert!(result.has_issues());
    }

    #[test]
    fn zero_simulation_duration_is_error() {
        let mut config = default_config();
        config.simulation_duration = 0;
        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid());
    }

    #[test]
    fn zero_simulation_base_fee_is_error() {
        let mut config = default_config();
        config.simulation_base_fee = 0;
        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid());
    }
}

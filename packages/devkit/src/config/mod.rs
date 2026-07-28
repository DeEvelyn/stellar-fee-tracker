//! Centralised configuration management for devkit.
//!
//! [`DevkitConfig`] is the master configuration struct, aggregating options
//! across the CLI, simulation, sandbox, and analysis modules. It can be
//! constructed with defaults, loaded from a TOML file (see [`loader`]), or
//! loaded from environment variables. [`validator`] provides post-load
//! validation of a constructed config.

mod defaults;
mod loader;
mod validator;

pub use validator::{ConfigValidator, ValidationIssue, ValidationResult, ValidationSeverity};

use std::collections::BTreeMap;
use std::path::PathBuf;

use defaults::{
    DEFAULT_ANALYSIS_WINDOW_HOURS, DEFAULT_BASE_RETRY_DELAY_MS, DEFAULT_DB_PATH,
    DEFAULT_HORIZON_URL, DEFAULT_POLL_INTERVAL_SECS, DEFAULT_PORT, DEFAULT_RETRY_ATTEMPTS,
    DEFAULT_SANDBOX_TIME_OFFSET_SECS, DEFAULT_SCENARIO, DEFAULT_SIMULATION_BASE_FEE,
    DEFAULT_SIMULATION_DURATION, DEFAULT_SIMULATION_SPIKE_PROB, DEFAULT_VERBOSE,
};

/// Master configuration struct for the Stellar fee tracker devkit.
///
/// Aggregates all configuration options across CLI, simulation, sandbox,
/// and analysis modules. Can be loaded from a TOML file, environment
/// variables, or constructed with defaults.
#[derive(Debug, Clone)]
pub struct DevkitConfig {
    /// Path to the fee database (SQLite).
    pub db_path: PathBuf,
    /// Default scenario file for mock data.
    pub scenario: String,
    /// Mock server port.
    pub port: u16,
    /// Whether to show detailed output.
    pub verbose: bool,
    /// Horizon API base URL.
    pub horizon_url: String,
    /// Polling interval in seconds for fee data collection.
    pub poll_interval_secs: u64,
    /// Maximum number of retry attempts for failed requests.
    pub retry_attempts: u32,
    /// Base delay in milliseconds between retries.
    pub base_retry_delay_ms: u64,
    /// Number of ledgers to simulate.
    pub simulation_duration: u64,
    /// Base fee floor in stroops for simulation.
    pub simulation_base_fee: u64,
    /// Probability of a fee spike on any given ledger [0.0, 1.0].
    pub simulation_spike_prob: f64,
    /// Sandbox time travel offset in seconds.
    pub sandbox_time_offset_secs: i64,
    /// Analysis window size in hours.
    pub analysis_window_hours: u32,
    /// Custom key-value overrides.
    pub overrides: BTreeMap<String, String>,
}

impl Default for DevkitConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from(DEFAULT_DB_PATH),
            scenario: String::from(DEFAULT_SCENARIO),
            port: DEFAULT_PORT,
            verbose: DEFAULT_VERBOSE,
            horizon_url: String::from(DEFAULT_HORIZON_URL),
            poll_interval_secs: DEFAULT_POLL_INTERVAL_SECS,
            retry_attempts: DEFAULT_RETRY_ATTEMPTS,
            base_retry_delay_ms: DEFAULT_BASE_RETRY_DELAY_MS,
            simulation_duration: DEFAULT_SIMULATION_DURATION,
            simulation_base_fee: DEFAULT_SIMULATION_BASE_FEE,
            simulation_spike_prob: DEFAULT_SIMULATION_SPIKE_PROB,
            sandbox_time_offset_secs: DEFAULT_SANDBOX_TIME_OFFSET_SECS,
            analysis_window_hours: DEFAULT_ANALYSIS_WINDOW_HOURS,
            overrides: BTreeMap::new(),
        }
    }
}

impl DevkitConfig {
    /// Display the full configuration as a formatted key/value report.
    pub fn display(&self) -> String {
        let mut out = String::new();
        out.push_str("devkit configuration\n");
        out.push_str("====================\n");

        let fields = [
            ("db_path", &self.db_path.display().to_string()),
            ("scenario", &self.scenario),
            ("port", &self.port.to_string()),
            ("verbose", &self.verbose.to_string()),
            ("horizon_url", &self.horizon_url),
            ("poll_interval_secs", &self.poll_interval_secs.to_string()),
            ("retry_attempts", &self.retry_attempts.to_string()),
            ("base_retry_delay_ms", &self.base_retry_delay_ms.to_string()),
            ("simulation_duration", &self.simulation_duration.to_string()),
            ("simulation_base_fee", &self.simulation_base_fee.to_string()),
            (
                "simulation_spike_prob",
                &self.simulation_spike_prob.to_string(),
            ),
            (
                "sandbox_time_offset_secs",
                &self.sandbox_time_offset_secs.to_string(),
            ),
            (
                "analysis_window_hours",
                &self.analysis_window_hours.to_string(),
            ),
        ];

        out.push_str(&format!("{:<12} {:<30} {}\n", "key", "value", "source"));
        for (key, value) in &fields {
            let source = if std::env::var(format!("DEVKIT_{}", key.to_uppercase())).is_ok() {
                "env"
            } else {
                "default"
            };
            out.push_str(&format!("{:<12} {:<30} {}\n", key, value, source));
        }

        if !self.overrides.is_empty() {
            out.push_str("overrides:\n");
            for (k, v) in &self.overrides {
                out.push_str(&format!("  {} = {}\n", k, v));
            }
        }
        out
    }
}

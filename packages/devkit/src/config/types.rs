use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

/// Serializable TOML representation of DevkitConfig.
#[derive(Debug, Deserialize, Default)]
struct DevkitConfigToml {
    db_path: Option<String>,
    scenario: Option<String>,
    port: Option<u16>,
    verbose: Option<bool>,
    horizon_url: Option<String>,
    poll_interval_secs: Option<u64>,
    retry_attempts: Option<u32>,
    base_retry_delay_ms: Option<u64>,
    simulation_duration: Option<u64>,
    simulation_base_fee: Option<u64>,
    simulation_spike_prob: Option<f64>,
    sandbox_time_offset_secs: Option<i64>,
    analysis_window_hours: Option<u32>,
}

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
            db_path: PathBuf::from("stellar_fees.db"),
            scenario: String::from("normal"),
            port: 8090,
            verbose: false,
            horizon_url: String::from("https://horizon-testnet.stellar.org"),
            poll_interval_secs: 10,
            retry_attempts: 3,
            base_retry_delay_ms: 1000,
            simulation_duration: 1000,
            simulation_base_fee: 100,
            simulation_spike_prob: 0.05,
            sandbox_time_offset_secs: 0,
            analysis_window_hours: 24,
            overrides: BTreeMap::new(),
        }
    }
}

impl DevkitConfig {
    /// Load configuration from a TOML file.
    pub fn from_toml_file(path: &PathBuf) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
        let toml_cfg: DevkitConfigToml =
            toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))?;

        let mut cfg = Self::default();
        if let Some(v) = toml_cfg.db_path {
            cfg.db_path = PathBuf::from(v);
        }
        if let Some(v) = toml_cfg.scenario {
            cfg.scenario = v;
        }
        if let Some(v) = toml_cfg.port {
            cfg.port = v;
        }
        if let Some(v) = toml_cfg.verbose {
            cfg.verbose = v;
        }
        if let Some(v) = toml_cfg.horizon_url {
            cfg.horizon_url = v;
        }
        if let Some(v) = toml_cfg.poll_interval_secs {
            cfg.poll_interval_secs = v;
        }
        if let Some(v) = toml_cfg.retry_attempts {
            cfg.retry_attempts = v;
        }
        if let Some(v) = toml_cfg.base_retry_delay_ms {
            cfg.base_retry_delay_ms = v;
        }
        if let Some(v) = toml_cfg.simulation_duration {
            cfg.simulation_duration = v;
        }
        if let Some(v) = toml_cfg.simulation_base_fee {
            cfg.simulation_base_fee = v;
        }
        if let Some(v) = toml_cfg.simulation_spike_prob {
            cfg.simulation_spike_prob = v;
        }
        if let Some(v) = toml_cfg.sandbox_time_offset_secs {
            cfg.sandbox_time_offset_secs = v;
        }
        if let Some(v) = toml_cfg.analysis_window_hours {
            cfg.analysis_window_hours = v;
        }

        Ok(cfg)
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        cfg.apply_env();
        cfg
    }

    /// Apply environment variable overrides on top of the current config.
    pub fn apply_env(&mut self) {
        if let Ok(v) = std::env::var("DEVKIT_DB_PATH") {
            self.db_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("DEVKIT_SCENARIO") {
            self.scenario = v;
        }
        if let Ok(v) = std::env::var("DEVKIT_PORT") {
            self.port = v.parse().unwrap_or(self.port);
        }
        if let Ok(v) = std::env::var("DEVKIT_VERBOSE") {
            self.verbose = v == "true" || v == "1";
        }
        if let Ok(v) = std::env::var("DEVKIT_HORIZON_URL") {
            self.horizon_url = v;
        }
        if let Ok(v) = std::env::var("DEVKIT_POLL_INTERVAL_SECS") {
            self.poll_interval_secs = v.parse().unwrap_or(self.poll_interval_secs);
        }
        if let Ok(v) = std::env::var("DEVKIT_RETRY_ATTEMPTS") {
            self.retry_attempts = v.parse().unwrap_or(self.retry_attempts);
        }
        if let Ok(v) = std::env::var("DEVKIT_BASE_RETRY_DELAY_MS") {
            self.base_retry_delay_ms = v.parse().unwrap_or(self.base_retry_delay_ms);
        }
        if let Ok(v) = std::env::var("DEVKIT_SIMULATION_DURATION") {
            self.simulation_duration = v.parse().unwrap_or(self.simulation_duration);
        }
        if let Ok(v) = std::env::var("DEVKIT_SIMULATION_BASE_FEE") {
            self.simulation_base_fee = v.parse().unwrap_or(self.simulation_base_fee);
        }
        if let Ok(v) = std::env::var("DEVKIT_SIMULATION_SPIKE_PROB") {
            self.simulation_spike_prob = v.parse().unwrap_or(self.simulation_spike_prob);
        }
        if let Ok(v) = std::env::var("DEVKIT_SANDBOX_TIME_OFFSET_SECS") {
            self.sandbox_time_offset_secs = v.parse().unwrap_or(self.sandbox_time_offset_secs);
        }
        if let Ok(v) = std::env::var("DEVKIT_ANALYSIS_WINDOW_HOURS") {
            self.analysis_window_hours = v.parse().unwrap_or(self.analysis_window_hours);
        }
    }

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
            ("simulation_spike_prob", &self.simulation_spike_prob.to_string()),
            (
                "sandbox_time_offset_secs",
                &self.sandbox_time_offset_secs.to_string(),
            ),
            (
                "analysis_window_hours",
                &self.analysis_window_hours.to_string(),
            ),
        ];

        out.push_str(&format!(
            "{:<12} {:<30} {}\n",
            "key", "value", "source"
        ));
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

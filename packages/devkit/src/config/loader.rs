//! Loading logic for `DevkitConfig`: TOML files and environment variables.

use std::path::PathBuf;

use serde::Deserialize;

use super::DevkitConfig;

/// Serializable TOML representation of DevkitConfig.
#[derive(Debug, Deserialize, Default)]
pub(crate) struct DevkitConfigToml {
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

impl DevkitConfig {
    /// Load configuration from a TOML file.
    pub fn from_toml_file(path: &PathBuf) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
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
}

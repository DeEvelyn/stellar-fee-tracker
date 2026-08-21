//! Pre-built test scenarios for the Stellar fee tracker harness.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Percentile fee distribution returned by Horizon's `/fee_stats` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDistribution {
    pub max: String,
    pub min: String,
    pub mode: String,
    pub p10: String,
    pub p20: String,
    pub p30: String,
    pub p40: String,
    pub p50: String,
    pub p60: String,
    pub p70: String,
    pub p80: String,
    pub p90: String,
    pub p95: String,
    pub p99: String,
}

/// The `fee_stats` block within a scenario document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioFeeStats {
    pub last_ledger: String,
    pub last_ledger_base_fee: String,
    pub ledger_capacity_usage: String,
    pub fee_charged: FeeDistribution,
    pub max_fee: FeeDistribution,
}

/// Full scenario document as parsed from a JSON fixture file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub scenario: String,
    pub description: String,
    pub fee_stats: ScenarioFeeStats,
}

/// Loads and validates a scenario from a JSON file at the given path.
///
/// Asserts all required fields are present and non-empty at load time.
/// Returns a clear error message identifying the first missing field.
pub fn load_scenario(path: &std::path::Path) -> Result<Scenario, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let scenario: Scenario = serde_json::from_str(&contents)?;
    validate_scenario(&scenario)?;
    Ok(scenario)
}

/// Validates that all required Horizon fee_stats schema fields are present and non-empty.
///
/// Called on startup to catch malformed scenario files before they reach a handler.
pub fn validate_scenario(s: &Scenario) -> Result<(), Box<dyn std::error::Error>> {
    if s.scenario.is_empty() {
        return Err("scenario: field must not be empty".into());
    }
    if s.fee_stats.last_ledger.is_empty() {
        return Err("fee_stats.last_ledger: field must not be empty".into());
    }
    if s.fee_stats.last_ledger_base_fee.is_empty() {
        return Err("fee_stats.last_ledger_base_fee: field must not be empty".into());
    }
    if s.fee_stats.ledger_capacity_usage.is_empty() {
        return Err("fee_stats.ledger_capacity_usage: field must not be empty".into());
    }
    validate_fee_distribution(&s.fee_stats.fee_charged, "fee_charged")?;
    validate_fee_distribution(&s.fee_stats.max_fee, "max_fee")?;
    Ok(())
}

fn validate_fee_distribution(
    d: &FeeDistribution,
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let required = [
        ("max", d.max.as_str()),
        ("min", d.min.as_str()),
        ("mode", d.mode.as_str()),
        ("p10", d.p10.as_str()),
        ("p20", d.p20.as_str()),
        ("p30", d.p30.as_str()),
        ("p40", d.p40.as_str()),
        ("p50", d.p50.as_str()),
        ("p60", d.p60.as_str()),
        ("p70", d.p70.as_str()),
        ("p80", d.p80.as_str()),
        ("p90", d.p90.as_str()),
        ("p95", d.p95.as_str()),
        ("p99", d.p99.as_str()),
    ];
    for (field, value) in &required {
        if value.is_empty() {
            return Err(format!("fee_stats.{}.{}: field must not be empty", prefix, field).into());
        }
    }
    Ok(())
}

/// Loads a scenario JSON file from the given path and returns its contents.
pub fn load_from_file(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

/// Cycles through a list of scenario names, returning the next one each call.
///
/// Useful in test harnesses that want to rotate through scenarios such as
/// `normal`, `spike`, `congested`, and `recovery` in a deterministic order.
pub struct ScenarioRotator {
    scenarios: Vec<String>,
    index: usize,
    /// How often (in seconds) to advance to the next scenario. 0 = manual only.
    pub interval_secs: u64,
    /// Unix timestamp of the last rotation.
    last_rotated: u64,
}

impl ScenarioRotator {
    /// Creates a new `ScenarioRotator` from an ordered list of scenario names.
    ///
    /// The rotator starts at the first scenario in the list.
    pub fn new(scenarios: Vec<String>) -> Self {
        Self {
            scenarios,
            index: 0,
            interval_secs: 0,
            last_rotated: current_unix_secs(),
        }
    }

    /// Creates a rotator that automatically advances every `interval_secs` seconds.
    pub fn with_interval(scenarios: Vec<String>, interval_secs: u64) -> Self {
        Self {
            scenarios,
            index: 0,
            interval_secs,
            last_rotated: current_unix_secs(),
        }
    }

    /// Returns the current scenario name and advances to the next.
    pub fn advance(&mut self) -> Option<&str> {
        if self.scenarios.is_empty() {
            return None;
        }
        let current = self.scenarios[self.index].as_str();
        self.index = (self.index + 1) % self.scenarios.len();
        self.last_rotated = current_unix_secs();
        Some(current)
    }

    /// Advances if the rotation interval has elapsed. Returns the new scenario name if rotated.
    pub fn advance_if_due(&mut self) -> Option<&str> {
        if self.interval_secs == 0 {
            return None;
        }
        let elapsed = current_unix_secs().saturating_sub(self.last_rotated);
        if elapsed >= self.interval_secs {
            self.advance()
        } else {
            None
        }
    }
}

fn current_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

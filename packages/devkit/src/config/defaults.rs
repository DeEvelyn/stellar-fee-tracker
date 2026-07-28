//! Default values for `DevkitConfig`.
//!
//! Centralised here so the fallback values used by `Default`, the TOML
//! loader, and env-var parsing all trace back to a single source of truth.

pub const DEFAULT_DB_PATH: &str = "stellar_fees.db";
pub const DEFAULT_SCENARIO: &str = "normal";
pub const DEFAULT_PORT: u16 = 8090;
pub const DEFAULT_VERBOSE: bool = false;
pub const DEFAULT_HORIZON_URL: &str = "https://horizon-testnet.stellar.org";
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 10;
pub const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
pub const DEFAULT_BASE_RETRY_DELAY_MS: u64 = 1000;
pub const DEFAULT_SIMULATION_DURATION: u64 = 1000;
pub const DEFAULT_SIMULATION_BASE_FEE: u64 = 100;
pub const DEFAULT_SIMULATION_SPIKE_PROB: f64 = 0.05;
pub const DEFAULT_SANDBOX_TIME_OFFSET_SECS: i64 = 0;
pub const DEFAULT_ANALYSIS_WINDOW_HOURS: u32 = 24;

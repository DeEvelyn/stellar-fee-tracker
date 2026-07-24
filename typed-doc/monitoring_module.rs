/// Documentation metadata for the monitoring module.
///
/// This module provides observability utilities including:
/// - Trace context propagation across module boundaries
/// - Benchmarking infrastructure for measuring overhead
/// - Log rotation with configurable size and retention policies
///
/// # Module Structure
///
/// ```text
/// src/monitoring/
/// ├── mod.rs              -- Re-exports + module constants
/// ├── trace_context.rs    -- TraceId, SpanId, TraceContext, TraceRegistry
/// ├── benchmark.rs        -- Measurement, MonitoringBenchmark
/// └── log_rotation.rs     -- LogRotationConfig, RotatingLogWriter
/// ```

/// Current version of the monitoring module API.
pub const MONITORING_VERSION: &str = "1.0.0";

/// Default sampling rate for trace spans (1.0 = sample all).
pub const DEFAULT_SAMPLING_RATE: f64 = 1.0;

/// Maximum baggage key-value pairs per trace context.
pub const MAX_BAGGAGE_ENTRIES: usize = 64;

/// Maximum traceparent header length (W3C spec).
pub const TRACEPARENT_MAX_LENGTH: usize = 55;

/// Default log file size before rotation (10 MB).
pub const DEFAULT_LOG_MAX_SIZE: u64 = 10 * 1024 * 1024;

/// Default number of archived log files to retain.
pub const DEFAULT_LOG_MAX_FILES: u32 = 5;

/// Default log output directory.
pub const DEFAULT_LOG_DIR: &str = "logs";

/// Known trace propagation formats supported by the module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropagationFormat {
    W3CTraceparent,
    ZipkinB3,
    Jaeger,
    Datadog,
}

impl PropagationFormat {
    /// Return the HTTP header name used by this format.
    pub fn header_name(&self) -> &'static str {
        match self {
            Self::W3CTraceparent => "traceparent",
            Self::ZipkinB3 => "b3",
            Self::Jaeger => "uber-trace-id",
            Self::Datadog => "x-datadog-trace-id",
        }
    }

    /// Return all supported formats.
    pub fn all() -> &'static [PropagationFormat] {
        &[
            Self::W3CTraceparent,
            Self::ZipkinB3,
            Self::Jaeger,
            Self::Datadog,
        ]
    }
}

/// Severity levels for monitoring events.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MonitoringLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl MonitoringLevel {
    /// Parse a level from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" | "warning" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    /// Return the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// A metric label for tagging monitoring data.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MetricLabel {
    pub key: String,
    pub value: String,
}

impl MetricLabel {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

/// Monitoring configuration that can be serialized/deserialized.
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Whether trace propagation is enabled.
    pub trace_enabled: bool,
    /// Whether benchmark collection is enabled.
    pub benchmark_enabled: bool,
    /// Whether log rotation is enabled.
    pub log_rotation_enabled: bool,
    /// Sampling rate for trace spans [0.0, 1.0].
    pub sampling_rate: f64,
    /// Active propagation format.
    pub propagation_format: PropagationFormat,
    /// Active monitoring level.
    pub level: MonitoringLevel,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            trace_enabled: true,
            benchmark_enabled: false,
            log_rotation_enabled: true,
            sampling_rate: DEFAULT_SAMPLING_RATE,
            propagation_format: PropagationFormat::W3CTraceparent,
            level: MonitoringLevel::Info,
        }
    }
}

impl MonitoringConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if !(0.0..=1.0).contains(&self.sampling_rate) {
            issues.push("sampling_rate must be between 0.0 and 1.0".into());
        }
        issues
    }

    /// Display the configuration as a formatted string.
    pub fn display(&self) -> String {
        format!(
            "Monitoring Configuration\n\
             ========================\n\
             trace:              {}\n\
             benchmark:          {}\n\
             log_rotation:       {}\n\
             sampling_rate:      {:.2}\n\
             propagation_format: {:?}\n\
             level:              {}\n",
            if self.trace_enabled { "enabled" } else { "disabled" },
            if self.benchmark_enabled { "enabled" } else { "disabled" },
            if self.log_rotation_enabled { "enabled" } else { "disabled" },
            self.sampling_rate,
            self.propagation_format,
            self.level.as_str(),
        )
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"trace":{},"benchmark":{},"log_rotation":{},"sampling_rate":{},"format":"{:?}","level":"{}"}}"#,
            self.trace_enabled,
            self.benchmark_enabled,
            self.log_rotation_enabled,
            self.sampling_rate,
            self.propagation_format,
            self.level.as_str(),
        )
    }
}

/// Generate a summary of the monitoring module's public API.
pub fn module_summary() -> Vec<(String, String)> {
    vec![
        ("TraceContext".into(), "W3C trace context with baggage propagation".into()),
        ("TraceRegistry".into(), "Thread-safe trace context store".into()),
        ("W3CPropagator".into(), "Inject/extract traceparent headers".into()),
        ("MonitoringBenchmark".into(), "Overhead measurement suite".into()),
        ("Measurement".into(), "Single benchmark result".into()),
        ("LogRotationConfig".into(), "Log file rotation settings".into()),
        ("RotatingLogWriter".into(), "Auto-rotating log file writer".into()),
        ("MonitoringConfig".into(), "Global monitoring settings".into()),
    ]
}

/// Arguments for the monitoring documentation subcommand.
pub struct MonitoringDocsArgs {
    /// Show the module summary.
    pub summary: bool,
    /// Show configuration documentation.
    pub config: bool,
    /// Output as JSON.
    pub json: bool,
}

impl Default for MonitoringDocsArgs {
    fn default() -> Self {
        Self {
            summary: true,
            config: false,
            json: false,
        }
    }
}

impl MonitoringDocsArgs {
    /// Run the monitoring docs subcommand.
    pub fn run(&self) {
        if self.summary {
            let items = module_summary();
            if self.json {
                let pairs: Vec<String> = items
                    .iter()
                    .map(|(n, d)| format!(r#"{{"name":"{}","desc":"{}"}}"#, n, d))
                    .collect();
                println!("[{}]", pairs.join(","));
            } else {
                println!("Monitoring Module — Public API");
                println!("===============================");
                for (name, desc) in &items {
                    println!("  {:<25} {}", name, desc);
                }
            }
        }
        if self.config {
            let cfg = MonitoringConfig::default();
            if self.json {
                println!("{}", cfg.to_json());
            } else {
                println!("\n{}", cfg.display());
            }
        }
    }

    /// Generate markdown documentation for the monitoring module.
    pub fn generate_docs() -> String {
        let mut docs = String::new();
        docs.push_str("# Monitoring Module\n\n");
        docs.push_str("## Overview\n\n");
        docs.push_str("The monitoring module provides distributed tracing, benchmarking, and log rotation for the devkit.\n\n");
        docs.push_str("## Components\n\n");
        docs.push_str("| Component | Description |\n");
        docs.push_str("|-----------|-------------|\n");
        for (name, desc) in module_summary() {
            docs.push_str(&format!("| `{}` | {} |\n", name, desc));
        }
        docs.push_str("\n## Configuration\n\n");
        docs.push_str("```rust\n");
        docs.push_str("MonitoringConfig {\n");
        docs.push_str("    trace_enabled: true,\n");
        docs.push_str("    benchmark_enabled: false,\n");
        docs.push_str("    log_rotation_enabled: true,\n");
        docs.push_str("    sampling_rate: 1.0,\n");
        docs.push_str("    propagation_format: W3CTraceparent,\n");
        docs.push_str("    level: Info,\n");
        docs.push_str("}\n```\n");
        docs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitoring_version_defined() {
        assert!(!MONITORING_VERSION.is_empty());
    }

    #[test]
    fn propagation_format_header_name() {
        assert_eq!(PropagationFormat::W3CTraceparent.header_name(), "traceparent");
        assert_eq!(PropagationFormat::ZipkinB3.header_name(), "b3");
    }

    #[test]
    fn propagation_format_all_returns_four() {
        assert_eq!(PropagationFormat::all().len(), 4);
    }

    #[test]
    fn monitoring_level_parse() {
        assert_eq!(MonitoringLevel::parse("info"), Some(MonitoringLevel::Info));
        assert_eq!(MonitoringLevel::parse("WARN"), Some(MonitoringLevel::Warn));
        assert_eq!(MonitoringLevel::parse("unknown"), None);
    }

    #[test]
    fn monitoring_level_as_str() {
        assert_eq!(MonitoringLevel::Debug.as_str(), "debug");
        assert_eq!(MonitoringLevel::Error.as_str(), "error");
    }

    #[test]
    fn config_validation_passes() {
        let cfg = MonitoringConfig::default();
        assert!(cfg.validate().is_empty());
    }

    #[test]
    fn config_validation_fails_for_bad_rate() {
        let cfg = MonitoringConfig {
            sampling_rate: 1.5,
            ..Default::default()
        };
        assert!(!cfg.validate().is_empty());
    }

    #[test]
    fn config_display_contains_fields() {
        let out = MonitoringConfig::default().display();
        assert!(out.contains("trace"));
        assert!(out.contains("benchmark"));
    }

    #[test]
    fn config_json_is_valid() {
        let json = MonitoringConfig::default().to_json();
        assert!(json.contains("sampling_rate"));
    }

    #[test]
    fn module_summary_returns_items() {
        let items = module_summary();
        assert!(items.len() >= 7);
        assert!(items.iter().any(|(n, _)| n == "TraceContext"));
    }

    #[test]
    fn generate_docs_contains_overview() {
        let docs = MonitoringDocsArgs::generate_docs();
        assert!(docs.contains("Monitoring Module"));
        assert!(docs.contains("TraceContext"));
        assert!(docs.contains("RotatingLogWriter"));
    }

    #[test]
    fn metric_label_new() {
        let label = MetricLabel::new("env", "prod");
        assert_eq!(label.key, "env");
        assert_eq!(label.value, "prod");
    }

    #[test]
    fn monitoring_docs_args_default() {
        let args = MonitoringDocsArgs::default();
        assert!(args.summary);
        assert!(!args.config);
    }

    #[test]
    fn max_baggage_entries_constant() {
        assert_eq!(MAX_BAGGAGE_ENTRIES, 64);
    }
}

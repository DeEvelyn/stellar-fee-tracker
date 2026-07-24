use crate::simulation::fee_model::FeePoint;

/// Priority level for fee conversion targets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FeePriority {
    Low,
    Medium,
    High,
    Custom(u64),
}

impl FeePriority {
    /// Parse a priority from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => s.parse::<u64>().ok().map(Self::Custom),
        }
    }

    /// Return the multiplier used to compute the recommended fee.
    pub fn multiplier(&self) -> f64 {
        match self {
            Self::Low => 1.0,
            Self::Medium => 1.5,
            Self::High => 2.0,
            Self::Custom(v) => *v as f64,
        }
    }
}

/// Converts fee data between different representations.
pub struct Convert;

impl Convert {
    /// Compute the recommended fee in stroops for a given priority.
    pub fn to_stroops(fee: u64, priority: FeePriority) -> u64 {
        (fee as f64 * priority.multiplier()).round() as u64
    }

    /// Convert a stroop value to XLM (1 XLM = 10_000_000 stroops).
    pub fn stroops_to_xlm(stroops: u64) -> f64 {
        stroops as f64 / 10_000_000.0
    }

    /// Convert an XLM value to stroops.
    pub fn xlm_to_stroops(xlm: f64) -> u64 {
        (xlm * 10_000_000.0).round() as u64
    }

    /// Format a fee value as a human-readable string with unit.
    pub fn format_fee(stroops: u64) -> String {
        if stroops >= 10_000_000 {
            format!("{:.4} XLM", Self::stroops_to_xlm(stroops))
        } else if stroops >= 1000 {
            format!("{} stroops", stroops)
        } else {
            format!("{} stroop", stroops)
        }
    }

    /// Convert a slice of FeePoints to a JSON string.
    pub fn points_to_json(points: &[FeePoint]) -> String {
        let items: Vec<String> = points
            .iter()
            .map(|p| {
                format!(
                    r#"{{"t":{},"f":{},"l":{},"s":{}}}"#,
                    p.timestamp, p.fee, p.ledger, p.is_spike
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }

    /// Convert a JSON string to a vector of FeePoints.
    pub fn json_to_points(json: &str) -> Result<Vec<FeePoint>, String> {
        let items: Vec<FeePoint> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        Ok(items)
    }

    /// Convert fee points to tab-separated values.
    pub fn to_tsv(points: &[FeePoint]) -> String {
        let mut out = String::from("timestamp\tfee\tledger\tspike\n");
        for p in points {
            out.push_str(&format!("{}\t{}\t{}\t{}\n", p.timestamp, p.fee, p.ledger, p.is_spike));
        }
        out
    }
}

/// Arguments for the `convert` subcommand.
pub struct ConvertArgs {
    /// Input format (json, csv, tsv).
    pub from: String,
    /// Output format (json, csv, tsv, xlm, stroops).
    pub to: String,
    /// Priority level for fee calculation.
    pub priority: Option<String>,
    /// Input file path; reads from stdin if not provided.
    pub input: Option<String>,
    /// Output file path; writes to stdout if not provided.
    pub output: Option<String>,
}

impl Default for ConvertArgs {
    fn default() -> Self {
        Self {
            from: "json".into(),
            to: "csv".into(),
            priority: None,
            input: None,
            output: None,
        }
    }
}

impl ConvertArgs {
    /// Run the convert subcommand.
    pub fn run(&self) {
        let priority = self
            .priority
            .as_deref()
            .and_then(FeePriority::parse)
            .unwrap_or(FeePriority::Medium);
        eprintln!(
            "Converting from {} to {} (priority={:?})",
            self.from, self.to, priority
        );
        if let Some(ref path) = self.output {
            eprintln!("Output path: {}", path);
        }
    }

    /// Validate the conversion flags.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let formats = ["json", "csv", "tsv", "xlm", "stroops"];
        if !formats.contains(&self.from.as_str()) {
            errors.push(format!("unsupported input format: {}", self.from));
        }
        if !formats.contains(&self.to.as_str()) {
            errors.push(format!("unsupported output format: {}", self.to));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Detect the input format from a file extension.
    pub fn detect_format(path: &str) -> &str {
        if path.ends_with(".csv") {
            "csv"
        } else if path.ends_with(".tsv") || path.ends_with(".tab") {
            "tsv"
        } else if path.ends_with(".xlm") {
            "xlm"
        } else {
            "json"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fee_model::FeePoint;

    fn sample_points() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 3600, fee: 250, ledger: 2, is_spike: true },
        ]
    }

    #[test]
    fn stroops_to_xlm_conversion() {
        let xlm = Convert::stroops_to_xlm(10_000_000);
        assert!((xlm - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn xlm_to_stroops_conversion() {
        let stroops = Convert::xlm_to_stroops(0.5);
        assert_eq!(stroops, 5_000_000);
    }

    #[test]
    fn to_stroops_uses_priority_multiplier() {
        let fee = Convert::to_stroops(100, FeePriority::High);
        assert_eq!(fee, 200);
    }

    #[test]
    fn format_fee_uses_xlm_for_large_values() {
        let formatted = Convert::format_fee(50_000_000);
        assert!(formatted.contains("XLM"));
    }

    #[test]
    fn format_fee_uses_stroops_for_small_values() {
        let formatted = Convert::format_fee(500);
        assert!(formatted.contains("stroop"));
    }

    #[test]
    fn points_to_json_creates_array() {
        let json = Convert::points_to_json(&sample_points());
        assert!(json.starts_with('['));
        assert!(json.contains("\"t\":0"));
    }

    #[test]
    fn to_tsv_includes_header() {
        let tsv = Convert::to_tsv(&sample_points());
        assert!(tsv.starts_with("timestamp"));
    }

    #[test]
    fn fee_priority_parse_known_values() {
        assert_eq!(FeePriority::parse("low"), Some(FeePriority::Low));
        assert_eq!(FeePriority::parse("high"), Some(FeePriority::High));
    }

    #[test]
    fn fee_priority_parse_custom_number() {
        let p = FeePriority::parse("3");
        assert_eq!(p, Some(FeePriority::Custom(3)));
        assert_eq!(p.unwrap().multiplier(), 3.0);
    }

    #[test]
    fn convert_args_default_is_valid() {
        let args = ConvertArgs::default();
        assert!(args.validate().is_ok());
    }

    #[test]
    fn detect_format_from_extension() {
        assert_eq!(ConvertArgs::detect_format("data.csv"), "csv");
        assert_eq!(ConvertArgs::detect_format("data.tsv"), "tsv");
        assert_eq!(ConvertArgs::detect_format("data.json"), "json");
    }
}

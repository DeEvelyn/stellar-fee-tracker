use crate::simulation::fee_model::FeePoint;

/// A data quality report in JSON format.
#[derive(Debug, Clone)]
pub struct JsonQualityReport {
    pub total_points: usize,
    pub quality_score: f64,
    pub issues: QualityIssues,
    pub fee_stats: FeeStats,
    pub issue_details: Vec<IssueEntry>,
}

#[derive(Debug, Clone)]
pub struct QualityIssues {
    pub duplicates: usize,
    pub gaps: usize,
    pub outliers: usize,
    pub zero_fees: usize,
    pub out_of_order: usize,
    pub total: usize,
}

#[derive(Debug, Clone)]
pub struct FeeStats {
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub median: u64,
    pub std_dev: f64,
}

#[derive(Debug, Clone)]
pub struct IssueEntry {
    pub issue_type: String,
    pub ledger: u64,
    pub description: String,
}

impl JsonQualityReport {
    /// Generate a JSON string of the report.
    pub fn to_json(&self) -> String {
        let details: Vec<String> = self
            .issue_details
            .iter()
            .map(|e| {
                format!(
                    r#"{{"type":"{}","ledger":{},"desc":"{}"}}"#,
                    e.issue_type, e.ledger, e.description
                )
            })
            .collect();
        format!(
            r#"{{
  "total_points": {},
  "quality_score": {:.4},
  "issues": {{
    "duplicates": {},
    "gaps": {},
    "outliers": {},
    "zero_fees": {},
    "out_of_order": {},
    "total": {}
  }},
  "fee_stats": {{
    "min": {},
    "max": {},
    "mean": {:.1},
    "median": {},
    "std_dev": {:.2}
  }},
  "issue_details": [{}]
}}"#,
            self.total_points,
            self.quality_score,
            self.issues.duplicates,
            self.issues.gaps,
            self.issues.outliers,
            self.issues.zero_fees,
            self.issues.out_of_order,
            self.issues.total,
            self.fee_stats.min,
            self.fee_stats.max,
            self.fee_stats.mean,
            self.fee_stats.median,
            self.fee_stats.std_dev,
            details.join(","),
        )
    }

    /// Generate a compact single-line JSON.
    pub fn to_json_compact(&self) -> String {
        format!(
            r#"{{"points":{},"score":{:.4},"issues":{},"min":{},"max":{},"mean":{:.1}}}"#,
            self.total_points,
            self.quality_score,
            self.issues.total,
            self.fee_stats.min,
            self.fee_stats.max,
            self.fee_stats.mean,
        )
    }
}

/// Generate a JSON-format quality report from fee points.
pub fn generate_json_report(points: &[FeePoint]) -> JsonQualityReport {
    let mut details = Vec::new();
    let mut seen_ledgers = std::collections::BTreeSet::new();
    let mut duplicates = 0;
    let mut gaps = 0;
    let mut outliers = 0;
    let mut zero_fees = 0;
    let mut out_of_order = 0;
    let mut prev_ledger: Option<u64> = None;
    let mut prev_ts: Option<u64> = None;

    let mut fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
    fees.sort_unstable();
    let mean = fees.iter().sum::<u64>() as f64 / fees.len().max(1) as f64;
    let std_dev = {
        let variance = fees.iter().map(|f| (*f as f64 - mean).powi(2)).sum::<f64>() / fees.len().max(1) as f64;
        variance.sqrt()
    };

    for p in points {
        if p.fee == 0 {
            zero_fees += 1;
            details.push(IssueEntry {
                issue_type: "zero_fee".into(),
                ledger: p.ledger,
                description: format!("Zero fee at ledger {}", p.ledger),
            });
        }
        if !seen_ledgers.insert(p.ledger) {
            duplicates += 1;
            details.push(IssueEntry {
                issue_type: "duplicate".into(),
                ledger: p.ledger,
                description: format!("Duplicate ledger {}", p.ledger),
            });
        }
        if let Some(pl) = prev_ledger {
            if p.ledger > pl + 1 {
                let gap = p.ledger - pl - 1;
                gaps += gap as usize;
                details.push(IssueEntry {
                    issue_type: "gap".into(),
                    ledger: p.ledger,
                    description: format!("Gap {} -> {} ({} missing)", pl, p.ledger, gap),
                });
            }
        }
        if let Some(ts) = prev_ts {
            if p.timestamp < ts {
                out_of_order += 1;
                details.push(IssueEntry {
                    issue_type: "out_of_order".into(),
                    ledger: p.ledger,
                    description: format!("Timestamp {} < prev {}", p.timestamp, ts),
                });
            }
        }
        if std_dev > 0.0 {
            let z = (p.fee as f64 - mean) / std_dev;
            if z.abs() > 3.0 {
                outliers += 1;
                details.push(IssueEntry {
                    issue_type: "outlier".into(),
                    ledger: p.ledger,
                    description: format!("Fee {} (z={:.1})", p.fee, z),
                });
            }
        }
        prev_ledger = Some(p.ledger);
        prev_ts = Some(p.timestamp);
    }

    let total = duplicates + gaps + outliers + zero_fees + out_of_order;
    let quality_score = if points.is_empty() {
        0.0
    } else {
        (1.0 - (total as f64 / points.len().max(1) as f64)).max(0.0)
    };

    let median = fees.get(fees.len() / 2).copied().unwrap_or(0);
    let min = fees.first().copied().unwrap_or(0);
    let max = fees.last().copied().unwrap_or(0);

    JsonQualityReport {
        total_points: points.len(),
        quality_score,
        issues: QualityIssues { duplicates, gaps, outliers, zero_fees, out_of_order, total },
        fee_stats: FeeStats { min, max, mean, median, std_dev },
        issue_details: details,
    }
}

/// Arguments for the JSON report subcommand.
pub struct JsonReportArgs {
    /// Use compact format (single line).
    pub compact: bool,
}

impl Default for JsonReportArgs {
    fn default() -> Self {
        Self { compact: false }
    }
}

impl JsonReportArgs {
    /// Run the JSON report subcommand.
    pub fn run(&self, points: &[FeePoint]) {
        let report = generate_json_report(points);
        if self.compact {
            println!("{}", report.to_json_compact());
        } else {
            println!("{}", report.to_json());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fee_model::FeePoint;

    fn sample() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 200, ledger: 2, is_spike: false },
            FeePoint { timestamp: 200, fee: 150, ledger: 3, is_spike: false },
        ]
    }

    #[test]
    fn clean_data_high_score() {
        let report = generate_json_report(&sample());
        assert!(report.quality_score > 0.9);
        assert_eq!(report.issues.duplicates, 0);
    }

    #[test]
    fn json_output_valid_structure() {
        let report = generate_json_report(&sample());
        let json = report.to_json();
        assert!(json.contains("quality_score"));
        assert!(json.contains("total_points"));
    }

    #[test]
    fn compact_json_single_line() {
        let report = generate_json_report(&sample());
        let json = report.to_json_compact();
        assert!(!json.contains('\n'));
        assert!(json.contains("score"));
    }

    #[test]
    fn detects_issues() {
        let data = vec![
            FeePoint { timestamp: 0, fee: 0, ledger: 1, is_spike: false },
            FeePoint { timestamp: 10, fee: 100, ledger: 1, is_spike: false },
        ];
        let report = generate_json_report(&data);
        assert_eq!(report.issues.zero_fees, 1);
        assert_eq!(report.issues.duplicates, 1);
    }

    #[test]
    fn empty_data_zero_score() {
        let report = generate_json_report(&[]);
        assert_eq!(report.quality_score, 0.0);
    }

    #[test]
    fn fee_stats_computed() {
        let report = generate_json_report(&sample());
        assert_eq!(report.fee_stats.min, 100);
        assert_eq!(report.fee_stats.max, 200);
    }

    #[test]
    fn args_default() {
        let args = JsonReportArgs::default();
        assert!(!args.compact);
    }
}

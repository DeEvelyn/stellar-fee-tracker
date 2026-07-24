use crate::simulation::fee_model::FeePoint;
use std::fmt::Write as FmtWrite;

/// A data quality report in human-readable text format.
#[derive(Debug, Clone)]
pub struct TextQualityReport {
    pub total_points: usize,
    pub duplicate_count: usize,
    pub gap_count: usize,
    pub outlier_count: usize,
    pub zero_fee_count: usize,
    pub out_of_order_count: usize,
    pub min_fee: u64,
    pub max_fee: u64,
    pub avg_fee: f64,
    pub score: f64,
    pub issues: Vec<String>,
}

impl TextQualityReport {
    /// Generate the text report.
    pub fn format(&self) -> String {
        let mut out = String::new();
        writeln!(out, "Data Quality Report").unwrap();
        writeln!(out, "=====================").unwrap();
        writeln!(out, "Score:              {:.1}%", self.score * 100.0).unwrap();
        writeln!(out, "Total points:       {}", self.total_points).unwrap();
        writeln!(out, "Duplicates:         {}", self.duplicate_count).unwrap();
        writeln!(out, "Gaps:               {}", self.gap_count).unwrap();
        writeln!(out, "Outliers:           {}", self.outlier_count).unwrap();
        writeln!(out, "Zero fees:          {}", self.zero_fee_count).unwrap();
        writeln!(out, "Out-of-order:       {}", self.out_of_order_count).unwrap();
        writeln!(out, "Fee range:          {} - {} stroops", self.min_fee, self.max_fee).unwrap();
        writeln!(out, "Average fee:        {:.1} stroops", self.avg_fee).unwrap();
        if !self.issues.is_empty() {
            writeln!(out, "\nIssues:").unwrap();
            for issue in &self.issues {
                writeln!(out, "  - {}", issue).unwrap();
            }
        }
        out
    }
}

/// Generate a text-format quality report from fee points.
pub fn generate_text_report(points: &[FeePoint]) -> TextQualityReport {
    let mut issues = Vec::new();
    let mut seen_ledgers = std::collections::BTreeSet::new();
    let mut duplicate_count = 0;
    let mut gap_count = 0;
    let mut outlier_count = 0;
    let mut zero_fee_count = 0;
    let mut out_of_order_count = 0;
    let mut prev_ledger: Option<u64> = None;
    let mut prev_ts: Option<u64> = None;

    let fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
    let mean = fees.iter().sum::<u64>() as f64 / fees.len().max(1) as f64;
    let std_dev = {
        let variance = fees.iter().map(|f| (*f as f64 - mean).powi(2)).sum::<f64>() / fees.len().max(1) as f64;
        variance.sqrt()
    };

    for p in points {
        if p.fee == 0 {
            zero_fee_count += 1;
            issues.push(format!("Zero fee at ledger {}", p.ledger));
        }
        if !seen_ledgers.insert(p.ledger) {
            duplicate_count += 1;
            issues.push(format!("Duplicate ledger {}", p.ledger));
        }
        if let Some(pl) = prev_ledger {
            if p.ledger > pl + 1 {
                let gap = p.ledger - pl - 1;
                gap_count += gap as usize;
                issues.push(format!("Ledger gap {} -> {} ({} missing)", pl, p.ledger, gap));
            }
        }
        if let Some(ts) = prev_ts {
            if p.timestamp < ts {
                out_of_order_count += 1;
                issues.push(format!("Out-of-order at ledger {}: ts {} < prev {}", p.ledger, p.timestamp, ts));
            }
        }
        if std_dev > 0.0 {
            let z = (p.fee as f64 - mean) / std_dev;
            if z.abs() > 3.0 {
                outlier_count += 1;
                issues.push(format!("Outlier at ledger {}: fee={} (z={:.1})", p.ledger, p.fee, z));
            }
        }
        prev_ledger = Some(p.ledger);
        prev_ts = Some(p.timestamp);
    }

    let total_issues = duplicate_count + gap_count + outlier_count + zero_fee_count + out_of_order_count;
    let score = if points.is_empty() {
        0.0
    } else {
        (1.0 - (total_issues as f64 / points.len().max(1) as f64)).max(0.0)
    };

    let min_fee = fees.iter().min().copied().unwrap_or(0);
    let max_fee = fees.iter().max().copied().unwrap_or(0);

    TextQualityReport {
        total_points: points.len(),
        duplicate_count,
        gap_count,
        outlier_count,
        zero_fee_count,
        out_of_order_count,
        min_fee,
        max_fee,
        avg_fee: mean,
        score,
        issues,
    }
}

/// Arguments for the text report subcommand.
pub struct TextReportArgs {
    /// Show detailed issue list.
    pub verbose: bool,
}

impl Default for TextReportArgs {
    fn default() -> Self {
        Self { verbose: false }
    }
}

impl TextReportArgs {
    /// Run the text report subcommand.
    pub fn run(&self, points: &[FeePoint]) {
        let report = generate_text_report(points);
        let output = report.format();
        if self.verbose {
            print!("{}", output);
        } else {
            println!("Score: {:.1}% | {} points | {} issues",
                report.score * 100.0, report.total_points,
                report.duplicate_count + report.gap_count + report.outlier_count + report.zero_fee_count + report.out_of_order_count);
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
            FeePoint { timestamp: 100, fee: 110, ledger: 2, is_spike: false },
            FeePoint { timestamp: 200, fee: 105, ledger: 3, is_spike: false },
        ]
    }

    fn dirty() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 10, fee: 0, ledger: 1, is_spike: false },
            FeePoint { timestamp: 200, fee: 110, ledger: 5, is_spike: false },
            FeePoint { timestamp: 150, fee: 150, ledger: 3, is_spike: false },
        ]
    }

    #[test]
    fn clean_data_high_score() {
        let report = generate_text_report(&sample());
        assert!(report.score > 0.9);
        assert_eq!(report.duplicate_count, 0);
    }

    #[test]
    fn dirty_data_lower_score() {
        let report = generate_text_report(&dirty());
        assert!(report.score < 1.0);
    }

    #[test]
    fn report_format_includes_fields() {
        let report = generate_text_report(&sample());
        let out = report.format();
        assert!(out.contains("Score"));
        assert!(out.contains("Total points"));
    }

    #[test]
    fn empty_data_returns_zero_score() {
        let report = generate_text_report(&[]);
        assert_eq!(report.score, 0.0);
    }

    #[test]
    fn detects_duplicates() {
        let data = vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 10, fee: 200, ledger: 1, is_spike: false },
        ];
        let report = generate_text_report(&data);
        assert_eq!(report.duplicate_count, 1);
    }

    #[test]
    fn detects_zero_fees() {
        let data = vec![
            FeePoint { timestamp: 0, fee: 0, ledger: 1, is_spike: false },
        ];
        let report = generate_text_report(&data);
        assert_eq!(report.zero_fee_count, 1);
    }

    #[test]
    fn text_report_args_default() {
        let args = TextReportArgs::default();
        assert!(!args.verbose);
    }
}

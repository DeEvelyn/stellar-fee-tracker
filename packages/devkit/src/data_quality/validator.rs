//! Data quality validator for fee data.
//!
//! Checks a slice of [`FeePoint`]s for common integrity issues and produces
//! a human-readable or JSON quality report.

use crate::simulation::fee_model::FeePoint;

/// A single validation finding.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationFinding {
    ZeroFees { count: usize },
    DuplicateLedgers { count: usize },
    OutOfOrder { count: usize },
    LedgerGaps { count: usize },
    Outliers { count: usize },
}

impl std::fmt::Display for ValidationFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroFees { count } => write!(f, "zero-fee records: {count}"),
            Self::DuplicateLedgers { count } => write!(f, "duplicate ledger numbers: {count}"),
            Self::OutOfOrder { count } => write!(f, "out-of-order timestamps: {count}"),
            Self::LedgerGaps { count } => write!(f, "ledger sequence gaps: {count}"),
            Self::Outliers { count } => write!(f, "statistical outliers (|z|>3): {count}"),
        }
    }
}

/// Summary report produced by [`Validator::run`].
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub record_count: usize,
    /// Quality score in [0.0, 1.0]. 1.0 means clean.
    pub score: f64,
    pub findings: Vec<ValidationFinding>,
}

impl QualityReport {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn display(&self) -> String {
        let status = if self.is_clean() { "PASS" } else { "FAIL" };
        let mut lines = vec![
            "Data Quality Report".to_string(),
            "===================".to_string(),
            format!("status:  {status}"),
            format!("records: {}", self.record_count),
            format!("score:   {:.2}", self.score),
        ];
        if self.findings.is_empty() {
            lines.push("No issues found.".into());
        } else {
            lines.push(format!("Findings ({}):", self.findings.len()));
            for fi in &self.findings {
                lines.push(format!("  - {fi}"));
            }
        }
        lines.join("\n")
    }

    pub fn to_json(&self) -> String {
        let finding_strs: Vec<String> = self.findings.iter().map(|f| format!("\"{}\"", f)).collect();
        format!(
            r#"{{"record_count":{},"score":{:.2},"clean":{},"findings":[{}]}}"#,
            self.record_count, self.score, self.is_clean(),
            finding_strs.join(","),
        )
    }
}

/// Validates a slice of [`FeePoint`]s and produces a [`QualityReport`].
pub struct Validator;

impl Validator {
    pub fn run(points: &[FeePoint]) -> QualityReport {
        if points.is_empty() {
            return QualityReport { record_count: 0, score: 0.0, findings: vec![] };
        }

        let mut findings = Vec::new();

        let zero_count = points.iter().filter(|p| p.fee == 0).count();
        if zero_count > 0 {
            findings.push(ValidationFinding::ZeroFees { count: zero_count });
        }

        let mut seen = std::collections::BTreeSet::new();
        let dup_count = points.iter().filter(|p| !seen.insert(p.ledger)).count();
        if dup_count > 0 {
            findings.push(ValidationFinding::DuplicateLedgers { count: dup_count });
        }

        let oor_count = points.windows(2).filter(|w| w[1].timestamp < w[0].timestamp).count();
        if oor_count > 0 {
            findings.push(ValidationFinding::OutOfOrder { count: oor_count });
        }

        let gap_count = points.windows(2).filter(|w| w[1].ledger > w[0].ledger + 1).count();
        if gap_count > 0 {
            findings.push(ValidationFinding::LedgerGaps { count: gap_count });
        }

        let fees: Vec<f64> = points.iter().map(|p| p.fee as f64).collect();
        let mean = fees.iter().sum::<f64>() / fees.len() as f64;
        let variance = fees.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / fees.len() as f64;
        let std_dev = variance.sqrt();
        if std_dev > 0.0 {
            let outlier_count = fees.iter().filter(|&&f| ((f - mean) / std_dev).abs() > 3.0).count();
            if outlier_count > 0 {
                findings.push(ValidationFinding::Outliers { count: outlier_count });
            }
        }

        let penalty = findings.len() as f64 * 0.1;
        let score = (1.0_f64 - penalty).max(0.0);

        QualityReport { record_count: points.len(), score, findings }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fee_model::FeePoint;

    fn clean() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0,  fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 5,  fee: 110, ledger: 2, is_spike: false },
            FeePoint { timestamp: 10, fee: 105, ledger: 3, is_spike: false },
        ]
    }

    fn dirty() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0,  fee: 0,   ledger: 1, is_spike: false },
            FeePoint { timestamp: 10, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 5,  fee: 110, ledger: 3, is_spike: false },
        ]
    }

    #[test]
    fn clean_data_passes() {
        let r = Validator::run(&clean());
        assert!(r.is_clean());
        assert!((r.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dirty_data_fails() {
        let r = Validator::run(&dirty());
        assert!(!r.is_clean());
        assert!(r.score < 1.0);
    }

    #[test]
    fn zero_fee_detected() {
        let r = Validator::run(&dirty());
        assert!(r.findings.iter().any(|f| matches!(f, ValidationFinding::ZeroFees { .. })));
    }

    #[test]
    fn duplicate_ledger_detected() {
        let r = Validator::run(&dirty());
        assert!(r.findings.iter().any(|f| matches!(f, ValidationFinding::DuplicateLedgers { .. })));
    }

    #[test]
    fn out_of_order_detected() {
        let r = Validator::run(&dirty());
        assert!(r.findings.iter().any(|f| matches!(f, ValidationFinding::OutOfOrder { .. })));
    }

    #[test]
    fn gap_detected() {
        let r = Validator::run(&dirty());
        assert!(r.findings.iter().any(|f| matches!(f, ValidationFinding::LedgerGaps { .. })));
    }

    #[test]
    fn outlier_detected() {
        let mut data = clean();
        data.push(FeePoint { timestamp: 50, fee: 100_000, ledger: 4, is_spike: true });
        let r = Validator::run(&data);
        assert!(r.findings.iter().any(|f| matches!(f, ValidationFinding::Outliers { .. })));
    }

    #[test]
    fn empty_input() {
        let r = Validator::run(&[]);
        assert_eq!(r.record_count, 0);
    }

    #[test]
    fn display_shows_pass() {
        assert!(Validator::run(&clean()).display().contains("PASS"));
        assert!(Validator::run(&dirty()).display().contains("FAIL"));
    }

    #[test]
    fn json_contains_clean_key() {
        let json = Validator::run(&clean()).to_json();
        assert!(json.contains("\"clean\":true"));
    }
}

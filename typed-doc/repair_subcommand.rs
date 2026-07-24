use crate::simulation::fee_model::FeePoint;
use std::collections::BTreeSet;

/// Identifies and repairs common issues in fee data.
pub struct Repair;

/// Types of issues that can be detected in fee data.
#[derive(Debug, Clone, PartialEq)]
pub enum DataIssue {
    /// Duplicate ledger sequence number.
    DuplicateLedger(u64),
    /// Out-of-order timestamp.
    OutOfOrder { ledger: u64, expected: u64, actual: u64 },
    /// Missing fee value (zero fee).
    ZeroFee(u64),
    /// Abnormally high fee (potential outlier).
    Outlier { ledger: u64, fee: u64, z_score: f64 },
    /// Gap in ledger sequence.
    LedgerGap { from: u64, to: u64, gap_size: u64 },
    /// Negative timestamp.
    NegativeTimestamp(u64),
}

impl std::fmt::Display for DataIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateLedger(l) => write!(f, "duplicate ledger {}", l),
            Self::OutOfOrder { ledger, expected, actual } => {
                write!(f, "out-of-order at ledger {}: expected ts >= {}, got {}", ledger, expected, actual)
            }
            Self::ZeroFee(l) => write!(f, "zero fee at ledger {}", l),
            Self::Outlier { ledger, fee, z_score } => {
                write!(f, "outlier at ledger {}: fee={}, z={:.2}", ledger, fee, z_score)
            }
            Self::LedgerGap { from, to, gap_size } => {
                write!(f, "ledger gap {} -> {} ({} missing)", from, to, gap_size)
            }
            Self::NegativeTimestamp(l) => write!(f, "negative timestamp at ledger {}", l),
        }
    }
}

/// A repair action that can be applied to fee data.
#[derive(Debug, Clone)]
pub enum RepairAction {
    /// Remove a duplicate point.
    RemoveDuplicate(u64),
    /// Reorder points by timestamp.
    Reorder,
    /// Replace a zero fee with the mean of neighbors.
    FillZeroFee { ledger: u64, replacement: u64 },
    /// Cap an outlier fee to a threshold.
    CapOutlier { ledger: u64, original: u64, capped: u64 },
    /// Interpolate missing ledgers.
    InterpolateGap { from: u64, to: u64, count: u64 },
}

impl Repair {
    /// Detect all issues in a set of fee points.
    pub fn detect(points: &[FeePoint]) -> Vec<DataIssue> {
        let mut issues = Vec::new();

        let mut seen_ledgers = BTreeSet::new();
        let mut prev_ts: Option<u64> = None;
        let mut prev_ledger: Option<u64> = None;

        let fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
        let mean = fees.iter().sum::<u64>() as f64 / fees.len().max(1) as f64;
        let std_dev = {
            let variance = fees.iter().map(|f| (*f as f64 - mean).powi(2)).sum::<f64>() / fees.len().max(1) as f64;
            variance.sqrt()
        };

        for p in points {
            if p.fee == 0 {
                issues.push(DataIssue::ZeroFee(p.ledger));
            }
            if !seen_ledgers.insert(p.ledger) {
                issues.push(DataIssue::DuplicateLedger(p.ledger));
            }
            if let Some(ts) = prev_ts {
                if p.timestamp < ts {
                    issues.push(DataIssue::OutOfOrder {
                        ledger: p.ledger,
                        expected: ts,
                        actual: p.timestamp,
                    });
                }
            }
            if let Some(pl) = prev_ledger {
                if p.ledger > pl + 1 {
                    issues.push(DataIssue::LedgerGap {
                        from: pl,
                        to: p.ledger,
                        gap_size: p.ledger - pl - 1,
                    });
                }
            }
            if std_dev > 0.0 {
                let z = (p.fee as f64 - mean) / std_dev;
                if z.abs() > 3.0 {
                    issues.push(DataIssue::Outlier {
                        ledger: p.ledger,
                        fee: p.fee,
                        z_score: z,
                    });
                }
            }
            if p.timestamp > 1_000_000_000_000 {
                issues.push(DataIssue::NegativeTimestamp(p.ledger));
            }
            prev_ts = Some(p.timestamp);
            prev_ledger = Some(p.ledger);
        }
        issues
    }

    /// Generate repair actions for the detected issues.
    pub fn plan(points: &[FeePoint]) -> Vec<RepairAction> {
        let issues = Self::detect(points);
        let mut actions = Vec::new();

        for issue in &issues {
            match issue {
                DataIssue::DuplicateLedger(l) => {
                    actions.push(RepairAction::RemoveDuplicate(*l));
                }
                DataIssue::OutOfOrder { .. } => {
                    if !actions.iter().any(|a| matches!(a, RepairAction::Reorder)) {
                        actions.push(RepairAction::Reorder);
                    }
                }
                DataIssue::ZeroFee(ledger) => {
                    let neighbors: Vec<u64> = points
                        .iter()
                        .filter(|p| p.ledger != *ledger && p.fee > 0)
                        .map(|p| p.fee)
                        .collect();
                    let replacement = if neighbors.is_empty() {
                        100
                    } else {
                        neighbors.iter().sum::<u64>() / neighbors.len() as u64
                    };
                    actions.push(RepairAction::FillZeroFee {
                        ledger: *ledger,
                        replacement,
                    });
                }
                DataIssue::Outlier { ledger, fee, .. } => {
                    actions.push(RepairAction::CapOutlier {
                        ledger: *ledger,
                        original: *fee,
                        capped: (mean(points) * 2.0) as u64,
                    });
                }
                DataIssue::LedgerGap { from, to, count } => {
                    actions.push(RepairAction::InterpolateGap {
                        from: *from,
                        to: *to,
                        count: *count,
                    });
                }
                DataIssue::NegativeTimestamp(ledger) => {
                    actions.push(RepairAction::FillZeroFee {
                        ledger: *ledger,
                        replacement: 0,
                    });
                }
            }
        }
        actions
    }

    /// Apply repairs to fee points and return the cleaned result.
    pub fn apply(points: &[FeePoint], dry_run: bool) -> (Vec<FeePoint>, Vec<RepairAction>) {
        let actions = Self::plan(points);
        if dry_run {
            return (points.to_vec(), actions);
        }

        let mut cleaned: Vec<FeePoint> = points.to_vec();
        let mut seen_ledgers = BTreeSet::new();

        cleaned.retain(|p| {
            if seen_ledgers.contains(&p.ledger) {
                return false;
            }
            seen_ledgers.insert(p.ledger);
            true
        });

        cleaned.sort_by_key(|p| (p.timestamp, p.ledger));

        for action in &actions {
            match action {
                RepairAction::FillZeroFee { ledger, replacement } => {
                    if let Some(pt) = cleaned.iter_mut().find(|p| p.ledger == *ledger) {
                        pt.fee = *replacement;
                    }
                }
                RepairAction::CapOutlier { ledger, capped, .. } => {
                    if let Some(pt) = cleaned.iter_mut().find(|p| p.ledger == *ledger) {
                        if pt.fee > *capped {
                            pt.fee = *capped;
                        }
                    }
                }
                _ => {}
            }
        }

        (cleaned, actions)
    }
}

fn mean(points: &[FeePoint]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    points.iter().map(|p| p.fee as f64).sum::<f64>() / points.len() as f64
}

/// Arguments for the `repair` subcommand.
pub struct RepairArgs {
    /// Perform a dry run without modifying data.
    pub dry_run: bool,
    /// Output the repair report as JSON.
    pub json: bool,
    /// Only show issues, do not repair.
    pub check_only: bool,
}

impl Default for RepairArgs {
    fn default() -> Self {
        Self {
            dry_run: true,
            json: false,
            check_only: false,
        }
    }
}

impl RepairArgs {
    /// Run the repair subcommand on a set of fee points.
    pub fn run(&self, points: &[FeePoint]) {
        let issues = Repair::detect(points);

        if self.check_only {
            if self.json {
                let issue_strs: Vec<String> = issues.iter().map(|i| format!("\"{}\"", i)).collect();
                println!("[{}]", issue_strs.join(","));
            } else {
                println!("Found {} issue(s):", issues.len());
                for issue in &issues {
                    println!("  - {}", issue);
                }
            }
            return;
        }

        let (cleaned, actions) = Repair::apply(points, self.dry_run);

        if self.json {
            println!(
                r#"{{"issues_found":{},"actions_taken":{},"points_before":{},"points_after":{}}}"#,
                issues.len(),
                actions.len(),
                points.len(),
                cleaned.len(),
            );
        } else {
            println!("Repair report:");
            println!("  Issues found:  {}", issues.len());
            println!("  Actions:       {}", actions.len());
            println!("  Points before: {}", points.len());
            println!("  Points after:  {}", cleaned.len());
            if self.dry_run {
                println!("  (dry run — no changes applied)");
            }
            if !actions.is_empty() {
                println!("\nActions:");
                for action in &actions {
                    println!("  - {:?}", action);
                }
            }
        }
    }

    /// Get the quality score [0.0, 1.0] for a data set.
    pub fn quality_score(points: &[FeePoint]) -> f64 {
        if points.is_empty() {
            return 0.0;
        }
        let issues = Repair::detect(points);
        let penalty = issues.len() as f64 * 0.1;
        (1.0 - penalty).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fee_model::FeePoint;

    fn clean_data() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 110, ledger: 2, is_spike: false },
            FeePoint { timestamp: 200, fee: 105, ledger: 3, is_spike: false },
        ]
    }

    fn dirty_data() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 50, fee: 0, ledger: 1, is_spike: false },    // duplicate + zero fee
            FeePoint { timestamp: 200, fee: 110, ledger: 3, is_spike: false },
            FeePoint { timestamp: 150, fee: 150, ledger: 2, is_spike: false },  // out-of-order
            FeePoint { timestamp: 300, fee: 999_999, ledger: 5, is_spike: true }, // outlier
        ]
    }

    #[test]
    fn detect_clean_data_returns_no_issues() {
        let issues = Repair::detect(&clean_data());
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn detect_dirty_data_finds_issues() {
        let issues = Repair::detect(&dirty_data());
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| matches!(i, DataIssue::ZeroFee(_))));
        assert!(issues.iter().any(|i| matches!(i, DataIssue::DuplicateLedger(_))));
    }

    #[test]
    fn repair_plan_generates_actions() {
        let actions = Repair::plan(&dirty_data());
        assert!(!actions.is_empty());
    }

    #[test]
    fn repair_apply_removes_duplicates() {
        let (cleaned, _) = Repair::apply(&dirty_data(), false);
        let ledgers: Vec<u64> = cleaned.iter().map(|p| p.ledger).collect();
        let unique: BTreeSet<u64> = ledgers.clone().into_iter().collect();
        assert_eq!(ledgers.len(), unique.len());
    }

    #[test]
    fn quality_score_perfect_for_clean() {
        let score = RepairArgs::quality_score(&clean_data());
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn quality_score_reduced_for_dirty() {
        let score = RepairArgs::quality_score(&dirty_data());
        assert!(score < 1.0);
    }

    #[test]
    fn quality_score_zero_for_empty() {
        let score = RepairArgs::quality_score(&[]);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn detect_outlier_high_z_score() {
        let data = vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 110, ledger: 2, is_spike: false },
            FeePoint { timestamp: 200, fee: 105, ledger: 3, is_spike: false },
            FeePoint { timestamp: 300, fee: 10_000, ledger: 4, is_spike: true },
        ];
        let issues = Repair::detect(&data);
        assert!(issues.iter().any(|i| matches!(i, DataIssue::Outlier { .. })));
    }

    #[test]
    fn repair_args_default_is_dry_run() {
        let args = RepairArgs::default();
        assert!(args.dry_run);
    }

    #[test]
    fn detect_ledger_gap() {
        let data = vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 200, fee: 110, ledger: 5, is_spike: false },
        ];
        let issues = Repair::detect(&data);
        assert!(issues.iter().any(|i| matches!(i, DataIssue::LedgerGap { .. })));
    }

    #[test]
    fn issue_display_format() {
        let issue = DataIssue::DuplicateLedger(5);
        assert_eq!(format!("{}", issue), "duplicate ledger 5");
    }
}

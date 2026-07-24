use crate::simulation::fee_model::FeePoint;
use std::collections::BTreeSet;

/// Removes duplicate ledger entries from fee sequences.
pub struct DuplicateRemover;

impl DuplicateRemover {
    /// Remove duplicate ledger entries, keeping the first occurrence.
    pub fn remove_duplicates(points: &[FeePoint]) -> Vec<FeePoint> {
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        for p in points {
            if seen.insert(p.ledger) {
                result.push(p.clone());
            }
        }
        result
    }

    /// Remove duplicate ledger entries, keeping the last occurrence.
    pub fn remove_duplicates_keep_last(points: &[FeePoint]) -> Vec<FeePoint> {
        let mut seen = BTreeSet::new();
        let mut result: Vec<FeePoint> = points.iter().rev().filter(|p| seen.insert(p.ledger)).cloned().collect();
        result.reverse();
        result
    }

    /// Remove duplicates and return the removed points.
    pub fn removed_points<'a>(points: &'a [FeePoint]) -> Vec<&'a FeePoint> {
        let mut seen = BTreeSet::new();
        let mut removed = Vec::new();
        for p in points {
            if !seen.insert(p.ledger) {
                removed.push(p);
            }
        }
        removed
    }

    /// Count duplicate entries.
    pub fn duplicate_count(points: &[FeePoint]) -> usize {
        let unique: BTreeSet<u64> = points.iter().map(|p| p.ledger).collect();
        points.len().saturating_sub(unique.len())
    }

    /// Merge two fee sequences, keeping the first occurrence of each ledger.
    pub fn merge(a: &[FeePoint], b: &[FeePoint]) -> Vec<FeePoint> {
        let mut combined: Vec<FeePoint> = a.to_vec();
        combined.extend_from_slice(b);
        Self::remove_duplicates(&combined)
    }

    /// Return a report of duplicates found.
    pub fn report(points: &[FeePoint]) -> String {
        let dup_count = Self::duplicate_count(points);
        let mut out = format!("Duplicate removal report\n");
        out.push_str(&format!("========================\n"));
        out.push_str(&format!("Total points:  {}\n", points.len()));
        out.push_str(&format!("Duplicates:    {}\n", dup_count));
        out.push_str(&format!("After removal: {}\n", points.len() - dup_count));

        if dup_count > 0 {
            let removed = Self::removed_points(points);
            out.push_str("Removed:\n");
            for p in removed {
                out.push_str(&format!("  ledger {} | fee {} | ts {}\n", p.ledger, p.fee, p.timestamp));
            }
        }
        out
    }
}

/// Arguments for the dedup subcommand.
pub struct DedupArgs {
    /// Keep the last occurrence instead of the first.
    pub keep_last: bool,
    /// Show a detailed report.
    pub verbose: bool,
}

impl Default for DedupArgs {
    fn default() -> Self {
        Self {
            keep_last: false,
            verbose: false,
        }
    }
}

impl DedupArgs {
    /// Run the dedup subcommand.
    pub fn run(&self, points: &[FeePoint]) {
        let cleaned = if self.keep_last {
            DuplicateRemover::remove_duplicates_keep_last(points)
        } else {
            DuplicateRemover::remove_duplicates(points)
        };

        if self.verbose {
            println!("{}", DuplicateRemover::report(points));
        } else {
            let removed = DuplicateRemover::duplicate_count(points);
            println!("{} points → {} points ({} duplicates removed)",
                points.len(), cleaned.len(), removed);
        }
    }

    /// Validate that duplicate removal would produce a different result.
    pub fn has_duplicates(points: &[FeePoint]) -> bool {
        DuplicateRemover::duplicate_count(points) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fee_model::FeePoint;

    fn with_duplicates() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 10, fee: 150, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 200, ledger: 2, is_spike: false },
            FeePoint { timestamp: 200, fee: 250, ledger: 2, is_spike: false },
            FeePoint { timestamp: 300, fee: 300, ledger: 3, is_spike: false },
        ]
    }

    #[test]
    fn remove_duplicates_keeps_first_occurrence() {
        let cleaned = DuplicateRemover::remove_duplicates(&with_duplicates());
        assert_eq!(cleaned.len(), 3);
        assert_eq!(cleaned[0].fee, 100); // first occurrence of ledger 1
        assert_eq!(cleaned[1].fee, 200); // first occurrence of ledger 2
    }

    #[test]
    fn remove_duplicates_keep_last() {
        let cleaned = DuplicateRemover::remove_duplicates_keep_last(&with_duplicates());
        assert_eq!(cleaned.len(), 3);
        assert_eq!(cleaned[0].fee, 150); // last occurrence of ledger 1
        assert_eq!(cleaned[1].fee, 250); // last occurrence of ledger 2
    }

    #[test]
    fn removed_points_returns_duplicates() {
        let removed = DuplicateRemover::removed_points(&with_duplicates());
        assert_eq!(removed.len(), 2);
        assert_eq!(removed[0].fee, 150);
        assert_eq!(removed[1].fee, 250);
    }

    #[test]
    fn duplicate_count() {
        assert_eq!(DuplicateRemover::duplicate_count(&with_duplicates()), 2);
    }

    #[test]
    fn no_duplicates_returns_same() {
        let data = vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 200, ledger: 2, is_spike: false },
        ];
        let cleaned = DuplicateRemover::remove_duplicates(&data);
        assert_eq!(cleaned.len(), 2);
        assert_eq!(DuplicateRemover::duplicate_count(&data), 0);
    }

    #[test]
    fn merge_combines_and_deduplicates() {
        let a = vec![FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false }];
        let b = vec![FeePoint { timestamp: 10, fee: 150, ledger: 1, is_spike: false }];
        let merged = DuplicateRemover::merge(&a, &b);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn report_includes_counts() {
        let report = DuplicateRemover::report(&with_duplicates());
        assert!(report.contains("Duplicates:"));
        assert!(report.contains("2"));
    }

    #[test]
    fn empty_input() {
        let cleaned = DuplicateRemover::remove_duplicates(&[]);
        assert!(cleaned.is_empty());
        assert_eq!(DuplicateRemover::duplicate_count(&[]), 0);
    }

    #[test]
    fn has_duplicates_check() {
        assert!(DedupArgs::has_duplicates(&with_duplicates()));
        assert!(!DedupArgs::has_duplicates(&[]));
    }

    #[test]
    fn dedup_args_default() {
        let args = DedupArgs::default();
        assert!(!args.keep_last);
        assert!(!args.verbose);
    }

    #[test]
    fn single_point_no_issues() {
        let data = vec![FeePoint { timestamp: 0, fee: 100, ledger: 5, is_spike: false }];
        assert_eq!(DuplicateRemover::duplicate_count(&data), 0);
    }
}

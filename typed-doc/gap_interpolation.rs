use crate::simulation::fee_model::FeePoint;

/// Interpolates missing fee values in ledger sequences.
pub struct GapInterpolator;

impl GapInterpolator {
    /// Fill gaps in a ledger sequence by interpolating fee values.
    /// Expects points sorted by ledger sequence number.
    pub fn fill_gaps(points: &[FeePoint]) -> Vec<FeePoint> {
        if points.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut prev: Option<&FeePoint> = None;

        for point in points {
            if let Some(prev_pt) = prev {
                let gap = point.ledger.saturating_sub(prev_pt.ledger);
                if gap > 1 {
                    let ts_step = (point.timestamp - prev_pt.timestamp) as f64 / gap as f64;
                    let fee_step = (point.fee as f64 - prev_pt.fee as f64) / gap as f64;

                    for i in 1..gap {
                        let interp_ledger = prev_pt.ledger + i;
                        let interp_ts = (prev_pt.timestamp as f64 + ts_step * i as f64).round() as u64;
                        let interp_fee = (prev_pt.fee as f64 + fee_step * i as f64).round() as u64;

                        result.push(FeePoint {
                            timestamp: interp_ts,
                            fee: interp_fee,
                            ledger: interp_ledger,
                            is_spike: false,
                        });
                    }
                }
            }

            result.push(point.clone());
            prev = Some(point);
        }

        result
    }

    /// Fill gaps and return only the interpolated points.
    pub fn interpolated_only(points: &[FeePoint]) -> Vec<FeePoint> {
        let filled = Self::fill_gaps(points);
        let original: std::collections::BTreeSet<u64> =
            points.iter().map(|p| p.ledger).collect();
        filled
            .into_iter()
            .filter(|p| !original.contains(&p.ledger))
            .collect()
    }

    /// Count the number of missing ledgers between the first and last point.
    pub fn gap_count(points: &[FeePoint]) -> u64 {
        if points.len() < 2 {
            return 0;
        }
        let first = points.first().unwrap().ledger;
        let last = points.last().unwrap().ledger;
        let expected = (last - first + 1) as usize;
        expected.saturating_sub(points.len()) as u64
    }

    /// Return the total time span covered by the ledger sequence.
    pub fn time_span(points: &[FeePoint]) -> u64 {
        if points.len() < 2 {
            return 0;
        }
        let first_ts = points.first().unwrap().timestamp;
        let last_ts = points.last().unwrap().timestamp;
        last_ts.saturating_sub(first_ts)
    }
}

/// Arguments for the gap-fill subcommand.
pub struct GapFillArgs {
    /// Dry-run: show what would be interpolated without modifying.
    pub dry_run: bool,
    /// Show interpolated points only.
    pub show_interpolated: bool,
}

impl Default for GapFillArgs {
    fn default() -> Self {
        Self {
            dry_run: false,
            show_interpolated: false,
        }
    }
}

impl GapFillArgs {
    /// Run the gap-fill subcommand.
    pub fn run(&self, points: &[FeePoint]) {
        let gaps = GapInterpolator::gap_count(points);
        let filled = GapInterpolator::fill_gaps(points);
        let interpolated = GapInterpolator::interpolated_only(points);

        eprintln!("Ledger gap analysis:");
        eprintln!("  Original points:  {}", points.len());
        eprintln!("  Gaps detected:    {}", gaps);
        eprintln!("  After fill:       {}", filled.len());
        eprintln!("  Interpolated:     {}", interpolated.len());

        if self.show_interpolated {
            for p in &interpolated {
                println!("  ledger {} | fee {} | ts {}", p.ledger, p.fee, p.timestamp);
            }
        }

        if self.dry_run {
            eprintln!("  (dry-run — no changes applied)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fee_model::FeePoint;

    fn with_gaps() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 200, fee: 200, ledger: 3, is_spike: false },
            FeePoint { timestamp: 400, fee: 300, ledger: 5, is_spike: false },
        ]
    }

    #[test]
    fn fill_gaps_interpolates_missing_ledgers() {
        let filled = GapInterpolator::fill_gaps(&with_gaps());
        assert_eq!(filled.len(), 5); // 1, 2, 3, 4, 5
        assert_eq!(filled[0].ledger, 1);
        assert_eq!(filled[1].ledger, 2);
        assert_eq!(filled[2].ledger, 3);
        assert_eq!(filled[3].ledger, 4);
        assert_eq!(filled[4].ledger, 5);
    }

    #[test]
    fn interpolated_values_are_between_endpoints() {
        let filled = GapInterpolator::fill_gaps(&with_gaps());
        assert!(filled[1].fee > 100 && filled[1].fee < 200);
        assert!(filled[3].fee > 200 && filled[3].fee < 300);
    }

    #[test]
    fn interpolated_only_returns_new_points() {
        let interp = GapInterpolator::interpolated_only(&with_gaps());
        assert_eq!(interp.len(), 2);
        assert_eq!(interp[0].ledger, 2);
        assert_eq!(interp[1].ledger, 4);
    }

    #[test]
    fn gap_count_returns_correct_number() {
        let count = GapInterpolator::gap_count(&with_gaps());
        assert_eq!(count, 2);
    }

    #[test]
    fn no_gaps_returns_same_points() {
        let data = vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 200, ledger: 2, is_spike: false },
        ];
        let filled = GapInterpolator::fill_gaps(&data);
        assert_eq!(filled.len(), 2);
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(GapInterpolator::fill_gaps(&[]).is_empty());
    }

    #[test]
    fn single_point_no_fill() {
        let data = vec![FeePoint { timestamp: 0, fee: 100, ledger: 5, is_spike: false }];
        let filled = GapInterpolator::fill_gaps(&data);
        assert_eq!(filled.len(), 1);
    }

    #[test]
    fn time_span_calculation() {
        let span = GapInterpolator::time_span(&with_gaps());
        assert_eq!(span, 400);
    }

    #[test]
    fn gap_count_single_point_zero() {
        let data = vec![FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false }];
        assert_eq!(GapInterpolator::gap_count(&data), 0);
    }

    #[test]
    fn linear_interpolation_fees() {
        let data = vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 200, ledger: 3, is_spike: false },
        ];
        let filled = GapInterpolator::fill_gaps(&data);
        assert_eq!(filled[1].ledger, 2);
        assert_eq!(filled[1].fee, 150);
    }
}

use crate::simulation::fee_model::FeePoint;

/// Statistical summary for a set of fee points.
#[derive(Debug, Clone)]
pub struct FeeSummary {
    /// Number of data points.
    pub count: usize,
    /// Minimum fee value.
    pub min: u64,
    /// Maximum fee value.
    pub max: u64,
    /// Mean fee value.
    pub mean: f64,
    /// Median fee value.
    pub median: u64,
    /// Standard deviation.
    pub std_dev: f64,
    /// Number of spike events.
    pub spike_count: usize,
    /// Fee at the 25th percentile.
    pub p25: u64,
    /// Fee at the 75th percentile.
    pub p75: u64,
    /// Fee at the 95th percentile.
    pub p95: u64,
    /// Fee at the 99th percentile.
    pub p99: u64,
}

impl FeeSummary {
    /// Compute a summary from a slice of FeePoints.
    pub fn from_points(points: &[FeePoint]) -> Self {
        if points.is_empty() {
            return Self {
                count: 0,
                min: 0,
                max: 0,
                mean: 0.0,
                median: 0,
                std_dev: 0.0,
                spike_count: 0,
                p25: 0,
                p75: 0,
                p95: 0,
                p99: 0,
            };
        }

        let mut fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
        fees.sort_unstable();
        let count = fees.len();
        let min = fees[0];
        let max = fees[count - 1];
        let sum: u64 = fees.iter().sum();
        let mean = sum as f64 / count as f64;
        let median = fees[count / 2];
        let variance: f64 = fees.iter().map(|f| (*f as f64 - mean).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();
        let spike_count = points.iter().filter(|p| p.is_spike).count();

        Self {
            count,
            min,
            max,
            mean,
            median,
            std_dev,
            spike_count,
            p25: fees[(count as f64 * 0.25) as usize],
            p75: fees[(count as f64 * 0.75) as usize],
            p95: fees[(count as f64 * 0.95) as usize],
            p99: fees[(count as f64 * 0.99) as usize],
        }
    }

    /// Display the summary as a formatted report.
    pub fn display(&self) -> String {
        format!(
            "Fee Summary\n\
             ===========\n\
             count:       {}\n\
             min:         {} stroops\n\
             max:         {} stroops\n\
             mean:        {:.2} stroops\n\
             median:      {} stroops\n\
             std_dev:     {:.2}\n\
             spike_count: {}\n\
             p25:         {} stroops\n\
             p75:         {} stroops\n\
             p95:         {} stroops\n\
             p99:         {} stroops",
            self.count,
            self.min,
            self.max,
            self.mean,
            self.median,
            self.std_dev,
            self.spike_count,
            self.p25,
            self.p75,
            self.p95,
            self.p99,
        )
    }

    /// Output summary as JSON.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"count":{},"min":{},"max":{},"mean":{:.2},"median":{},"std_dev":{:.2},"spike_count":{},"p25":{},"p75":{},"p95":{},"p99":{}}}"#,
            self.count, self.min, self.max, self.mean, self.median,
            self.std_dev, self.spike_count, self.p25, self.p75, self.p95, self.p99,
        )
    }
}

/// Fee point inspector for detailed examination of fee data.
pub struct Inspector;

impl Inspector {
    /// Find fee points above a given threshold.
    pub fn above_threshold(points: &[FeePoint], threshold: u64) -> Vec<&FeePoint> {
        points.iter().filter(|p| p.fee > threshold).collect()
    }

    /// Find fee points within a time range.
    pub fn in_time_range<'a>(points: &'a [FeePoint], start: u64, end: u64) -> Vec<&'a FeePoint> {
        points.iter().filter(|p| p.timestamp >= start && p.timestamp <= end).collect()
    }

    /// Find the top-N largest fees.
    pub fn top_n(points: &[FeePoint], n: usize) -> Vec<&FeePoint> {
        let mut sorted: Vec<&FeePoint> = points.iter().collect();
        sorted.sort_by(|a, b| b.fee.cmp(&a.fee));
        sorted.truncate(n);
        sorted
    }

    /// Return consecutive runs of spike events.
    pub fn spike_runs(points: &[FeePoint]) -> Vec<Vec<&FeePoint>> {
        let mut runs: Vec<Vec<&FeePoint>> = Vec::new();
        let mut current: Vec<&FeePoint> = Vec::new();
        for p in points {
            if p.is_spike {
                current.push(p);
            } else if !current.is_empty() {
                runs.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            runs.push(current);
        }
        runs
    }

    /// Compute the gap (in seconds) between consecutive fee points.
    pub fn gaps(points: &[FeePoint]) -> Vec<u64> {
        points.windows(2).map(|w| w[1].timestamp.saturating_sub(w[0].timestamp)).collect()
    }
}

/// Arguments for the `inspect` subcommand.
pub struct InspectArgs {
    /// Show a full fee summary report.
    pub summary: bool,
    /// Filter points above this fee threshold.
    pub above: Option<u64>,
    /// Show the top-N highest fees.
    pub top: Option<usize>,
    /// Show spike runs.
    pub spike_runs: bool,
    /// Output as JSON.
    pub json: bool,
}

impl Default for InspectArgs {
    fn default() -> Self {
        Self {
            summary: true,
            above: None,
            top: None,
            spike_runs: false,
            json: false,
        }
    }
}

impl InspectArgs {
    /// Run the inspect subcommand with a set of fee points.
    pub fn run(&self, points: &[FeePoint]) {
        if self.summary {
            let summary = FeeSummary::from_points(points);
            if self.json {
                println!("{}", summary.to_json());
            } else {
                println!("{}", summary.display());
            }
        }
        if let Some(threshold) = self.above {
            let high = Inspector::above_threshold(points, threshold);
            if self.json {
                let ids: Vec<u64> = high.iter().map(|p| p.ledger).collect();
                println!("{}", serde_json::to_string(&ids).unwrap_or_default());
            } else {
                println!("Points above {} stroops: {}", threshold, high.len());
                for p in high {
                    println!("  ledger {} | fee {} | ts {}", p.ledger, p.fee, p.timestamp);
                }
            }
        }
        if let Some(n) = self.top {
            let top = Inspector::top_n(points, n);
            if self.json {
                let ids: Vec<u64> = top.iter().map(|p| p.ledger).collect();
                println!("{}", serde_json::to_string(&ids).unwrap_or_default());
            } else {
                println!("Top {} fees:", n);
                for p in top {
                    println!("  {} stroops (ledger {})", p.fee, p.ledger);
                }
            }
        }
        if self.spike_runs {
            let runs = Inspector::spike_runs(points);
            println!("Spike runs: {}", runs.len());
            for (i, run) in runs.iter().enumerate() {
                println!("  run {}: {} spikes from ledger {}", i + 1, run.len(), run[0].ledger);
            }
        }
    }

    /// Validate that the fee data is not empty.
    pub fn validate_data(points: &[FeePoint]) -> Result<(), String> {
        if points.is_empty() {
            Err("no fee points to inspect".into())
        } else {
            Ok(())
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
            FeePoint { timestamp: 200, fee: 500, ledger: 3, is_spike: true },
            FeePoint { timestamp: 300, fee: 150, ledger: 4, is_spike: false },
            FeePoint { timestamp: 400, fee: 600, ledger: 5, is_spike: true },
        ]
    }

    #[test]
    fn fee_summary_computes_correctly() {
        let s = FeeSummary::from_points(&sample());
        assert_eq!(s.count, 5);
        assert_eq!(s.min, 100);
        assert_eq!(s.max, 600);
        assert_eq!(s.spike_count, 2);
    }

    #[test]
    fn fee_summary_empty_returns_zeros() {
        let s = FeeSummary::from_points(&[]);
        assert_eq!(s.count, 0);
    }

    #[test]
    fn inspect_above_threshold_filters() {
        let high = Inspector::above_threshold(&sample(), 300);
        assert_eq!(high.len(), 2);
    }

    #[test]
    fn inspect_in_time_range() {
        let range = Inspector::in_time_range(&sample(), 100, 300);
        assert_eq!(range.len(), 3);
    }

    #[test]
    fn inspect_top_n() {
        let top = Inspector::top_n(&sample(), 2);
        assert_eq!(top[0].fee, 600);
        assert_eq!(top[1].fee, 500);
    }

    #[test]
    fn inspect_spike_runs() {
        let runs = Inspector::spike_runs(&sample());
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn inspect_gaps() {
        let g = Inspector::gaps(&sample());
        assert_eq!(g, vec![100, 100, 100, 100]);
    }

    #[test]
    fn summary_json_is_valid() {
        let s = FeeSummary::from_points(&sample());
        let json = s.to_json();
        assert!(json.contains("\"count\":5"));
    }

    #[test]
    fn validate_data_rejects_empty() {
        assert!(InspectArgs::validate_data(&[]).is_err());
    }

    #[test]
    fn summary_display_contains_fields() {
        let s = FeeSummary::from_points(&sample());
        let out = s.display();
        assert!(out.contains("count"));
        assert!(out.contains("spike_count"));
    }
}

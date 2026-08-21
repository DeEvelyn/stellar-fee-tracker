use crate::analysis::percentile::Percentile;

/// Severity level of a fee spike relative to baseline.
#[derive(Debug, Clone, PartialEq)]
pub enum SpikeSeverity {
    /// 2–5× baseline
    Low,
    /// 5–10× baseline
    Medium,
    /// 10–50× baseline
    High,
    /// >50× baseline
    Critical,
}

/// A detected spike event with severity and duration (consecutive-ledger model).
#[derive(Debug, Clone, PartialEq)]
pub struct SpikeEvent {
    pub severity: SpikeSeverity,
    /// Number of consecutive ledgers the spike persisted.
    pub duration_ledgers: usize,
}

/// A detected spike event carrying full time-series context (timestamped model).
#[derive(Debug, Clone)]
pub struct TimestampedSpikeEvent {
    pub timestamp: u64,
    pub fee_amount: u64,
    pub baseline: u64,
    pub multiplier: f64,
    pub severity: SpikeSeverity,
}

/// Classifies fee spikes in a time series.
pub struct SpikeClassifier {
    baseline: u64,
}

impl Default for SpikeClassifier {
    fn default() -> Self {
        Self { baseline: 200 }
    }
}

impl SpikeClassifier {
    /// Create a classifier with the given baseline.
    pub fn new(baseline: u64) -> Self {
        Self { baseline }
    }

    /// Classify a single fee using the configured baseline.
    ///
    /// Returns [`SpikeSeverity::Low`] for fees below the 2× spike threshold.
    pub fn classify(&self, fee: u64) -> SpikeSeverity {
        if self.baseline == 0 {
            return SpikeSeverity::Low;
        }
        let ratio = fee as f64 / self.baseline as f64;
        if ratio > 50.0 {
            SpikeSeverity::Critical
        } else if ratio >= 10.0 {
            SpikeSeverity::High
        } else if ratio >= 5.0 {
            SpikeSeverity::Medium
        } else {
            SpikeSeverity::Low
        }
    }

    /// Returns indices of fees that fall outside 1.5 × IQR from Q1/Q3.
    /// `fees` need not be sorted; indices refer to the original slice.
    pub fn iqr_outliers(fees: &[u64]) -> Vec<usize> {
        if fees.len() < 4 {
            return vec![];
        }
        let mut sorted = fees.to_vec();
        sorted.sort_unstable();
        let q1 = Percentile::nearest_rank(&sorted, 25) as f64;
        let q3 = Percentile::nearest_rank(&sorted, 75) as f64;
        let iqr = q3 - q1;
        let lower = q1 - 1.5 * iqr;
        let upper = q3 + 1.5 * iqr;
        fees.iter()
            .enumerate()
            .filter(|(_, &v)| (v as f64) < lower || (v as f64) > upper)
            .map(|(i, _)| i)
            .collect()
    }

    /// Classify a single fee against a baseline.
    /// Returns `None` if the fee is below the 2× spike threshold.
    pub fn classify_with_baseline(fee: u64, baseline: u64) -> Option<SpikeSeverity> {
        if baseline == 0 {
            return None;
        }
        let ratio = fee as f64 / baseline as f64;
        match ratio {
            r if r > 50.0 => Some(SpikeSeverity::Critical),
            r if r >= 10.0 => Some(SpikeSeverity::High),
            r if r >= 5.0 => Some(SpikeSeverity::Medium),
            r if r >= 2.0 => Some(SpikeSeverity::Low),
            _ => None,
        }
    }

    /// Detect all spike events in a fee sequence given a fixed baseline.
    /// Consecutive ledgers above the 2× threshold are grouped into one `SpikeEvent`.
    pub fn detect(fees: &[u64], baseline: u64) -> Vec<SpikeEvent> {
        let mut events = Vec::new();
        let mut i = 0;
        while i < fees.len() {
            if let Some(severity) = Self::classify_with_baseline(fees[i], baseline) {
                let start = i;
                while i < fees.len() && Self::classify_with_baseline(fees[i], baseline).is_some() {
                    i += 1;
                }
                let severity = fees[start..i]
                    .iter()
                    .filter_map(|&f| Self::classify_with_baseline(f, baseline))
                    .max_by_key(|s| match s {
                        SpikeSeverity::Low => 1,
                        SpikeSeverity::Medium => 2,
                        SpikeSeverity::High => 3,
                        SpikeSeverity::Critical => 4,
                    })
                    .unwrap_or(severity);
                events.push(SpikeEvent {
                    severity,
                    duration_ledgers: i - start,
                });
            } else {
                i += 1;
            }
        }
        events
    }

    /// Detect spikes in a `(timestamp, fee)` slice where `fee > baseline * threshold`.
    /// Returns a `TimestampedSpikeEvent` per entry that exceeds the threshold.
    pub fn detect_with_threshold(
        fees: &[(u64, u64)],
        baseline: u64,
        threshold: f64,
    ) -> Vec<TimestampedSpikeEvent> {
        fees.iter()
            .filter_map(|&(timestamp, fee_amount)| {
                let multiplier = fee_amount as f64 / baseline as f64;
                if multiplier > threshold {
                    let severity = if multiplier < 2.0 {
                        SpikeSeverity::Low
                    } else if multiplier < 5.0 {
                        SpikeSeverity::Medium
                    } else if multiplier < 10.0 {
                        SpikeSeverity::High
                    } else {
                        SpikeSeverity::Critical
                    };
                    Some(TimestampedSpikeEvent {
                        timestamp,
                        fee_amount,
                        baseline,
                        multiplier,
                        severity,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iqr_outliers_detects_spike() {
        let fees = [100u64, 102, 98, 101, 99, 500, 103, 97];
        let outliers = SpikeClassifier::iqr_outliers(&fees);
        assert!(outliers.contains(&5), "500 should be flagged");
    }

    #[test]
    fn iqr_outliers_no_outliers() {
        let fees = [10u64, 11, 12, 13, 14, 15];
        assert!(SpikeClassifier::iqr_outliers(&fees).is_empty());
    }

    #[test]
    fn iqr_outliers_too_small() {
        assert!(SpikeClassifier::iqr_outliers(&[1, 2, 3]).is_empty());
    }

    #[test]
    fn detects_spikes_above_threshold() {
        let fees = [(1, 100), (2, 250), (3, 90), (4, 600)];
        let spikes = SpikeClassifier::detect_with_threshold(&fees, 100, 1.5);
        assert_eq!(spikes.len(), 2);
        assert_eq!(spikes[0].timestamp, 2);
        assert_eq!(spikes[1].timestamp, 4);
    }

    #[test]
    fn no_spikes_below_threshold() {
        let fees = [(1, 100), (2, 110), (3, 95)];
        let spikes = SpikeClassifier::detect_with_threshold(&fees, 100, 1.5);
        assert!(spikes.is_empty());
    }

    #[test]
    fn severity_levels() {
        let fees = [(1, 150), (2, 300), (3, 700), (4, 1100)];
        let spikes = SpikeClassifier::detect_with_threshold(&fees, 100, 1.0);
        assert_eq!(spikes[0].severity, SpikeSeverity::Low);
        assert_eq!(spikes[1].severity, SpikeSeverity::Medium);
        assert_eq!(spikes[2].severity, SpikeSeverity::High);
        assert_eq!(spikes[3].severity, SpikeSeverity::Critical);
    }
}

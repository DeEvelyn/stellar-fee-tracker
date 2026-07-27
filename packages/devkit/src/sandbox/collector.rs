//! Result collector for sandbox test runs.
//!
//! Accumulates [`FeeRecord`]s produced during a test run and provides
//! aggregate statistics such as average fee and elapsed duration.

use std::time::Duration;

/// A single fee record collected during a sandbox run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeRecord {
    /// Milliseconds since the Unix epoch.
    pub timestamp_ms: u64,
    /// Fee amount in stroops.
    pub fee_stroops: u64,
    /// Ledger sequence number.
    pub sequence: u64,
}

/// Collects [`FeeRecord`]s and computes summary statistics for a sandbox run.
#[derive(Debug)]
pub struct ResultCollector {
    duration: Duration,
    records: Vec<FeeRecord>,
    record_count: usize,
}

impl ResultCollector {
    /// Create an empty collector with zero duration.
    pub fn new() -> Self {
        Self {
            duration: Duration::ZERO,
            records: Vec::new(),
            record_count: 0,
        }
    }

    /// Append a fee record to the collector.
    pub fn record(&mut self, fee: FeeRecord) {
        self.record_count += 1;
        self.records.push(fee);
    }

    /// Number of records collected so far.
    pub fn record_count(&self) -> usize {
        self.record_count
    }

    /// Read-only access to all collected records.
    pub fn records(&self) -> &[FeeRecord] {
        &self.records
    }

    /// Average fee across all collected records, or `0.0` if empty.
    pub fn avg_fee(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }
        let total: u64 = self.records.iter().map(|r| r.fee_stroops).sum();
        total as f64 / self.records.len() as f64
    }

    /// The duration attributed to this collection run.
    pub fn duration(&self) -> Duration {
        self.duration
    }
}

impl Default for ResultCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(fee_stroops: u64) -> FeeRecord {
        FeeRecord { timestamp_ms: 1_000, fee_stroops, sequence: 1 }
    }

    #[test]
    fn new_collector_is_empty() {
        let c = ResultCollector::new();
        assert_eq!(c.record_count(), 0);
        assert!(c.records().is_empty());
        assert_eq!(c.avg_fee(), 0.0);
    }

    #[test]
    fn record_increments_count() {
        let mut c = ResultCollector::new();
        c.record(make_record(100));
        c.record(make_record(200));
        assert_eq!(c.record_count(), 2);
    }

    #[test]
    fn avg_fee_computed_correctly() {
        let mut c = ResultCollector::new();
        c.record(make_record(100));
        c.record(make_record(300));
        assert!((c.avg_fee() - 200.0).abs() < f64::EPSILON);
    }
}

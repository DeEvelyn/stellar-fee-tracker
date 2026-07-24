use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Tracks metrics for analysis runs.
#[derive(Debug, Clone)]
pub struct AnalysisRunMetrics {
    /// Number of percentile computations.
    pub percentile_computations: u64,
    /// Number of rolling window operations.
    pub rolling_window_ops: u64,
    /// Number of spike classifications.
    pub spike_classifications: u64,
    /// Total points analyzed.
    pub total_points_analyzed: u64,
    /// Total spikes identified.
    pub total_spikes_found: u64,
    /// Total time spent in analysis.
    pub total_analysis_time_ns: u64,
    /// Per-analysis-type breakdown.
    pub by_type: BTreeMap<String, AnalysisTypeStats>,
    /// Timestamp of the last analysis run.
    pub last_analysis_at: u64,
}

/// Statistics for a specific type of analysis.
#[derive(Debug, Clone)]
pub struct AnalysisTypeStats {
    /// Number of operations.
    pub operations: u64,
    /// Total points processed.
    pub points: u64,
    /// Total time spent.
    pub time_ns: u64,
    /// Average processing rate (points/sec).
    pub rate: f64,
}

impl AnalysisRunMetrics {
    pub fn new() -> Self {
        Self {
            percentile_computations: 0,
            rolling_window_ops: 0,
            spike_classifications: 0,
            total_points_analyzed: 0,
            total_spikes_found: 0,
            total_analysis_time_ns: 0,
            by_type: BTreeMap::new(),
            last_analysis_at: 0,
        }
    }

    /// Record a percentile computation.
    pub fn record_percentile(&mut self, points: u64, elapsed_ns: u64) {
        self.percentile_computations += 1;
        self.total_points_analyzed += points;
        self.total_analysis_time_ns += elapsed_ns;
        self.last_analysis_at = now_nanos();
        self.update_type_stats("percentile", points, elapsed_ns);
    }

    /// Record a rolling window operation.
    pub fn record_rolling_window(&mut self, points: u64, elapsed_ns: u64) {
        self.rolling_window_ops += 1;
        self.total_points_analyzed += points;
        self.total_analysis_time_ns += elapsed_ns;
        self.last_analysis_at = now_nanos();
        self.update_type_stats("rolling_window", points, elapsed_ns);
    }

    /// Record a spike classification.
    pub fn record_spike_classification(&mut self, points: u64, spikes: u64, elapsed_ns: u64) {
        self.spike_classifications += 1;
        self.total_points_analyzed += points;
        self.total_spikes_found += spikes;
        self.total_analysis_time_ns += elapsed_ns;
        self.last_analysis_at = now_nanos();
        self.update_type_stats("spike_classifier", points, elapsed_ns);
    }

    fn update_type_stats(&mut self, analysis_type: &str, points: u64, elapsed_ns: u64) {
        let stats = self
            .by_type
            .entry(analysis_type.to_string())
            .or_insert(AnalysisTypeStats {
                operations: 0,
                points: 0,
                time_ns: 0,
                rate: 0.0,
            });
        stats.operations += 1;
        stats.points += points;
        stats.time_ns += elapsed_ns;
        stats.rate = if stats.time_ns > 0 {
            stats.points as f64 / Duration::from_nanos(stats.time_ns).as_secs_f64()
        } else {
            0.0
        };
    }

    /// Total number of analysis operations.
    pub fn total_operations(&self) -> u64 {
        self.percentile_computations + self.rolling_window_ops + self.spike_classifications
    }

    /// Average analysis time per operation.
    pub fn avg_time_per_op(&self) -> Duration {
        let ops = self.total_operations();
        if ops == 0 {
            Duration::ZERO
        } else {
            Duration::from_nanos(self.total_analysis_time_ns / ops)
        }
    }

    /// Overall processing rate (points per second).
    pub fn overall_rate(&self) -> f64 {
        if self.total_analysis_time_ns == 0 {
            0.0
        } else {
            self.total_points_analyzed as f64 / Duration::from_nanos(self.total_analysis_time_ns).as_secs_f64()
        }
    }

    /// Return a display-friendly report.
    pub fn report(&self) -> String {
        let mut out = format!(
            "Analysis Run Metrics\n\
             ====================\n\
             percentile:      {}\n\
             rolling window:  {}\n\
             spike classify:  {}\n\
             total ops:       {}\n\
             points analyzed: {}\n\
             spikes found:    {}\n\
             avg time/op:     {:?}\n\
             overall rate:    {:.0} pts/s\n",
            self.percentile_computations,
            self.rolling_window_ops,
            self.spike_classifications,
            self.total_operations(),
            self.total_points_analyzed,
            self.total_spikes_found,
            self.avg_time_per_op(),
            self.overall_rate(),
        );
        if !self.by_type.is_empty() {
            out.push_str("\nBy type:\n");
            for (name, stats) in &self.by_type {
                out.push_str(&format!(
                    "  {}: {} ops, {} pts, {:.0} pts/s\n",
                    name, stats.operations, stats.points, stats.rate,
                ));
            }
        }
        out
    }

    /// Return a JSON representation.
    pub fn to_json(&self) -> String {
        let types: Vec<String> = self
            .by_type
            .iter()
            .map(|(name, s)| {
                format!(
                    r#""{}":{{"ops":{},"points":{},"rate":{:.0}}}"#,
                    name, s.operations, s.points, s.rate,
                )
            })
            .collect();
        format!(
            r#"{{"percentile":{},"rolling_window":{},"spike_classify":{},"total_ops":{},"points":{},"spikes":{},"rate":{:.0},"types":{{{}}}}}"#,
            self.percentile_computations,
            self.rolling_window_ops,
            self.spike_classifications,
            self.total_operations(),
            self.total_points_analyzed,
            self.total_spikes_found,
            self.overall_rate(),
            types.join(","),
        )
    }

    /// Reset all metrics.
    pub fn reset(&mut self) {
        self.percentile_computations = 0;
        self.rolling_window_ops = 0;
        self.spike_classifications = 0;
        self.total_points_analyzed = 0;
        self.total_spikes_found = 0;
        self.total_analysis_time_ns = 0;
        self.by_type.clear();
        self.last_analysis_at = 0;
    }

    /// Merge metrics from another collector.
    pub fn merge(&mut self, other: &AnalysisRunMetrics) {
        self.percentile_computations += other.percentile_computations;
        self.rolling_window_ops += other.rolling_window_ops;
        self.spike_classifications += other.spike_classifications;
        self.total_points_analyzed += other.total_points_analyzed;
        self.total_spikes_found += other.total_spikes_found;
        self.total_analysis_time_ns += other.total_analysis_time_ns;
        for (name, stats) in &other.by_type {
            let entry = self.by_type.entry(name.clone()).or_insert(AnalysisTypeStats {
                operations: 0, points: 0, time_ns: 0, rate: 0.0,
            });
            entry.operations += stats.operations;
            entry.points += stats.points;
            entry.time_ns += stats.time_ns;
            entry.rate = if entry.time_ns > 0 {
                entry.points as f64 / Duration::from_nanos(entry.time_ns).as_secs_f64()
            } else {
                0.0
            };
        }
        self.last_analysis_at = self.last_analysis_at.max(other.last_analysis_at);
    }
}

impl Default for AnalysisRunMetrics {
    fn default() -> Self {
        Self::new()
    }
}

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_metrics_are_zeroed() {
        let m = AnalysisRunMetrics::new();
        assert_eq!(m.total_operations(), 0);
        assert_eq!(m.total_points_analyzed, 0);
    }

    #[test]
    fn record_percentile_increments() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(1000, 500_000);
        assert_eq!(m.percentile_computations, 1);
        assert_eq!(m.total_points_analyzed, 1000);
    }

    #[test]
    fn record_rolling_window_increments() {
        let mut m = AnalysisRunMetrics::new();
        m.record_rolling_window(500, 200_000);
        assert_eq!(m.rolling_window_ops, 1);
    }

    #[test]
    fn record_spike_classification_increments() {
        let mut m = AnalysisRunMetrics::new();
        m.record_spike_classification(100, 5, 100_000);
        assert_eq!(m.spike_classifications, 1);
        assert_eq!(m.total_spikes_found, 5);
    }

    #[test]
    fn total_operations_returns_sum() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(100, 1000);
        m.record_rolling_window(100, 1000);
        assert_eq!(m.total_operations(), 2);
    }

    #[test]
    fn avg_time_per_op() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(100, 1_000_000);
        m.record_percentile(100, 1_000_000);
        assert_eq!(m.avg_time_per_op(), Duration::from_millis(1));
    }

    #[test]
    fn overall_rate() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(1000, 1_000_000_000); // 1 second
        assert!(m.overall_rate() > 0.0);
    }

    #[test]
    fn by_type_tracks_separate_types() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(100, 1000);
        m.record_rolling_window(200, 1000);
        assert_eq!(m.by_type.len(), 2);
    }

    #[test]
    fn report_contains_fields() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(500, 1_000_000);
        let report = m.report();
        assert!(report.contains("percentile"));
        assert!(report.contains("points"));
    }

    #[test]
    fn to_json_contains_keys() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(100, 500_000);
        let json = m.to_json();
        assert!(json.contains("percentile"));
    }

    #[test]
    fn reset_clears_all() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(100, 1000);
        m.reset();
        assert_eq!(m.total_operations(), 0);
        assert!(m.by_type.is_empty());
    }

    #[test]
    fn merge_combines() {
        let mut a = AnalysisRunMetrics::new();
        a.record_percentile(100, 1000);
        let mut b = AnalysisRunMetrics::new();
        b.record_percentile(200, 2000);
        a.merge(&b);
        assert_eq!(a.percentile_computations, 2);
        assert_eq!(a.total_points_analyzed, 300);
    }

    #[test]
    fn type_stats_rate() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(1000, 1_000_000_000); // 1000 pts in 1 sec
        let stats = m.by_type.get("percentile").unwrap();
        assert!(stats.rate > 0.0);
    }

    #[test]
    fn multiple_records_same_type() {
        let mut m = AnalysisRunMetrics::new();
        m.record_percentile(100, 1000);
        m.record_percentile(100, 1000);
        assert_eq!(m.by_type.get("percentile").unwrap().operations, 2);
    }
}

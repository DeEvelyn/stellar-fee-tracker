use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Tracks metrics for simulation runs.
#[derive(Debug, Clone)]
pub struct SimulationMetrics {
    /// Total number of simulation runs.
    pub total_runs: u64,
    /// Total fee points generated.
    pub total_points: u64,
    /// Total spikes detected.
    pub total_spikes: u64,
    /// Total time spent simulating.
    pub total_simulation_time_ns: u64,
    /// Per-scenario breakdown.
    pub by_scenario: BTreeMap<String, ScenarioStats>,
    /// Timestamp of the last simulation run.
    pub last_run_at: u64,
}

/// Statistics for a single simulation scenario.
#[derive(Debug, Clone)]
pub struct ScenarioStats {
    /// Number of runs for this scenario.
    pub runs: u64,
    /// Total points generated.
    pub points: u64,
    /// Spikes detected.
    pub spikes: u64,
    /// Average fee.
    pub avg_fee: f64,
    /// Minimum fee observed.
    pub min_fee: u64,
    /// Maximum fee observed.
    pub max_fee: u64,
}

impl SimulationMetrics {
    pub fn new() -> Self {
        Self {
            total_runs: 0,
            total_points: 0,
            total_spikes: 0,
            total_simulation_time_ns: 0,
            by_scenario: BTreeMap::new(),
            last_run_at: 0,
        }
    }

    /// Record a completed simulation run.
    pub fn record_run(&mut self, scenario: &str, points: u64, spikes: u64, min_fee: u64, max_fee: u64, elapsed_ns: u64) {
        self.total_runs += 1;
        self.total_points += points;
        self.total_spikes += spikes;
        self.total_simulation_time_ns += elapsed_ns;
        self.last_run_at = now_nanos();

        let stats = self.by_scenario.entry(scenario.to_string()).or_insert(ScenarioStats {
            runs: 0,
            points: 0,
            spikes: 0,
            avg_fee: 0.0,
            min_fee: u64::MAX,
            max_fee: 0,
        });
        stats.runs += 1;
        stats.points += points;
        stats.spikes += spikes;
        stats.min_fee = stats.min_fee.min(min_fee);
        stats.max_fee = stats.max_fee.max(max_fee);
        stats.avg_fee = ((stats.avg_fee * (stats.runs - 1) as f64) + ((min_fee + max_fee) as f64 / 2.0)) / stats.runs as f64;
    }

    /// Average points per simulation run.
    pub fn avg_points_per_run(&self) -> f64 {
        if self.total_runs == 0 { 0.0 } else { self.total_points as f64 / self.total_runs as f64 }
    }

    /// Spike rate as a percentage.
    pub fn spike_rate(&self) -> f64 {
        if self.total_points == 0 { 0.0 } else { self.total_spikes as f64 / self.total_points as f64 * 100.0 }
    }

    /// Average simulation time per run.
    pub fn avg_time_per_run(&self) -> Duration {
        if self.total_runs == 0 {
            Duration::ZERO
        } else {
            Duration::from_nanos(self.total_simulation_time_ns / self.total_runs)
        }
    }

    /// Return a display-friendly report.
    pub fn report(&self) -> String {
        let mut out = format!(
            "Simulation Metrics\n\
             ==================\n\
             total runs:     {}\n\
             total points:   {}\n\
             total spikes:   {}\n\
             spike rate:     {:.2}%\n\
             avg points/run: {:.1}\n\
             avg time/run:   {:?}\n",
            self.total_runs,
            self.total_points,
            self.total_spikes,
            self.spike_rate(),
            self.avg_points_per_run(),
            self.avg_time_per_run(),
        );
        if !self.by_scenario.is_empty() {
            out.push_str("\nBy scenario:\n");
            for (name, stats) in &self.by_scenario {
                out.push_str(&format!(
                    "  {}: {} runs, {} points, {} spikes, avg fee {:.1}\n",
                    name, stats.runs, stats.points, stats.spikes, stats.avg_fee,
                ));
            }
        }
        out
    }

    /// Return a JSON representation.
    pub fn to_json(&self) -> String {
        let scenarios: Vec<String> = self
            .by_scenario
            .iter()
            .map(|(name, s)| {
                format!(
                    r#""{}":{{"runs":{},"points":{},"spikes":{},"avg_fee":{:.1},"min_fee":{},"max_fee":{}}}"#,
                    name, s.runs, s.points, s.spikes, s.avg_fee, s.min_fee, s.max_fee,
                )
            })
            .collect();
        format!(
            r#"{{"total_runs":{},"total_points":{},"total_spikes":{},"spike_rate":{:.2},"avg_points_per_run":{:.1},"scenarios":{{{}}}}}"#,
            self.total_runs,
            self.total_points,
            self.total_spikes,
            self.spike_rate(),
            self.avg_points_per_run(),
            scenarios.join(","),
        )
    }

    /// Reset all metrics.
    pub fn reset(&mut self) {
        self.total_runs = 0;
        self.total_points = 0;
        self.total_spikes = 0;
        self.total_simulation_time_ns = 0;
        self.by_scenario.clear();
        self.last_run_at = 0;
    }

    /// Merge metrics from another collector.
    pub fn merge(&mut self, other: &SimulationMetrics) {
        self.total_runs += other.total_runs;
        self.total_points += other.total_points;
        self.total_spikes += other.total_spikes;
        self.total_simulation_time_ns += other.total_simulation_time_ns;
        for (name, stats) in &other.by_scenario {
            let entry = self.by_scenario.entry(name.clone()).or_insert_with(|| ScenarioStats {
                runs: 0, points: 0, spikes: 0, avg_fee: 0.0, min_fee: u64::MAX, max_fee: 0,
            });
            entry.runs += stats.runs;
            entry.points += stats.points;
            entry.spikes += stats.spikes;
            entry.min_fee = entry.min_fee.min(stats.min_fee);
            entry.max_fee = entry.max_fee.max(stats.max_fee);
            entry.avg_fee = (entry.avg_fee + stats.avg_fee) / 2.0;
        }
        self.last_run_at = self.last_run_at.max(other.last_run_at);
    }
}

impl Default for SimulationMetrics {
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
        let m = SimulationMetrics::new();
        assert_eq!(m.total_runs, 0);
        assert_eq!(m.total_points, 0);
    }

    #[test]
    fn record_run_increments_counters() {
        let mut m = SimulationMetrics::new();
        m.record_run("normal", 1000, 5, 50, 500, 1_000_000);
        assert_eq!(m.total_runs, 1);
        assert_eq!(m.total_points, 1000);
        assert_eq!(m.total_spikes, 5);
    }

    #[test]
    fn avg_points_per_run() {
        let mut m = SimulationMetrics::new();
        m.record_run("normal", 1000, 0, 100, 200, 1_000_000);
        m.record_run("normal", 2000, 0, 100, 200, 2_000_000);
        assert!((m.avg_points_per_run() - 1500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spike_rate_calculation() {
        let mut m = SimulationMetrics::new();
        m.record_run("normal", 1000, 50, 100, 200, 1_000_000);
        assert!((m.spike_rate() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spike_rate_zero_when_no_points() {
        let m = SimulationMetrics::new();
        assert_eq!(m.spike_rate(), 0.0);
    }

    #[test]
    fn by_scenario_tracks_separately() {
        let mut m = SimulationMetrics::new();
        m.record_run("normal", 100, 1, 50, 150, 1_000_000);
        m.record_run("congested", 200, 10, 100, 500, 2_000_000);
        assert_eq!(m.by_scenario.len(), 2);
        assert_eq!(m.by_scenario.get("normal").unwrap().runs, 1);
        assert_eq!(m.by_scenario.get("congested").unwrap().runs, 1);
    }

    #[test]
    fn report_contains_fields() {
        let mut m = SimulationMetrics::new();
        m.record_run("test", 500, 3, 50, 200, 1_000_000);
        let report = m.report();
        assert!(report.contains("total runs"));
        assert!(report.contains("test"));
    }

    #[test]
    fn to_json_contains_keys() {
        let mut m = SimulationMetrics::new();
        m.record_run("scenario", 100, 2, 50, 200, 1_000_000);
        let json = m.to_json();
        assert!(json.contains("total_runs"));
        assert!(json.contains("scenario"));
    }

    #[test]
    fn reset_clears_all() {
        let mut m = SimulationMetrics::new();
        m.record_run("normal", 100, 1, 50, 150, 1_000_000);
        m.reset();
        assert_eq!(m.total_runs, 0);
        assert!(m.by_scenario.is_empty());
    }

    #[test]
    fn merge_combines_metrics() {
        let mut a = SimulationMetrics::new();
        a.record_run("normal", 100, 1, 50, 150, 1_000_000);
        let mut b = SimulationMetrics::new();
        b.record_run("normal", 200, 2, 60, 160, 2_000_000);
        a.merge(&b);
        assert_eq!(a.total_runs, 2);
        assert_eq!(a.total_points, 300);
    }

    #[test]
    fn avg_time_per_run() {
        let mut m = SimulationMetrics::new();
        m.record_run("normal", 100, 0, 50, 100, 2_000_000);
        assert_eq!(m.avg_time_per_run(), Duration::from_millis(2));
    }

    #[test]
    fn scenario_stats_avg_fee() {
        let mut m = SimulationMetrics::new();
        m.record_run("s1", 100, 1, 100, 200, 1_000_000);
        m.record_run("s1", 100, 0, 100, 200, 1_000_000);
        let stats = m.by_scenario.get("s1").unwrap();
        assert!(stats.avg_fee > 0.0);
    }
}

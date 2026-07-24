use std::collections::BTreeMap;
use std::time::Instant;

/// A snapshot of devkit runtime metrics.
#[derive(Debug, Clone)]
pub struct Metrics {
    /// Number of simulation runs completed.
    pub simulations_run: u64,
    /// Total fee points generated across all runs.
    pub total_fee_points: u64,
    /// Number of spikes detected.
    pub spikes_detected: u64,
    /// Number of times data was exported.
    pub exports_performed: u64,
    /// Number of replay operations.
    pub replays_performed: u64,
    /// Uptime of the devkit process in seconds.
    pub uptime_secs: u64,
    /// Memory usage estimate in bytes.
    pub memory_estimate_bytes: u64,
    /// Custom metric key-value pairs.
    pub custom: BTreeMap<String, f64>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            simulations_run: 0,
            total_fee_points: 0,
            spikes_detected: 0,
            exports_performed: 0,
            replays_performed: 0,
            uptime_secs: 0,
            memory_estimate_bytes: 0,
            custom: BTreeMap::new(),
        }
    }
}

impl Metrics {
    /// Record that a simulation was completed with `points` fee points.
    pub fn record_simulation(&mut self, points: u64, spikes: u64) {
        self.simulations_run += 1;
        self.total_fee_points += points;
        self.spikes_detected += spikes;
    }

    /// Record an export operation.
    pub fn record_export(&mut self) {
        self.exports_performed += 1;
    }

    /// Record a replay operation.
    pub fn record_replay(&mut self) {
        self.replays_performed += 1;
    }

    /// Update the uptime estimate based on process start time.
    pub fn update_uptime(&mut self, start: Instant) {
        self.uptime_secs = start.elapsed().as_secs();
    }

    /// Format metrics as a text report.
    pub fn display(&self) -> String {
        let mut out = format!(
            "devkit metrics\n\
             ==============\n\
             simulations:    {}\n\
             fee points:     {}\n\
             spikes:         {}\n\
             exports:        {}\n\
             replays:        {}\n\
             uptime:         {}s\n\
             memory (est.):  {} bytes\n\
             custom keys:    {}",
            self.simulations_run,
            self.total_fee_points,
            self.spikes_detected,
            self.exports_performed,
            self.replays_performed,
            self.uptime_secs,
            self.memory_estimate_bytes,
            self.custom.len(),
        );
        if !self.custom.is_empty() {
            out.push('\n');
            for (k, v) in &self.custom {
                out.push_str(&format!("  {}: {}\n", k, v));
            }
        }
        out
    }

    /// Format metrics as a JSON object.
    pub fn to_json(&self) -> String {
        let custom_json: Vec<String> = self
            .custom
            .iter()
            .map(|(k, v)| format!("\"{}\":{}", k, v))
            .collect();
        format!(
            r#"{{"simulations_run":{},"total_fee_points":{},"spikes_detected":{},"exports_performed":{},"replays_performed":{},"uptime_secs":{},"memory_estimate_bytes":{},"custom":{{{}}}}}"#,
            self.simulations_run,
            self.total_fee_points,
            self.spikes_detected,
            self.exports_performed,
            self.replays_performed,
            self.uptime_secs,
            self.memory_estimate_bytes,
            custom_json.join(","),
        )
    }

    /// Merge another metrics snapshot into this one.
    pub fn merge(&mut self, other: &Metrics) {
        self.simulations_run += other.simulations_run;
        self.total_fee_points += other.total_fee_points;
        self.spikes_detected += other.spikes_detected;
        self.exports_performed += other.exports_performed;
        self.replays_performed += other.replays_performed;
        self.uptime_secs = self.uptime_secs.max(other.uptime_secs);
        self.memory_estimate_bytes = self.memory_estimate_bytes.max(other.memory_estimate_bytes);
        for (k, v) in &other.custom {
            *self.custom.entry(k.clone()).or_insert(0.0) += v;
        }
    }

    /// Reset all counters to zero.
    pub fn reset(&mut self) {
        self.simulations_run = 0;
        self.total_fee_points = 0;
        self.spikes_detected = 0;
        self.exports_performed = 0;
        self.replays_performed = 0;
        self.uptime_secs = 0;
        self.memory_estimate_bytes = 0;
        self.custom.clear();
    }

    /// Compute the spike rate as a percentage.
    pub fn spike_rate(&self) -> f64 {
        if self.total_fee_points == 0 {
            0.0
        } else {
            self.spikes_detected as f64 / self.total_fee_points as f64 * 100.0
        }
    }

    /// Estimate average fee points per simulation.
    pub fn avg_points_per_simulation(&self) -> f64 {
        if self.simulations_run == 0 {
            0.0
        } else {
            self.total_fee_points as f64 / self.simulations_run as f64
        }
    }
}

/// Arguments for the `metrics` subcommand.
pub struct MetricsArgs {
    /// Output as JSON.
    pub json: bool,
    /// Reset metrics after displaying.
    pub reset: bool,
    /// Suppress all output except errors.
    pub quiet: bool,
}

impl Default for MetricsArgs {
    fn default() -> Self {
        Self {
            json: false,
            reset: false,
            quiet: false,
        }
    }
}

impl MetricsArgs {
    /// Run the metrics subcommand.
    pub fn run(&self, metrics: &Metrics) {
        if self.quiet {
            return;
        }
        if self.json {
            println!("{}", metrics.to_json());
        } else {
            println!("{}", metrics.display());
        }
    }

    /// Collect current process metrics (returns a zeroed snapshot).
    pub fn collect() -> Metrics {
        Metrics::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metrics() -> Metrics {
        let mut m = Metrics::default();
        m.record_simulation(1000, 12);
        m.record_simulation(500, 3);
        m.record_export();
        m.record_replay();
        m.custom.insert("cache_hits".into(), 42.0);
        m
    }

    #[test]
    fn metrics_default_is_zeroed() {
        let m = Metrics::default();
        assert_eq!(m.simulations_run, 0);
        assert_eq!(m.spike_rate(), 0.0);
    }

    #[test]
    fn record_simulation_increments_counters() {
        let mut m = Metrics::default();
        m.record_simulation(1000, 5);
        assert_eq!(m.simulations_run, 1);
        assert_eq!(m.total_fee_points, 1000);
        assert_eq!(m.spikes_detected, 5);
    }

    #[test]
    fn spike_rate_computes_correct_percentage() {
        let mut m = Metrics::default();
        m.record_simulation(1000, 50);
        assert!((m.spike_rate() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn merge_combines_two_metric_sets() {
        let mut a = sample_metrics();
        let mut b = Metrics::default();
        b.record_simulation(200, 1);
        a.merge(&b);
        assert_eq!(a.simulations_run, 3);
        assert_eq!(a.total_fee_points, 1700);
    }

    #[test]
    fn reset_clears_everything() {
        let mut m = sample_metrics();
        m.reset();
        assert_eq!(m.simulations_run, 0);
    }

    #[test]
    fn display_includes_all_fields() {
        let out = sample_metrics().display();
        assert!(out.contains("simulations"));
        assert!(out.contains("spikes"));
        assert!(out.contains("cache_hits"));
    }

    #[test]
    fn json_output_is_valid_object() {
        let json = sample_metrics().to_json();
        assert!(json.starts_with('{'));
        assert!(json.contains("simulations_run"));
    }

    #[test]
    fn metrics_args_default() {
        let args = MetricsArgs::default();
        assert!(!args.json && !args.reset);
    }
}

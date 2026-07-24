use std::time::{Duration, Instant};

/// A single benchmark measurement.
#[derive(Debug, Clone)]
pub struct Measurement {
    /// Name of the benchmark.
    pub name: String,
    /// Number of iterations.
    pub iterations: u64,
    /// Total elapsed time.
    pub elapsed: Duration,
    /// Optional throughput (operations per second).
    pub throughput: Option<f64>,
    /// Optional memory delta in bytes.
    pub memory_bytes: Option<u64>,
}

impl Measurement {
    /// Average time per iteration in nanoseconds.
    pub fn avg_ns(&self) -> f64 {
        self.elapsed.as_nanos() as f64 / self.iterations as f64
    }

    /// Average time per iteration in microseconds.
    pub fn avg_us(&self) -> f64 {
        self.avg_ns() / 1000.0
    }

    /// Average time per iteration in milliseconds.
    pub fn avg_ms(&self) -> f64 {
        self.avg_us() / 1000.0
    }

    /// Format the measurement as a human-readable string.
    pub fn display(&self) -> String {
        let avg = if self.avg_ms() >= 1.0 {
            format!("{:.2} ms", self.avg_ms())
        } else if self.avg_us() >= 1.0 {
            format!("{:.2} µs", self.avg_us())
        } else {
            format!("{:.0} ns", self.avg_ns())
        };
        let throughput = self
            .throughput
            .map(|t| format!(", {:.0} ops/s", t))
            .unwrap_or_default();
        let mem = self
            .memory_bytes
            .map(|b| format!(", {} bytes", b))
            .unwrap_or_default();
        format!("{}: {} iterations in {:?}{}{}", self.name, self.iterations, self.elapsed, avg, throughput, mem)
    }

    /// Format as a JSON line.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"name":"{}","iterations":{},"elapsed_ns":{},"avg_ns":{:.2},"throughput":{},"memory_bytes":{}}}"#,
            self.name,
            self.iterations,
            self.elapsed.as_nanos(),
            self.avg_ns(),
            self.throughput.map(|t| t.to_string()).unwrap_or_else(|| "null".into()),
            self.memory_bytes.map(|b| b.to_string()).unwrap_or_else(|| "null".into()),
        )
    }
}

/// A benchmark suite that measures monitoring overhead.
pub struct MonitoringBenchmark {
    results: Vec<Measurement>,
}

impl MonitoringBenchmark {
    pub fn new() -> Self {
        Self { results: Vec::new() }
    }

    /// Run a benchmark measuring trace context creation overhead.
    pub fn bench_trace_creation(&mut self, iterations: u64) {
        let start = Instant::now();
        for _ in 0..iterations {
            let _ctx = super::TraceContext::new_root();
        }
        let elapsed = start.elapsed();
        self.results.push(Measurement {
            name: "trace_creation".into(),
            iterations,
            elapsed,
            throughput: Some(iterations as f64 / elapsed.as_secs_f64()),
            memory_bytes: None,
        });
    }

    /// Run a benchmark measuring trace context propagation overhead.
    pub fn bench_trace_propagation(&mut self, iterations: u64) {
        let propagator = super::W3CPropagator;
        let ctx = super::TraceContext::new_root();
        let mut carrier = std::collections::HashMap::new();
        let start = Instant::now();
        for _ in 0..iterations {
            propagator.inject(&ctx, &mut carrier);
            let _extracted = propagator.extract(&carrier);
            carrier.clear();
        }
        let elapsed = start.elapsed();
        self.results.push(Measurement {
            name: "trace_propagation".into(),
            iterations,
            elapsed,
            throughput: Some(iterations as f64 / elapsed.as_secs_f64()),
            memory_bytes: None,
        });
    }

    /// Run a benchmark measuring baggage operations.
    pub fn bench_baggage_ops(&mut self, iterations: u64) {
        let start = Instant::now();
        for i in 0..iterations {
            let ctx = super::TraceContext::new_root()
                .with_baggage("key", &format!("value{}", i));
            let _val = ctx.baggage("key");
        }
        let elapsed = start.elapsed();
        self.results.push(Measurement {
            name: "baggage_ops".into(),
            iterations,
            elapsed,
            throughput: Some(iterations as f64 / elapsed.as_secs_f64()),
            memory_bytes: None,
        });
    }

    /// Run a benchmark measuring context propagation cost.
    pub fn bench_span_creation(&mut self, iterations: u64) {
        let root = super::TraceContext::new_root();
        let start = Instant::now();
        for _ in 0..iterations {
            let _child = root.child_span();
        }
        let elapsed = start.elapsed();
        self.results.push(Measurement {
            name: "span_creation".into(),
            iterations,
            elapsed,
            throughput: Some(iterations as f64 / elapsed.as_secs_f64()),
            memory_bytes: None,
        });
    }

    /// Run all standard benchmarks with a given iteration count.
    pub fn run_all(&mut self, iterations: u64) {
        self.bench_trace_creation(iterations);
        self.bench_trace_propagation(iterations);
        self.bench_baggage_ops(iterations);
        self.bench_span_creation(iterations);
    }

    /// Display all benchmark results.
    pub fn report(&self) -> String {
        let mut out = String::from("Monitoring Overhead Benchmark\n");
        out.push_str("==============================\n");
        for m in &self.results {
            out.push_str(&format!("  {}\n", m.display()));
        }
        out
    }

    /// Export all results as a JSON array.
    pub fn to_json(&self) -> String {
        let items: Vec<String> = self.results.iter().map(|m| m.to_json()).collect();
        format!("[{}]", items.join(","))
    }

    /// Clear all results.
    pub fn reset(&mut self) {
        self.results.clear();
    }

    /// Return the worst-case average latency across all benchmarks.
    pub fn worst_avg_ns(&self) -> f64 {
        self.results.iter().map(|m| m.avg_ns()).fold(0.0_f64, f64::max)
    }

    /// Return the total elapsed time across all benchmarks.
    pub fn total_elapsed(&self) -> Duration {
        self.results.iter().fold(Duration::ZERO, |acc, m| acc + m.elapsed)
    }
}

/// Arguments for the benchmark subcommand.
pub struct BenchmarkOverheadArgs {
    /// Number of iterations per benchmark.
    pub iterations: u64,
    /// Output as JSON.
    pub json: bool,
}

impl Default for BenchmarkOverheadArgs {
    fn default() -> Self {
        Self {
            iterations: 10_000,
            json: false,
        }
    }
}

impl BenchmarkOverheadArgs {
    /// Run the benchmark.
    pub fn run(&self) {
        let mut bench = MonitoringBenchmark::new();
        bench.run_all(self.iterations);
        if self.json {
            println!("{}", bench.to_json());
        } else {
            println!("{}", bench.report());
        }
    }

    /// Suggest an iteration count based on desired precision.
    pub fn suggest_iterations(precision_pct: f64) -> u64 {
        (100.0 / precision_pct).powi(2) as u64
    }

    /// Format a duration in a human-friendly way.
    pub fn format_duration(d: &Duration) -> String {
        let total_ns = d.as_nanos();
        if total_ns >= 1_000_000_000 {
            format!("{:.2}s", d.as_secs_f64())
        } else if total_ns >= 1_000_000 {
            format!("{:.2}ms", d.as_secs_f64() * 1000.0)
        } else if total_ns >= 1_000 {
            format!("{:.2}µs", d.as_secs_f64() * 1_000_000.0)
        } else {
            format!("{}ns", total_ns)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurement_avg_ns_is_correct() {
        let m = Measurement {
            name: "test".into(),
            iterations: 10,
            elapsed: Duration::from_nanos(1000),
            throughput: None,
            memory_bytes: None,
        };
        assert!((m.avg_ns() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn benchmark_runs_without_panic() {
        let mut bench = MonitoringBenchmark::new();
        bench.run_all(100);
        assert_eq!(bench.results.len(), 4);
    }

    #[test]
    fn benchmark_report_contains_names() {
        let mut bench = MonitoringBenchmark::new();
        bench.run_all(100);
        let report = bench.report();
        assert!(report.contains("trace_creation"));
        assert!(report.contains("span_creation"));
    }

    #[test]
    fn benchmark_json_is_array() {
        let mut bench = MonitoringBenchmark::new();
        bench.run_all(100);
        let json = bench.to_json();
        assert!(json.starts_with('['));
        assert!(json.contains("trace_creation"));
    }

    #[test]
    fn reset_clears_results() {
        let mut bench = MonitoringBenchmark::new();
        bench.run_all(100);
        bench.reset();
        assert!(bench.results.is_empty());
    }

    #[test]
    fn worst_avg_ns_returns_max() {
        let mut bench = MonitoringBenchmark::new();
        bench.run_all(100);
        assert!(bench.worst_avg_ns() > 0.0);
    }

    #[test]
    fn suggest_iterations_scales_with_precision() {
        let low = BenchmarkOverheadArgs::suggest_iterations(10.0);
        let high = BenchmarkOverheadArgs::suggest_iterations(1.0);
        assert!(high > low);
    }

    #[test]
    fn format_duration_handles_ns() {
        let s = BenchmarkOverheadArgs::format_duration(&Duration::from_nanos(500));
        assert!(s.contains("ns"));
    }

    #[test]
    fn benchmark_args_default() {
        let args = BenchmarkOverheadArgs::default();
        assert_eq!(args.iterations, 10_000);
        assert!(!args.json);
    }

    #[test]
    fn measurement_json_includes_fields() {
        let m = Measurement {
            name: "test".into(),
            iterations: 100,
            elapsed: Duration::from_micros(500),
            throughput: Some(200_000.0),
            memory_bytes: Some(1024),
        };
        let json = m.to_json();
        assert!(json.contains("throughput"));
        assert!(json.contains("memory_bytes"));
    }
}

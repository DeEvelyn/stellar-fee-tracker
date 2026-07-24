use std::time::{Duration, Instant};

/// Measures and records execution time of operations.
#[derive(Debug, Clone)]
pub struct ExecutionTimer {
    /// Label identifying the measured operation.
    pub label: String,
    /// Start time.
    start: Option<Instant>,
    /// Accumulated duration across multiple runs.
    accumulated: Duration,
    /// Number of times the timer was started.
    run_count: u64,
}

impl ExecutionTimer {
    /// Create a new execution timer with a label.
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            start: None,
            accumulated: Duration::ZERO,
            run_count: 0,
        }
    }

    /// Start the timer.
    pub fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    /// Stop the timer and record the elapsed duration.
    pub fn stop(&mut self) -> Duration {
        let elapsed = self
            .start
            .take()
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);
        self.accumulated += elapsed;
        self.run_count += 1;
        elapsed
    }

    /// Return the total accumulated duration.
    pub fn total(&self) -> Duration {
        self.accumulated
    }

    /// Return the average duration per run.
    pub fn average(&self) -> Duration {
        if self.run_count == 0 {
            Duration::ZERO
        } else {
            self.accumulated / self.run_count as u32
        }
    }

    /// Number of times the timer was stopped.
    pub fn count(&self) -> u64 {
        self.run_count
    }

    /// Reset the timer to its initial state.
    pub fn reset(&mut self) {
        self.start = None;
        self.accumulated = Duration::ZERO;
        self.run_count = 0;
    }

    /// Return a summary string.
    pub fn summary(&self) -> String {
        format!(
            "{}: {} runs, total {:?}, avg {:?}",
            self.label,
            self.run_count,
            self.accumulated,
            self.average(),
        )
    }

    /// Return a JSON representation.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"label":"{}","runs":{},"total_ns":{},"avg_ns":{}}}"#,
            self.label,
            self.run_count,
            self.accumulated.as_nanos(),
            self.average().as_nanos(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_timer_has_zero_state() {
        let timer = ExecutionTimer::new("test");
        assert_eq!(timer.label, "test");
        assert_eq!(timer.count(), 0);
        assert_eq!(timer.total(), Duration::ZERO);
    }

    #[test]
    fn start_stop_records_elapsed() {
        let mut timer = ExecutionTimer::new("op");
        timer.start();
        std::thread::sleep(Duration::from_micros(1));
        let elapsed = timer.stop();
        assert!(elapsed > Duration::ZERO);
        assert!(timer.count() == 1);
    }

    #[test]
    fn multiple_runs_accumulate() {
        let mut timer = ExecutionTimer::new("batch");
        for _ in 0..5 {
            timer.start();
            timer.stop();
        }
        assert_eq!(timer.count(), 5);
    }

    #[test]
    fn average_is_correct() {
        let mut timer = ExecutionTimer::new("test");
        timer.start();
        timer.stop();
        let avg = timer.average();
        assert_eq!(avg, timer.total() / 1);
    }

    #[test]
    fn reset_clears_state() {
        let mut timer = ExecutionTimer::new("tmp");
        timer.start();
        timer.stop();
        timer.reset();
        assert_eq!(timer.count(), 0);
        assert_eq!(timer.total(), Duration::ZERO);
    }

    #[test]
    fn stop_without_start_returns_zero() {
        let mut timer = ExecutionTimer::new("noop");
        assert_eq!(timer.stop(), Duration::ZERO);
    }

    #[test]
    fn summary_includes_label() {
        let mut timer = ExecutionTimer::new("my-op");
        timer.start();
        timer.stop();
        let s = timer.summary();
        assert!(s.contains("my-op"));
    }

    #[test]
    fn to_json_contains_label() {
        let mut timer = ExecutionTimer::new("json-test");
        timer.start();
        timer.stop();
        let json = timer.to_json();
        assert!(json.contains("json-test"));
        assert!(json.contains("runs"));
    }

    #[test]
    fn average_with_no_runs_is_zero() {
        let timer = ExecutionTimer::new("empty");
        assert_eq!(timer.average(), Duration::ZERO);
    }

    #[test]
    fn concurrent_timers_are_independent() {
        let mut a = ExecutionTimer::new("a");
        let mut b = ExecutionTimer::new("b");
        a.start();
        b.start();
        a.stop();
        b.stop();
        assert!(a.count() == 1);
        assert!(b.count() == 1);
    }
}

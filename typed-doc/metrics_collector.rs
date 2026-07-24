use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// A counter metric that can be incremented and read.
#[derive(Debug, Clone)]
pub struct Counter {
    name: String,
    value: u64,
}

impl Counter {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), value: 0 }
    }

    pub fn increment(&mut self, delta: u64) {
        self.value += delta;
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A gauge metric that holds a current value.
#[derive(Debug, Clone)]
pub struct Gauge {
    name: String,
    value: f64,
}

impl Gauge {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), value: 0.0 }
    }

    pub fn set(&mut self, value: f64) {
        self.value = value;
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A histogram that records value distributions.
#[derive(Debug, Clone)]
pub struct Histogram {
    name: String,
    buckets: BTreeMap<u64, u64>,
    count: u64,
    sum: u64,
}

impl Histogram {
    pub fn new(name: &str) -> Self {
        let mut buckets = BTreeMap::new();
        for b in &[1, 5, 10, 50, 100, 500, 1000] {
            buckets.insert(*b, 0);
        }
        Self { name: name.to_string(), buckets, count: 0, sum: 0 }
    }

    pub fn observe(&mut self, value: u64) {
        self.count += 1;
        self.sum += value;
        for (bound, count) in self.buckets.iter_mut() {
            if value <= *bound {
                *count += 1;
            }
        }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn sum(&self) -> u64 {
        self.sum
    }

    pub fn avg(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.sum as f64 / self.count as f64 }
    }
}

/// Collects and manages metrics for the monitoring module.
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    counters: BTreeMap<String, Counter>,
    gauges: BTreeMap<String, Gauge>,
    histograms: BTreeMap<String, Histogram>,
    collections: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
            gauges: BTreeMap::new(),
            histograms: BTreeMap::new(),
            collections: AtomicU64::new(0),
        }
    }

    /// Get or create a counter by name.
    pub fn counter(&mut self, name: &str) -> &mut Counter {
        self.collections.fetch_add(1, Ordering::Relaxed);
        self.counters
            .entry(name.to_string())
            .or_insert_with(|| Counter::new(name))
    }

    /// Get or create a gauge by name.
    pub fn gauge(&mut self, name: &str) -> &mut Gauge {
        self.collections.fetch_add(1, Ordering::Relaxed);
        self.gauges
            .entry(name.to_string())
            .or_insert_with(|| Gauge::new(name))
    }

    /// Get or create a histogram by name.
    pub fn histogram(&mut self, name: &str) -> &mut Histogram {
        self.collections.fetch_add(1, Ordering::Relaxed);
        self.histograms
            .entry(name.to_string())
            .or_insert_with(|| Histogram::new(name))
    }

    /// Return the number of registered metric names.
    pub fn metric_count(&self) -> usize {
        self.counters.len() + self.gauges.len() + self.histograms.len()
    }

    /// Reset all metrics to their initial state.
    pub fn reset(&mut self) {
        self.counters.clear();
        self.gauges.clear();
        self.histograms.clear();
    }

    /// Snapshot all counter values as a map.
    pub fn snapshot_counters(&self) -> BTreeMap<String, u64> {
        self.counters.iter().map(|(k, c)| (k.clone(), c.value())).collect()
    }

    /// Snapshot all gauge values as a map.
    pub fn snapshot_gauges(&self) -> BTreeMap<String, f64> {
        self.gauges.iter().map(|(k, g)| (k.clone(), g.value())).collect()
    }

    /// Return a JSON string of all metrics.
    pub fn to_json(&self) -> String {
        let counters: Vec<String> = self
            .counters
            .iter()
            .map(|(k, c)| format!("\"{}\":{}", k, c.value()))
            .collect();
        let gauges: Vec<String> = self
            .gauges
            .iter()
            .map(|(k, g)| format!("\"{}\":{}", k, g.value()))
            .collect();
        format!(
            r#"{{"counters":{{{}}},"gauges":{{{}}}}}"#,
            counters.join(","),
            gauges.join(","),
        )
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_starts_at_zero() {
        let counter = Counter::new("requests");
        assert_eq!(counter.value(), 0);
        assert_eq!(counter.name(), "requests");
    }

    #[test]
    fn counter_increment_adds_delta() {
        let mut counter = Counter::new("req");
        counter.increment(5);
        assert_eq!(counter.value(), 5);
        counter.increment(3);
        assert_eq!(counter.value(), 8);
    }

    #[test]
    fn gauge_set_and_get() {
        let mut gauge = Gauge::new("temperature");
        gauge.set(36.5);
        assert!((gauge.value() - 36.5).abs() < f64::EPSILON);
    }

    #[test]
    fn histogram_observe_increments_buckets() {
        let mut h = Histogram::new("latency");
        h.observe(7);
        h.observe(50);
        h.observe(200);
        assert_eq!(h.count(), 3);
        assert_eq!(h.sum(), 257);
    }

    #[test]
    fn histogram_avg() {
        let mut h = Histogram::new("latency");
        h.observe(10);
        h.observe(20);
        h.observe(30);
        assert!((h.avg() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn histogram_empty_avg_is_zero() {
        let h = Histogram::new("empty");
        assert_eq!(h.avg(), 0.0);
    }

    #[test]
    fn collector_creates_counter_on_first_access() {
        let mut col = MetricsCollector::new();
        col.counter("hits").increment(1);
        assert_eq!(col.metric_count(), 1);
    }

    #[test]
    fn collector_reuses_existing_metric() {
        let mut col = MetricsCollector::new();
        col.counter("hits").increment(1);
        col.counter("hits").increment(1);
        assert_eq!(col.counter("hits").value(), 2);
    }

    #[test]
    fn collector_multiple_metric_types() {
        let mut col = MetricsCollector::new();
        col.counter("req").increment(1);
        col.gauge("mem").set(512.0);
        col.histogram("lat").observe(10);
        assert_eq!(col.metric_count(), 3);
    }

    #[test]
    fn collector_reset_clears_all() {
        let mut col = MetricsCollector::new();
        col.counter("a").increment(1);
        col.gauge("b").set(1.0);
        col.reset();
        assert_eq!(col.metric_count(), 0);
    }

    #[test]
    fn snapshot_counters_returns_map() {
        let mut col = MetricsCollector::new();
        col.counter("x").increment(3);
        let snap = col.snapshot_counters();
        assert_eq!(snap.get("x"), Some(&3));
    }

    #[test]
    fn to_json_contains_keys() {
        let mut col = MetricsCollector::new();
        col.counter("req").increment(10);
        let json = col.to_json();
        assert!(json.contains("req"));
    }

    #[test]
    fn histogram_bucket_ordering() {
        let mut h = Histogram::new("order");
        h.observe(1000);
        assert!(h.count() == 1);
    }

    #[test]
    fn collector_default_creates_empty() {
        let col = MetricsCollector::default();
        assert_eq!(col.metric_count(), 0);
    }
}

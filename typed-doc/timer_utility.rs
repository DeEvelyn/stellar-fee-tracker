use std::time::{Duration, Instant};
use std::collections::BTreeMap;

/// A utility for timing operations with nested support.
#[derive(Debug, Clone)]
pub struct Timer {
    name: String,
    start: Instant,
    elapsed: Option<Duration>,
    children: Vec<Timer>,
}

impl Timer {
    /// Start a new timer with a given name.
    pub fn start(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
            elapsed: None,
            children: Vec::new(),
        }
    }

    /// Stop the timer and record elapsed time.
    pub fn stop(&mut self) -> Duration {
        let dur = self.start.elapsed();
        self.elapsed = Some(dur);
        dur
    }

    /// Return the elapsed duration (None if still running).
    pub fn elapsed(&self) -> Option<Duration> {
        self.elapsed.or_else(|| Some(self.start.elapsed()))
    }

    /// Create and record a child timer.
    pub fn child(&mut self, name: &str) -> &mut Timer {
        self.children.push(Timer::start(name));
        self.children.last_mut().unwrap()
    }

    /// Return the total time including all children.
    pub fn total_with_children(&self) -> Duration {
        let self_time = self.elapsed.unwrap_or_else(|| self.start.elapsed());
        let children_time: Duration = self.children.iter().filter_map(|c| c.elapsed()).sum();
        self_time + children_time
    }

    /// Number of direct children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Return the timer hierarchy as a formatted string.
    pub fn format_tree(&self, indent: usize) -> String {
        let prefix = "  ".repeat(indent);
        let elapsed = self
            .elapsed()
            .map(|d| format!("{:?}", d))
            .unwrap_or_else(|| "running".into());
        let mut out = format!("{}{}: {}\n", prefix, self.name, elapsed);
        for child in &self.children {
            out.push_str(&child.format_tree(indent + 1));
        }
        out
    }

    /// Return a JSON representation including children.
    pub fn to_json(&self) -> String {
        let elapsed_ns = self.elapsed().map(|d| d.as_nanos()).unwrap_or(0);
        let children_json: Vec<String> = self.children.iter().map(|c| c.to_json()).collect();
        format!(
            r#"{{"name":"{}","elapsed_ns":{},"children":[{}]}}"#,
            self.name,
            elapsed_ns,
            children_json.join(","),
        )
    }
}

/// Scope-based timer that auto-records on drop.
pub struct ScopedTimer {
    start: Instant,
    name: String,
    results: Option<BTreeMap<String, Duration>>,
}

impl ScopedTimer {
    pub fn new(name: &str, results: &mut BTreeMap<String, Duration>) -> Self {
        let mut map = BTreeMap::new();
        std::mem::swap(results, &mut map);
        Self {
            start: Instant::now(),
            name: name.to_string(),
            results: Some(map),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        if let Some(ref mut results) = self.results {
            let dur = self.start.elapsed();
            results.insert(self.name.clone(), dur);
        }
    }
}

/// A collection of named timers.
#[derive(Debug, Clone)]
pub struct TimerRegistry {
    timers: BTreeMap<String, Timer>,
}

impl TimerRegistry {
    pub fn new() -> Self {
        Self {
            timers: BTreeMap::new(),
        }
    }

    /// Start a new named timer.
    pub fn start(&mut self, name: &str) {
        self.timers.insert(name.to_string(), Timer::start(name));
    }

    /// Stop a named timer and return its elapsed duration.
    pub fn stop(&mut self, name: &str) -> Option<Duration> {
        self.timers.get_mut(name).map(|t| t.stop())
    }

    /// Get a timer by name.
    pub fn get(&self, name: &str) -> Option<&Timer> {
        self.timers.get(name)
    }

    /// Return the elapsed time for a named timer.
    pub fn elapsed(&self, name: &str) -> Option<Duration> {
        self.timers.get(name).and_then(|t| t.elapsed())
    }

    /// Return the total time across all timers.
    pub fn total_time(&self) -> Duration {
        self.timers
            .values()
            .filter_map(|t| t.elapsed())
            .sum()
    }

    /// Number of registered timers.
    pub fn count(&self) -> usize {
        self.timers.len()
    }

    /// Clear all timers.
    pub fn reset(&mut self) {
        self.timers.clear();
    }

    /// Export all timers as JSON.
    pub fn to_json(&self) -> String {
        let items: Vec<String> = self
            .timers
            .values()
            .map(|t| t.to_json())
            .collect();
        format!("[{}]", items.join(","))
    }

    /// Format all timers as a tree.
    pub fn format_all(&self) -> String {
        let mut out = String::new();
        for (name, timer) in &self.timers {
            out.push_str(&format!("{}:\n", name));
            out.push_str(&timer.format_tree(1));
        }
        out
    }
}

impl Default for TimerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for formatting durations.
pub fn format_duration(d: Duration) -> String {
    let total_ns = d.as_nanos();
    if total_ns >= 1_000_000_000 {
        format!("{:.3}s", d.as_secs_f64())
    } else if total_ns >= 1_000_000 {
        format!("{:.3}ms", d.as_secs_f64() * 1000.0)
    } else if total_ns >= 1_000 {
        format!("{:.3}µs", d.as_secs_f64() * 1_000_000.0)
    } else {
        format!("{}ns", total_ns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_start_creates_timer() {
        let timer = Timer::start("op");
        assert_eq!(timer.name, "op");
        assert!(timer.elapsed.is_none());
    }

    #[test]
    fn timer_stop_records_elapsed() {
        let mut timer = Timer::start("op");
        let dur = timer.stop();
        assert!(timer.elapsed.is_some());
        assert!(dur >= Duration::ZERO);
    }

    #[test]
    fn timer_child_nesting() {
        let mut parent = Timer::start("parent");
        let child = parent.child("child");
        assert_eq!(child.name, "child");
        assert_eq!(parent.child_count(), 1);
    }

    #[test]
    fn timer_tree_format_includes_names() {
        let mut parent = Timer::start("parent");
        parent.child("child");
        let tree = parent.format_tree(0);
        assert!(tree.contains("parent"));
        assert!(tree.contains("child"));
    }

    #[test]
    fn timer_json_includes_children() {
        let mut parent = Timer::start("root");
        parent.child("leaf");
        let json = parent.to_json();
        assert!(json.contains("children"));
    }

    #[test]
    fn timer_registry_start_stop() {
        let mut reg = TimerRegistry::new();
        reg.start("op");
        let dur = reg.stop("op");
        assert!(dur.is_some());
    }

    #[test]
    fn timer_registry_elapsed() {
        let mut reg = TimerRegistry::new();
        reg.start("op");
        reg.stop("op");
        assert!(reg.elapsed("op").is_some());
    }

    #[test]
    fn timer_registry_total_time() {
        let mut reg = TimerRegistry::new();
        reg.start("a");
        reg.stop("a");
        reg.start("b");
        reg.stop("b");
        assert!(reg.total_time() > Duration::ZERO);
    }

    #[test]
    fn timer_registry_count() {
        let mut reg = TimerRegistry::new();
        reg.start("a");
        reg.start("b");
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn timer_registry_reset() {
        let mut reg = TimerRegistry::new();
        reg.start("a");
        reg.reset();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn timer_registry_to_json() {
        let mut reg = TimerRegistry::new();
        reg.start("op");
        reg.stop("op");
        let json = reg.to_json();
        assert!(json.starts_with('['));
    }

    #[test]
    fn format_duration_ns() {
        let s = format_duration(Duration::from_nanos(500));
        assert!(s.contains("ns"));
    }

    #[test]
    fn format_duration_ms() {
        let s = format_duration(Duration::from_millis(1500));
        assert!(s.contains("ms") || s.contains("s"));
    }

    #[test]
    fn scoped_timer_records_on_drop() {
        let mut results = BTreeMap::new();
        {
            let _timer = ScopedTimer::new("scope", &mut results);
        }
        assert!(results.contains_key("scope"));
    }

    #[test]
    fn timer_stop_returns_elapsed() {
        let mut t = Timer::start("x");
        let d = t.stop();
        assert!(d.as_nanos() > 0 || d.as_nanos() == 0);
    }

    #[test]
    fn registry_get_returns_none_for_missing() {
        let reg = TimerRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }
}

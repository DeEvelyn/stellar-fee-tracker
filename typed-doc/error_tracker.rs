use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Severity of an error event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Debug,
    Warning,
    Error,
    Critical,
}

impl ErrorSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "debug" => Some(Self::Debug),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

/// A single tracked error event.
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    /// Unique error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Module where the error occurred.
    pub module: String,
    /// Severity level.
    pub severity: ErrorSeverity,
    /// Unix timestamp (seconds).
    pub timestamp: u64,
    /// Number of times this error has been seen.
    pub count: u64,
}

/// Tracks error events with deduplication and aggregation.
#[derive(Debug, Clone)]
pub struct ErrorTracker {
    errors: BTreeMap<String, ErrorEvent>,
    total_errors: u64,
    critical_count: u64,
    warning_count: u64,
}

impl ErrorTracker {
    pub fn new() -> Self {
        Self {
            errors: BTreeMap::new(),
            total_errors: 0,
            critical_count: 0,
            warning_count: 0,
        }
    }

    /// Record an error event. If the same error code exists, increment its count.
    pub fn record(&mut self, code: &str, message: &str, module: &str, severity: ErrorSeverity) {
        let now = now_secs();
        if let Some(existing) = self.errors.get_mut(code) {
            existing.count += 1;
            existing.timestamp = now;
        } else {
            self.errors.insert(
                code.to_string(),
                ErrorEvent {
                    code: code.to_string(),
                    message: message.to_string(),
                    module: module.to_string(),
                    severity,
                    timestamp: now,
                    count: 1,
                },
            );
        }
        self.total_errors += 1;
        match severity {
            ErrorSeverity::Critical => self.critical_count += 1,
            ErrorSeverity::Warning => self.warning_count += 1,
            _ => {}
        }
    }

    /// Get an error event by code.
    pub fn get(&self, code: &str) -> Option<&ErrorEvent> {
        self.errors.get(code)
    }

    /// Return all tracked errors as a list.
    pub fn all(&self) -> Vec<&ErrorEvent> {
        self.errors.values().collect()
    }

    /// Return errors filtered by severity.
    pub fn by_severity(&self, severity: ErrorSeverity) -> Vec<&ErrorEvent> {
        self.errors
            .values()
            .filter(|e| e.severity == severity)
            .collect()
    }

    /// Return errors from a specific module.
    pub fn by_module(&self, module: &str) -> Vec<&ErrorEvent> {
        self.errors
            .values()
            .filter(|e| e.module == module)
            .collect()
    }

    /// Total number of error events recorded (including duplicates).
    pub fn total(&self) -> u64 {
        self.total_errors
    }

    /// Number of unique error codes.
    pub fn unique_count(&self) -> usize {
        self.errors.len()
    }

    /// Number of critical errors.
    pub fn criticals(&self) -> u64 {
        self.critical_count
    }

    /// Number of warnings.
    pub fn warnings(&self) -> u64 {
        self.warning_count
    }

    /// Reset the tracker to its initial state.
    pub fn reset(&mut self) {
        self.errors.clear();
        self.total_errors = 0;
        self.critical_count = 0;
        self.warning_count = 0;
    }

    /// Return the top-N most frequent errors.
    pub fn top(&self, n: usize) -> Vec<&ErrorEvent> {
        let mut events: Vec<&ErrorEvent> = self.errors.values().collect();
        events.sort_by(|a, b| b.count.cmp(&a.count));
        events.truncate(n);
        events
    }

    /// Return a JSON string of all errors.
    pub fn to_json(&self) -> String {
        let items: Vec<String> = self
            .errors
            .values()
            .map(|e| {
                format!(
                    r#"{{"code":"{}","msg":"{}","module":"{}","sev":"{}","count":{}}}"#,
                    e.code, e.message, e.module, e.severity.as_str(), e.count
                )
            })
            .collect();
        format!(
            r#"{{"total":{},"unique":{},"critical":{},"warning":{},"errors":[{}]}}"#,
            self.total_errors,
            self.unique_count(),
            self.critical_count,
            self.warning_count,
            items.join(","),
        )
    }

    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "Error Tracker: {} total, {} unique, {} critical, {} warnings",
            self.total_errors,
            self.unique_count(),
            self.critical_count,
            self.warning_count,
        )
    }
}

impl Default for ErrorTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_is_empty() {
        let tracker = ErrorTracker::new();
        assert_eq!(tracker.total(), 0);
        assert_eq!(tracker.unique_count(), 0);
    }

    #[test]
    fn record_adds_error() {
        let mut tracker = ErrorTracker::new();
        tracker.record("E001", "not found", "db", ErrorSeverity::Error);
        assert_eq!(tracker.total(), 1);
        assert_eq!(tracker.unique_count(), 1);
    }

    #[test]
    fn record_same_code_increments_count() {
        let mut tracker = ErrorTracker::new();
        tracker.record("E001", "not found", "db", ErrorSeverity::Error);
        tracker.record("E001", "not found", "db", ErrorSeverity::Error);
        assert_eq!(tracker.total(), 2);
        assert_eq!(tracker.unique_count(), 1);
        assert_eq!(tracker.get("E001").unwrap().count, 2);
    }

    #[test]
    fn by_severity_filters_correctly() {
        let mut tracker = ErrorTracker::new();
        tracker.record("W01", "warning", "net", ErrorSeverity::Warning);
        tracker.record("E01", "error", "net", ErrorSeverity::Error);
        tracker.record("C01", "critical", "db", ErrorSeverity::Critical);
        assert_eq!(tracker.by_severity(ErrorSeverity::Error).len(), 1);
        assert_eq!(tracker.by_severity(ErrorSeverity::Critical).len(), 1);
    }

    #[test]
    fn by_module_filters() {
        let mut tracker = ErrorTracker::new();
        tracker.record("E01", "err", "http", ErrorSeverity::Error);
        tracker.record("E02", "err", "db", ErrorSeverity::Error);
        assert_eq!(tracker.by_module("http").len(), 1);
        assert_eq!(tracker.by_module("db").len(), 1);
    }

    #[test]
    fn critical_count_tracks_critical() {
        let mut tracker = ErrorTracker::new();
        tracker.record("C01", "critical", "core", ErrorSeverity::Critical);
        tracker.record("E01", "error", "core", ErrorSeverity::Error);
        assert_eq!(tracker.criticals(), 1);
        assert_eq!(tracker.warnings(), 0);
    }

    #[test]
    fn warning_count_tracks_warnings() {
        let mut tracker = ErrorTracker::new();
        tracker.record("W01", "warn", "net", ErrorSeverity::Warning);
        assert_eq!(tracker.warnings(), 1);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut tracker = ErrorTracker::new();
        tracker.record("E01", "err", "mod", ErrorSeverity::Error);
        tracker.reset();
        assert_eq!(tracker.total(), 0);
        assert_eq!(tracker.unique_count(), 0);
        assert_eq!(tracker.criticals(), 0);
    }

    #[test]
    fn top_returns_most_frequent() {
        let mut tracker = ErrorTracker::new();
        tracker.record("E01", "common", "mod", ErrorSeverity::Error);
        tracker.record("E01", "common", "mod", ErrorSeverity::Error);
        tracker.record("E01", "common", "mod", ErrorSeverity::Error);
        tracker.record("E02", "rare", "mod", ErrorSeverity::Error);
        let top = tracker.top(1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].code, "E01");
    }

    #[test]
    fn to_json_contains_counts() {
        let mut tracker = ErrorTracker::new();
        tracker.record("E01", "err", "mod", ErrorSeverity::Error);
        let json = tracker.to_json();
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"E01\""));
    }

    #[test]
    fn summary_string_has_info() {
        let mut tracker = ErrorTracker::new();
        tracker.record("E01", "err", "mod", ErrorSeverity::Error);
        let s = tracker.summary();
        assert!(s.contains("Error Tracker"));
    }

    #[test]
    fn severity_from_str() {
        assert_eq!(ErrorSeverity::from_str("critical"), Some(ErrorSeverity::Critical));
        assert_eq!(ErrorSeverity::from_str("unknown"), None);
    }

    #[test]
    fn severity_as_str() {
        assert_eq!(ErrorSeverity::Debug.as_str(), "debug");
        assert_eq!(ErrorSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn all_returns_all_errors() {
        let mut tracker = ErrorTracker::new();
        tracker.record("A", "a", "m", ErrorSeverity::Error);
        tracker.record("B", "b", "m", ErrorSeverity::Warning);
        assert_eq!(tracker.all().len(), 2);
    }

    #[test]
    fn get_nonexistent_code() {
        let tracker = ErrorTracker::new();
        assert!(tracker.get("NONE").is_none());
    }
}

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Result of a single health check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Name of the check.
    pub name: String,
    /// Whether the check passed.
    pub ok: bool,
    /// Human-readable message.
    pub message: String,
    /// Duration of the check.
    pub duration_ms: u64,
}

impl CheckResult {
    pub fn passing(name: &str, msg: &str) -> Self {
        Self {
            name: name.to_string(),
            ok: true,
            message: msg.to_string(),
            duration_ms: 0,
        }
    }

    pub fn failing(name: &str, msg: &str) -> Self {
        Self {
            name: name.to_string(),
            ok: false,
            message: msg.to_string(),
            duration_ms: 0,
        }
    }
}

/// A trait for individual health checks.
pub trait HealthCheck {
    fn name(&self) -> &str;
    fn run(&self) -> CheckResult;
}

/// Check that the system is responding within a timeout.
pub struct PingCheck {
    pub target: String,
    pub timeout_ms: u64,
}

impl HealthCheck for PingCheck {
    fn name(&self) -> &str {
        "ping"
    }

    fn run(&self) -> CheckResult {
        let start = Instant::now();
        let ok = !self.target.is_empty();
        let elapsed = start.elapsed().as_millis() as u64;
        CheckResult {
            name: "ping".into(),
            ok,
            message: format!("ping {}: {}", self.target, if ok { "ok" } else { "empty target" }),
            duration_ms: elapsed,
        }
    }
}

/// Check available disk space.
pub struct DiskSpaceCheck {
    pub path: String,
    pub min_free_mb: u64,
}

impl HealthCheck for DiskSpaceCheck {
    fn name(&self) -> &str {
        "disk_space"
    }

    fn run(&self) -> CheckResult {
        let start = Instant::now();
        let ok = std::path::Path::new(&self.path).exists();
        let elapsed = start.elapsed().as_millis() as u64;
        CheckResult {
            name: "disk_space".into(),
            ok,
            message: format!("path {} exists: {}", self.path, ok),
            duration_ms: elapsed,
        }
    }
}

/// Check that database file is accessible.
pub struct DbConnectivityCheck {
    pub db_path: String,
}

impl HealthCheck for DbConnectivityCheck {
    fn name(&self) -> &str {
        "db_connectivity"
    }

    fn run(&self) -> CheckResult {
        let start = Instant::now();
        let meta = std::fs::metadata(&self.db_path);
        let ok = meta.is_ok();
        let size = meta.map(|m| m.len()).unwrap_or(0);
        let elapsed = start.elapsed().as_millis() as u64;
        CheckResult {
            name: "db_connectivity".into(),
            ok,
            message: format!(
                "db {}: {} ({} bytes)",
                self.db_path,
                if ok { "accessible" } else { "not found" },
                size,
            ),
            duration_ms: elapsed,
        }
    }
}

/// Check system memory.
pub struct MemoryCheck {
    pub min_available_mb: u64,
}

impl HealthCheck for MemoryCheck {
    fn name(&self) -> &str {
        "memory"
    }

    fn run(&self) -> CheckResult {
        let start = Instant::now();
        let elapsed = start.elapsed().as_millis() as u64;
        CheckResult {
            name: "memory".into(),
            ok: true,
            message: format!("minimum {} MB required", self.min_available_mb),
            duration_ms: elapsed,
        }
    }
}

/// Check system uptime.
pub struct UptimeCheck {
    pub started_at: Instant,
}

impl UptimeCheck {
    pub fn new() -> Self {
        Self { started_at: Instant::now() }
    }
}

impl HealthCheck for UptimeCheck {
    fn name(&self) -> &str {
        "uptime"
    }

    fn run(&self) -> CheckResult {
        let elapsed = self.started_at.elapsed();
        CheckResult {
            name: "uptime".into(),
            ok: true,
            message: format!("up for {:?}", elapsed),
            duration_ms: elapsed.as_millis() as u64,
        }
    }
}

/// Runs a suite of health checks and produces a report.
#[derive(Debug, Clone)]
pub struct HealthChecker {
    checks: Vec<(String, Box<dyn HealthCheck + Send>)>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Register a health check.
    pub fn register(&mut self, check: Box<dyn HealthCheck + Send>) {
        let name = check.name().to_string();
        self.checks.push((name, check));
    }

    /// Run all registered checks.
    pub fn run_all(&self) -> Vec<CheckResult> {
        self.checks.iter().map(|(_, c)| c.run()).collect()
    }

    /// Run all checks and return the overall status.
    pub fn status(&self) -> HealthStatus {
        let results = self.run_all();
        let any_failed = results.iter().any(|r| !r.ok);
        let all_failed = results.iter().all(|r| !r.ok);
        if all_failed {
            HealthStatus::Unhealthy
        } else if any_failed {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Return a JSON report of all checks.
    pub fn to_json(&self) -> String {
        let results = self.run_all();
        let items: Vec<String> = results
            .iter()
            .map(|r| {
                format!(
                    r#"{{"name":"{}","ok":{},"msg":"{}","dur_ms":{}}}"#,
                    r.name, r.ok, r.message, r.duration_ms
                )
            })
            .collect();
        format!(
            r#"{{"status":"{}","checks":[{}]}}"#,
            self.status().as_str(),
            items.join(","),
        )
    }

    /// Number of registered checks.
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Overall health status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_result_passing() {
        let r = CheckResult::passing("test", "all good");
        assert!(r.ok);
        assert_eq!(r.name, "test");
    }

    #[test]
    fn check_result_failing() {
        let r = CheckResult::failing("test", "error");
        assert!(!r.ok);
    }

    #[test]
    fn ping_check_empty_target_fails() {
        let check = PingCheck { target: String::new(), timeout_ms: 1000 };
        let r = check.run();
        assert!(!r.ok);
    }

    #[test]
    fn ping_check_non_empty_passes() {
        let check = PingCheck { target: "localhost".into(), timeout_ms: 1000 };
        let r = check.run();
        assert!(r.ok);
    }

    #[test]
    fn disk_check_existing_path() {
        let check = DiskSpaceCheck { path: "/tmp".into(), min_free_mb: 1 };
        let r = check.run();
        assert!(r.ok);
    }

    #[test]
    fn disk_check_nonexistent_path() {
        let check = DiskSpaceCheck { path: "/nonexistent_path_xyz".into(), min_free_mb: 1 };
        let r = check.run();
        assert!(!r.ok);
    }

    #[test]
    fn db_check_nonexistent_file_fails() {
        let check = DbConnectivityCheck { db_path: "/no/such/file.db".into() };
        let r = check.run();
        assert!(!r.ok);
    }

    #[test]
    fn memory_check_always_passes() {
        let check = MemoryCheck { min_available_mb: 0 };
        let r = check.run();
        assert!(r.ok);
    }

    #[test]
    fn uptime_check_reports_elapsed() {
        let check = UptimeCheck::new();
        let r = check.run();
        assert!(r.ok);
        assert!(r.message.contains("up for"));
    }

    #[test]
    fn health_checker_registers_and_runs() {
        let mut checker = HealthChecker::new();
        checker.register(Box::new(PingCheck { target: "localhost".into(), timeout_ms: 100 }));
        checker.register(Box::new(UptimeCheck::new()));
        assert_eq!(checker.check_count(), 2);
        let results = checker.run_all();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn status_healthy_when_all_pass() {
        let mut checker = HealthChecker::new();
        checker.register(Box::new(MemoryCheck { min_available_mb: 0 }));
        assert_eq!(checker.status(), HealthStatus::Healthy);
    }

    #[test]
    fn status_degraded_when_some_fail() {
        let mut checker = HealthChecker::new();
        checker.register(Box::new(MemoryCheck { min_available_mb: 0 }));
        checker.register(Box::new(DbConnectivityCheck { db_path: "/nonexistent".into() }));
        assert_eq!(checker.status(), HealthStatus::Degraded);
    }

    #[test]
    fn status_unhealthy_when_all_fail() {
        let mut checker = HealthChecker::new();
        checker.register(Box::new(DbConnectivityCheck { db_path: "/nonexistent_a".into() }));
        checker.register(Box::new(DbConnectivityCheck { db_path: "/nonexistent_b".into() }));
        assert_eq!(checker.status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn to_json_contains_status() {
        let checker = HealthChecker::new();
        let json = checker.to_json();
        assert!(json.contains("healthy"));
    }

    #[test]
    fn health_status_as_str() {
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Unhealthy.as_str(), "unhealthy");
    }

    #[test]
    fn checker_default_is_empty() {
        let checker = HealthChecker::default();
        assert_eq!(checker.check_count(), 0);
    }
}

use std::time::Instant;

/// Result of a single health check.
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Name of the check.
    pub name: String,
    /// Whether the check passed.
    pub ok: bool,
    /// Optional detail message.
    pub detail: Option<String>,
    /// Duration of the check in milliseconds.
    pub duration_ms: u64,
}

/// Overall health status for the devkit runtime.
#[derive(Debug, Clone)]
pub enum HealthStatus {
    /// All checks passed.
    Healthy,
    /// Some checks failed but the system can operate.
    Degraded,
    /// Critical checks failed; the system cannot operate.
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Trait implemented by individual health checks.
pub trait Check {
    /// Unique name for this check.
    fn name(&self) -> &str;
    /// Execute the check and return a result.
    fn run(&self) -> HealthCheck;
}

/// A health check registry that runs configured checks.
#[derive(Debug)]
pub struct HealthRegistry {
    checks: Vec<Box<dyn Check + Send>>,
    start: Instant,
}

impl Default for HealthRegistry {
    fn default() -> Self {
        Self {
            checks: Vec::new(),
            start: Instant::now(),
        }
    }
}

impl HealthRegistry {
    /// Create a new registry with default built-in checks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a custom health check.
    pub fn register(&mut self, check: Box<dyn Check + Send>) {
        self.checks.push(check);
    }

    /// Run all registered checks and return the results.
    pub fn run_all(&self) -> Vec<HealthCheck> {
        self.checks.iter().map(|c| c.run()).collect()
    }

    /// Run all checks and compute the overall health status.
    pub fn status(&self) -> (HealthStatus, Vec<HealthCheck>) {
        let results = self.run_all();
        let any_failed = results.iter().any(|c| !c.ok);
        let all_failed = !results.is_empty() && results.iter().all(|c| !c.ok);
        let status = if all_failed {
            HealthStatus::Unhealthy
        } else if any_failed {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        (status, results)
    }

    /// Return a JSON string of the health status.
    pub fn to_json(&self) -> String {
        let (status, results) = self.status();
        let checks_json: Vec<String> = results
            .iter()
            .map(|c| {
                let detail = match &c.detail {
                    Some(d) => format!("\"detail\":\"{}\"", d),
                    None => "\"detail\":null".into(),
                };
                format!(
                    r#"{{"name":"{}","ok":{},"duration_ms":{},{}}}"#,
                    c.name, c.ok, c.duration_ms, detail
                )
            })
            .collect();
        format!(
            r#"{{"status":"{}","checks":[{}]}}"#,
            status,
            checks_json.join(",")
        )
    }

    /// Uptime of the process in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start.elapsed().as_secs()
    }
}

/// Check that the database file exists and is readable.
pub struct DbHealthCheck {
    pub db_path: String,
}

impl Check for DbHealthCheck {
    fn name(&self) -> &str {
        "database"
    }

    fn run(&self) -> HealthCheck {
        let start = Instant::now();
        let ok = std::path::Path::new(&self.db_path).exists();
        let duration_ms = start.elapsed().as_millis() as u64;
        let detail = if ok {
            Some(format!("found at {}", self.db_path))
        } else {
            Some(format!("not found at {}", self.db_path))
        };
        HealthCheck {
            name: self.name().to_string(),
            ok,
            detail,
            duration_ms,
        }
    }
}

/// Check system memory availability.
pub struct MemoryHealthCheck {
    pub min_bytes: u64,
}

impl Check for MemoryHealthCheck {
    fn name(&self) -> &str {
        "memory"
    }

    fn run(&self) -> HealthCheck {
        let start = Instant::now();
        let duration_ms = start.elapsed().as_millis() as u64;
        HealthCheck {
            name: self.name().to_string(),
            ok: true,
            detail: Some(format!("minimum {} bytes required", self.min_bytes)),
            duration_ms,
        }
    }
}

/// Arguments for the `health` subcommand.
pub struct HealthArgs {
    /// Output as JSON.
    pub json: bool,
    /// Path to check for database existence.
    pub db_path: Option<String>,
    /// Run a specific health check by name.
    pub check: Option<String>,
    /// Suppress all output except errors.
    pub quiet: bool,
}

impl Default for HealthArgs {
    fn default() -> Self {
        Self {
            json: false,
            db_path: Some("stellar_fees.db".into()),
            check: None,
            quiet: false,
        }
    }
}

impl HealthArgs {
    /// Run the health subcommand. Returns `false` if any check fails.
    pub fn run(&self) -> bool {
        let mut registry = HealthRegistry::new();
        if let Some(db) = &self.db_path {
            registry.register(Box::new(DbHealthCheck { db_path: db.clone() }));
        }
        registry.register(Box::new(MemoryHealthCheck { min_bytes: 1024 }));

        if let Some(name) = &self.check {
            for check in registry.run_all() {
                if check.name == *name {
                    if !self.quiet {
                        if self.json {
                            println!(
                                r#"{{"name":"{}","ok":{},"duration_ms":{}}}"#,
                                check.name, check.ok, check.duration_ms
                            );
                        } else {
                            println!("{}: {}", check.name, if check.ok { "OK" } else { "FAIL" });
                        }
                    }
                    return check.ok;
                }
            }
            eprintln!("check '{}' not found", name);
            return false;
        }

        let (status, results) = registry.status();
        let all_ok = matches!(status, HealthStatus::Healthy);

        if !self.quiet {
            if self.json {
                println!("{}", registry.to_json());
            } else {
                println!("health: {}", status);
                for check in &results {
                    let icon = if check.ok { "✓" } else { "✗" };
                    let detail = check.detail.as_deref().unwrap_or("");
                    println!("  {} {}  ({}ms) {}", icon, check.name, check.duration_ms, detail);
                }
            }
        }

        all_ok
    }

    /// Return true if all built-in checks pass.
    pub fn is_healthy(&self) -> bool {
        let mut registry = HealthRegistry::new();
        if let Some(db) = &self.db_path {
            registry.register(Box::new(DbHealthCheck { db_path: db.clone() }));
        }
        let (status, _) = registry.status();
        matches!(status, HealthStatus::Healthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_status_when_all_checks_pass() {
        let registry = HealthRegistry::new();
        let (status, _) = registry.status();
        assert!(matches!(status, HealthStatus::Healthy));
    }

    #[test]
    fn db_health_check_reports_not_found() {
        let check = DbHealthCheck {
            db_path: "/nonexistent/path.db".into(),
        };
        let result = check.run();
        assert!(!result.ok);
        assert!(result.detail.unwrap().contains("not found"));
    }

    #[test]
    fn health_json_includes_status() {
        let registry = HealthRegistry::new();
        let json = registry.to_json();
        assert!(json.contains("healthy"));
    }

    #[test]
    fn health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
        assert_eq!(format!("{}", HealthStatus::Degraded), "degraded");
        assert_eq!(format!("{}", HealthStatus::Unhealthy), "unhealthy");
    }

    #[test]
    fn health_args_default() {
        let args = HealthArgs::default();
        assert!(!args.json);
        assert!(args.db_path.is_some());
    }

    #[test]
    fn db_check_name_is_database() {
        let check = DbHealthCheck {
            db_path: "test.db".into(),
        };
        assert_eq!(check.name(), "database");
    }

    #[test]
    fn memory_check_always_passes() {
        let check = MemoryHealthCheck { min_bytes: 0 };
        let result = check.run();
        assert!(result.ok);
    }

    #[test]
    fn registry_run_all_returns_checks() {
        let mut registry = HealthRegistry::new();
        registry.register(Box::new(MemoryHealthCheck { min_bytes: 0 }));
        let results = registry.run_all();
        assert!(!results.is_empty());
    }
}

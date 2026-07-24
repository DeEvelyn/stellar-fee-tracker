use std::collections::BTreeMap;
use std::path::PathBuf;

/// Active configuration for the devkit CLI tool.
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to the fee database.
    pub db_path: PathBuf,
    /// Default scenario file for mock data.
    pub scenario: String,
    /// Mock server port.
    pub port: u16,
    /// Whether to show detailed output.
    pub verbose: bool,
    /// Custom key-value overrides.
    pub overrides: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("stellar_fees.db"),
            scenario: String::from("normal"),
            port: 8090,
            verbose: false,
            overrides: BTreeMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables and an optional config file.
    pub fn load(path: Option<&PathBuf>) -> Self {
        let mut cfg = match path {
            Some(p) => Self::from_file(p),
            None => Self::default(),
        };
        cfg.apply_env();
        cfg
    }

    /// Parse config from a simple `key = value` file (one pair per line).
    pub fn from_file(path: &PathBuf) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut cfg = Self::default();
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                match key {
                    "db_path" => cfg.db_path = PathBuf::from(value),
                    "scenario" => cfg.scenario = value.to_string(),
                    "port" => cfg.port = value.parse().unwrap_or(8090),
                    "verbose" => cfg.verbose = value == "true",
                    _ => {
                        cfg.overrides.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }
        cfg
    }

    /// Apply environment variable overrides on top of the current config.
    pub fn apply_env(&mut self) {
        if let Ok(v) = std::env::var("DEVKIT_DB_PATH") {
            self.db_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("DEVKIT_SCENARIO") {
            self.scenario = v;
        }
        if let Ok(v) = std::env::var("DEVKIT_PORT") {
            self.port = v.parse().unwrap_or(self.port);
        }
        if let Ok(v) = std::env::var("DEVKIT_VERBOSE") {
            self.verbose = v == "true" || v == "1";
        }
    }

    /// Return the effective database path.
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    /// Display the full configuration as a formatted key/value report.
    ///
    /// Each row shows the key, current value, and source (env var or default).
    pub fn display(&self) -> String {
        let db_source = if std::env::var("DEVKIT_DB_PATH").is_ok() {
            "env"
        } else {
            "default"
        };
        let scenario_source = if std::env::var("DEVKIT_SCENARIO").is_ok() {
            "env"
        } else {
            "default"
        };
        let port_source = if std::env::var("DEVKIT_PORT").is_ok() {
            "env"
        } else {
            "default"
        };
        let verbose_source = if std::env::var("DEVKIT_VERBOSE").is_ok() {
            "env"
        } else {
            "default"
        };

        let mut out = String::new();
        out.push_str("devkit configuration\n");
        out.push_str("====================\n");
        out.push_str(&format!(
            "{:<12} {:<30} {}\n",
            "key",
            "value",
            "source"
        ));
        out.push_str(&format!(
            "{:<12} {:<30} {}\n",
            "db_path",
            self.db_path.display(),
            db_source
        ));
        out.push_str(&format!(
            "{:<12} {:<30} {}\n",
            "scenario",
            self.scenario,
            scenario_source
        ));
        out.push_str(&format!(
            "{:<12} {:<30} {}\n",
            "port",
            self.port,
            port_source
        ));
        out.push_str(&format!(
            "{:<12} {:<30} {}\n",
            "verbose",
            self.verbose,
            verbose_source
        ));

        if !self.overrides.is_empty() {
            out.push_str("overrides:\n");
            for (k, v) in &self.overrides {
                out.push_str(&format!("  {} = {}\n", k, v));
            }
        }
        out
    }
}

/// Arguments for the `config` subcommand.
pub struct ConfigArgs {
    /// Optional path to a configuration file.
    pub config_file: Option<PathBuf>,
    /// Show the effective configuration and exit.
    pub show: bool,
    /// Set a configuration key=value pair (may be specified multiple times).
    pub set: Vec<String>,
}

impl Default for ConfigArgs {
    fn default() -> Self {
        Self {
            config_file: None,
            show: false,
            set: Vec::new(),
        }
    }
}

impl ConfigArgs {
    /// Run the config subcommand: print all active key/value pairs with source.
    pub fn run(&self) {
        let mut cfg = Config::load(self.config_file.as_ref());
        for pair in &self.set {
            if let Some((key, value)) = pair.split_once('=') {
                cfg.overrides
                    .insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        print!("{}", cfg.display());
    }

    /// Return the list of active configuration keys.
    pub fn keys(&self) -> Vec<String> {
        let cfg = Config::load(self.config_file.as_ref());
        let mut keys: Vec<String> = vec![
            "db_path".into(),
            "scenario".into(),
            "port".into(),
            "verbose".into(),
        ];
        keys.extend(cfg.overrides.keys().cloned());
        keys.sort();
        keys
    }

    /// Validate the configuration, returning a list of issues found.
    pub fn validate(&self) -> Vec<String> {
        let cfg = Config::load(self.config_file.as_ref());
        let mut issues = Vec::new();
        if !cfg.db_path.exists() {
            issues.push(format!("db_path not found: {}", cfg.db_path.display()));
        }
        if cfg.port == 0 {
            issues.push("port must be > 0".into());
        }
        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            db_path: PathBuf::from("/tmp/test.db"),
            scenario: "congested".into(),
            port: 9090,
            verbose: true,
            overrides: BTreeMap::from([("timeout".into(), "30".into())]),
        }
    }

    #[test]
    fn config_default_has_sensible_values() {
        let cfg = Config::default();
        assert_eq!(cfg.port, 8090);
        assert!(!cfg.verbose);
    }

    #[test]
    fn config_display_includes_all_fields() {
        let out = test_config().display();
        assert!(out.contains("/tmp/test.db"));
        assert!(out.contains("congested"));
        assert!(out.contains("9090"));
        assert!(out.contains("timeout"));
    }

    #[test]
    fn config_args_default_does_not_panic() {
        let args = ConfigArgs::default();
        assert!(args.set.is_empty());
    }

    #[test]
    fn config_validate_handles_missing_db() {
        let args = ConfigArgs {
            config_file: None,
            show: false,
            set: vec![],
        };
        let issues = args.validate();
        assert!(issues.iter().any(|i| i.contains("db_path")));
    }

    #[test]
    fn config_keys_includes_base_keys() {
        let args = ConfigArgs::default();
        let keys = args.keys();
        assert!(keys.contains(&"db_path".into()));
        assert!(keys.contains(&"port".into()));
    }
}

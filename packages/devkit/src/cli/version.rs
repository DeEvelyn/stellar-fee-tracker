/// Semantic version of the devkit crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build metadata collected at compile time via build.rs.
#[derive(Debug, Clone)]
pub struct BuildInfo {
    /// Crate version from Cargo.toml.
    pub version: String,
    /// Git commit SHA at build time.
    pub commit_sha: String,
    /// Unix timestamp of the build.
    pub build_time: String,
    /// Rust compiler version.
    pub rustc_version: String,
    /// Target triple.
    pub target: String,
    /// Whether the build was compiled in debug mode.
    pub debug: bool,
    /// Enabled feature flags.
    pub features: Vec<String>,
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self {
            version: VERSION.to_string(),
            commit_sha: option_env!("DEVKIT_COMMIT_SHA")
                .unwrap_or("unknown")
                .to_string(),
            build_time: option_env!("DEVKIT_BUILD_TIME")
                .unwrap_or("unknown")
                .to_string(),
            rustc_version: option_env!("DEVKIT_RUSTC_VERSION")
                .unwrap_or("unknown")
                .to_string(),
            target: option_env!("DEVKIT_TARGET")
                .unwrap_or("unknown")
                .to_string(),
            debug: option_env!("DEVKIT_DEBUG")
                .map(|v| v == "true")
                .unwrap_or(cfg!(debug_assertions)),
            features: Vec::new(),
        }
    }
}

impl BuildInfo {
    /// Format build info as a human-readable block.
    pub fn display(&self) -> String {
        let features = if self.features.is_empty() {
            "none".to_string()
        } else {
            self.features.join(", ")
        };
        format!(
            "devkit {}\n  commit:     {}\n  build time: {}\n  rustc:      {}\n  target:     {}\n  debug:      {}\n  features:   {}",
            self.version,
            self.commit_sha,
            self.build_time,
            self.rustc_version,
            self.target,
            self.debug,
            features,
        )
    }

    /// Format build info as a JSON object.
    pub fn to_json(&self) -> String {
        let features_json = self
            .features
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"version":"{}","commit":"{}","build_time":"{}","rustc":"{}","target":"{}","debug":{},"features":[{}]}}"#,
            self.version,
            self.commit_sha,
            self.build_time,
            self.rustc_version,
            self.target,
            self.debug,
            features_json,
        )
    }

    /// Compare whether this build is newer than `other` using semver logic.
    pub fn is_newer_than(&self, other: &str) -> bool {
        fn parse_version(v: &str) -> Vec<u64> {
            v.trim_start_matches('v')
                .split('.')
                .filter_map(|s| s.parse().ok())
                .collect()
        }
        let ours = parse_version(&self.version);
        let theirs = parse_version(other);
        for (a, b) in ours.iter().zip(theirs.iter()) {
            if a != b {
                return a > b;
            }
        }
        ours.len() > theirs.len()
    }
}

/// Arguments for the `version` subcommand.
pub struct VersionArgs {
    /// Output as JSON instead of plain text.
    pub json: bool,
    /// Check if the current version is newer than this version string.
    pub check: Option<String>,
}

impl Default for VersionArgs {
    fn default() -> Self {
        Self {
            json: false,
            check: None,
        }
    }
}

impl VersionArgs {
    /// Run the version subcommand, printing build info to stdout.
    pub fn run(&self) {
        let info = BuildInfo::default();
        if let Some(other) = &self.check {
            let newer = info.is_newer_than(other);
            println!("{}", if newer { "true" } else { "false" });
            return;
        }
        if self.json {
            println!("{}", info.to_json());
        } else {
            println!("{}", info.display());
        }
    }

    /// Return just the crate version string.
    pub fn version_only(&self) -> &'static str {
        VERSION
    }

    /// Check if a minimum version requirement is met.
    pub fn meets_minimum(&self, minimum: &str) -> bool {
        let info = BuildInfo::default();
        info.is_newer_than(minimum) || info.version == minimum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn build_info_default_contains_version() {
        let info = BuildInfo::default();
        assert!(info.display().contains(&info.version));
    }

    #[test]
    fn build_info_json_is_valid_object() {
        let json = BuildInfo {
            version: "1.0.0".into(),
            commit_sha: "abc123".into(),
            build_time: "1234567890".into(),
            rustc_version: "1.75.0".into(),
            target: "x86_64-linux".into(),
            debug: false,
            features: vec!["json".into()],
        }
        .to_json();
        assert!(json.starts_with('{'));
        assert!(json.contains("\"version\":\"1.0.0\""));
    }

    #[test]
    fn is_newer_than_returns_true_for_higher_major() {
        let info = BuildInfo {
            version: "2.0.0".into(),
            ..Default::default()
        };
        assert!(info.is_newer_than("1.9.9"));
    }

    #[test]
    fn is_newer_than_returns_false_for_lower() {
        let info = BuildInfo {
            version: "0.9.0".into(),
            ..Default::default()
        };
        assert!(!info.is_newer_than("1.0.0"));
    }

    #[test]
    fn is_newer_than_same_version() {
        let info = BuildInfo {
            version: "1.0.0".into(),
            ..Default::default()
        };
        assert!(!info.is_newer_than("1.0.0"));
    }

    #[test]
    fn version_args_default_does_not_panic() {
        let args = VersionArgs::default();
        assert!(!args.json);
    }

    #[test]
    fn version_only_returns_str() {
        let args = VersionArgs::default();
        assert!(!args.version_only().is_empty());
    }
}

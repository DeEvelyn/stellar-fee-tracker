use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Compression strategy for rotated logs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Compression {
    None,
    Gzip,
}

/// Rotation trigger type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationTrigger {
    /// Rotate when the log file reaches a certain size (bytes).
    Size(u64),
    /// Rotate after a certain number of seconds.
    Time(u64),
}

/// Configuration for log file rotation.
#[derive(Debug, Clone)]
pub struct LogRotationConfig {
    /// Directory for log files.
    pub output_dir: PathBuf,
    /// Base filename (without extension).
    pub base_name: String,
    /// Maximum size of a single log file before rotation (bytes).
    pub max_file_size: u64,
    /// Maximum number of archived log files to retain.
    pub max_files: u32,
    /// Compression strategy for archived logs.
    pub compression: Compression,
    /// Rotation trigger.
    pub trigger: RotationTrigger,
    /// Whether to include timestamps in archived filenames.
    pub timestamp_suffix: bool,
    /// File permission mode (Unix).
    pub file_mode: u32,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("logs"),
            base_name: "devkit".into(),
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_files: 5,
            compression: Compression::None,
            trigger: RotationTrigger::Size(10 * 1024 * 1024),
            timestamp_suffix: true,
            file_mode: 0o644,
        }
    }
}

impl LogRotationConfig {
    /// Validate the configuration, returning a list of issues.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.max_file_size == 0 {
            issues.push("max_file_size must be > 0".into());
        }
        if self.max_files == 0 {
            issues.push("max_files must be > 0".into());
        }
        if self.base_name.is_empty() {
            issues.push("base_name must not be empty".into());
        }
        issues
    }

    /// Generate the path for the current active log file.
    pub fn active_path(&self) -> PathBuf {
        self.output_dir.join(format!("{}.log", self.base_name))
    }

    /// Generate a path for an archived log file.
    pub fn archive_path(&self, index: u32) -> PathBuf {
        let ext = match self.compression {
            Compression::Gzip => "log.gz",
            Compression::None => "log",
        };
        if self.timestamp_suffix {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.output_dir.join(format!("{}.{}.{}.{}", self.base_name, ts, index, ext))
        } else {
            self.output_dir.join(format!("{}.{}.{}", self.base_name, index, ext))
        }
    }

    /// Ensure the output directory exists.
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.output_dir)
    }
}

/// A log file writer with automatic rotation support.
pub struct RotatingLogWriter {
    config: LogRotationConfig,
    current_size: AtomicU64,
    current_index: AtomicU64,
    writer: Option<fs::File>,
}

impl RotatingLogWriter {
    /// Create a new rotating log writer.
    pub fn new(config: LogRotationConfig) -> std::io::Result<Self> {
        config.ensure_dir()?;
        let path = config.active_path();
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(Self {
            current_size: AtomicU64::new(size),
            current_index: AtomicU64::new(0),
            writer: Some(file),
        })
    }

    /// Write a log line, rotating if necessary.
    pub fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        let line_size = line.len() as u64 + 1;
        let current = self.current_size.load(Ordering::Relaxed);

        if current + line_size > self.config.max_file_size {
            self.rotate()?;
        }

        if let Some(ref mut file) = self.writer {
            writeln!(file, "{}", line)?;
            file.flush()?;
            self.current_size.fetch_add(line_size, Ordering::Relaxed);
        }
        Ok(())
    }

    /// Rotate the log file.
    pub fn rotate(&mut self) -> std::io::Result<()> {
        if let Some(mut file) = self.writer.take() {
            file.flush()?;
        }

        let active = self.config.active_path();
        if active.exists() {
            let idx = self.current_index.fetch_add(1, Ordering::Relaxed) as u32;
            let archive = self.config.archive_path(idx);
            fs::rename(&active, &archive)?;

            // Enforce max_files retention policy
            if idx >= self.config.max_files {
                let oldest = self.config.archive_path(idx - self.config.max_files);
                let _ = fs::remove_file(&oldest);
            }
        }

        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active)?;
        self.current_size.store(0, Ordering::Relaxed);
        self.writer = Some(file);
        Ok(())
    }

    /// Flush and close the current log file.
    pub fn close(&mut self) -> std::io::Result<()> {
        if let Some(mut file) = self.writer.take() {
            file.flush()?;
        }
        Ok(())
    }

    /// Current log file size in bytes.
    pub fn size(&self) -> u64 {
        self.current_size.load(Ordering::Relaxed)
    }

    /// Number of rotations performed.
    pub fn rotation_count(&self) -> u64 {
        self.current_index.load(Ordering::Relaxed)
    }
}

/// Parse a log rotation configuration from a TOML-like string.
pub fn parse_rotation_config(content: &str) -> LogRotationConfig {
    let mut cfg = LogRotationConfig::default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "max_file_size" => {
                    if let Some(n) = parse_size(value) {
                        cfg.max_file_size = n;
                    }
                }
                "max_files" => {
                    if let Ok(n) = value.parse() {
                        cfg.max_files = n;
                    }
                }
                "compression" => {
                    cfg.compression = if value == "gzip" { Compression::Gzip } else { Compression::None };
                }
                "output_dir" => {
                    cfg.output_dir = PathBuf::from(value);
                }
                "base_name" => {
                    cfg.base_name = value.to_string();
                }
                _ => {}
            }
        }
    }
    cfg
}

/// Parse a human-readable size string (e.g. "10MB", "1GB") into bytes.
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();
    if let Some(n) = s.strip_suffix("gb") {
        n.parse::<u64>().ok().map(|v| v * 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("mb") {
        n.parse::<u64>().ok().map(|v| v * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("kb") {
        n.parse::<u64>().ok().map(|v| v * 1024)
    } else if let Some(n) = s.strip_suffix('b') {
        n.parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}

/// Arguments for the log rotation subcommand.
pub struct LogRotationArgs {
    /// Max file size before rotation (e.g. "10MB").
    pub max_size: String,
    /// Number of archive files to retain.
    pub max_files: u32,
    /// Output directory for logs.
    pub output_dir: Option<String>,
    /// Show current rotation configuration.
    pub show: bool,
    /// Validate the configuration and report issues.
    pub check: bool,
}

impl Default for LogRotationArgs {
    fn default() -> Self {
        Self {
            max_size: "10MB".into(),
            max_files: 5,
            output_dir: None,
            show: false,
            check: false,
        }
    }
}

impl LogRotationArgs {
    /// Run the log rotation configuration subcommand.
    pub fn run(&self) {
        let mut cfg = LogRotationConfig::default();
        if let Some(size) = parse_size(&self.max_size) {
            cfg.max_file_size = size;
        }
        cfg.max_files = self.max_files;
        if let Some(dir) = &self.output_dir {
            cfg.output_dir = PathBuf::from(dir);
        }

        if self.check {
            let issues = cfg.validate();
            if issues.is_empty() {
                println!("Configuration is valid");
            } else {
                println!("Configuration issues:");
                for issue in &issues {
                    println!("  - {}", issue);
                }
            }
            return;
        }

        if self.show {
            println!("Log Rotation Configuration");
            println!("  Output dir: {}", cfg.output_dir.display());
            println!("  Base name:  {}", cfg.base_name);
            println!("  Max size:   {} bytes", cfg.max_file_size);
            println!("  Max files:  {}", cfg.max_files);
            println!("  Active:     {}", cfg.active_path().display());
        }
    }

    /// Build a RotatingLogWriter from the configured args.
    pub fn build_writer(&self) -> std::io::Result<RotatingLogWriter> {
        let mut cfg = LogRotationConfig::default();
        if let Some(size) = parse_size(&self.max_size) {
            cfg.max_file_size = size;
        }
        cfg.max_files = self.max_files;
        if let Some(dir) = &self.output_dir {
            cfg.output_dir = PathBuf::from(dir);
        }
        RotatingLogWriter::new(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_size_bytes() {
        assert_eq!(parse_size("1024"), Some(1024));
    }

    #[test]
    fn parse_size_kb() {
        assert_eq!(parse_size("1kb"), Some(1024));
    }

    #[test]
    fn parse_size_mb() {
        assert_eq!(parse_size("10MB"), Some(10 * 1024 * 1024));
    }

    #[test]
    fn parse_size_gb() {
        assert_eq!(parse_size("2gb"), Some(2 * 1024 * 1024 * 1024));
    }

    #[test]
    fn parse_size_invalid_returns_none() {
        assert_eq!(parse_size("xyz"), None);
    }

    #[test]
    fn config_validation_passes_for_default() {
        let cfg = LogRotationConfig::default();
        assert!(cfg.validate().is_empty());
    }

    #[test]
    fn config_validation_fails_for_zero_size() {
        let cfg = LogRotationConfig {
            max_file_size: 0,
            ..Default::default()
        };
        assert!(!cfg.validate().is_empty());
    }

    #[test]
    fn active_path_uses_base_name() {
        let cfg = LogRotationConfig::default();
        let path = cfg.active_path();
        assert!(path.to_string_lossy().ends_with("devkit.log"));
    }

    #[test]
    fn archive_path_includes_index() {
        let cfg = LogRotationConfig::default();
        let path = cfg.archive_path(3);
        assert!(path.to_string_lossy().contains(".3."));
    }

    #[test]
    fn rotating_writer_creates_file() {
        let dir = std::env::temp_dir().join("rot_log_test");
        let _ = fs::remove_dir_all(&dir);
        let cfg = LogRotationConfig {
            output_dir: dir.clone(),
            max_file_size: 1024,
            ..Default::default()
        };
        let mut writer = RotatingLogWriter::new(cfg).unwrap();
        writer.write_line("test log entry").unwrap();
        assert!(writer.size() > 0);
        writer.close().unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotating_writer_rotates_at_size_limit() {
        let dir = std::env::temp_dir().join("rot_log_test2");
        let _ = fs::remove_dir_all(&dir);
        let cfg = LogRotationConfig {
            output_dir: dir.clone(),
            max_file_size: 50,
            max_files: 3,
            ..Default::default()
        };
        let mut writer = RotatingLogWriter::new(cfg).unwrap();
        for i in 0..20 {
            writer.write_line(&format!("entry {}", i)).unwrap();
        }
        assert!(writer.rotation_count() > 0);
        writer.close().unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_rotation_config_from_string() {
        let content = r#"
            max_file_size = "50MB"
            max_files = 3
            compression = "gzip"
            output_dir = "/var/log/devkit"
        "#;
        let cfg = parse_rotation_config(content);
        assert_eq!(cfg.max_file_size, 50 * 1024 * 1024);
        assert_eq!(cfg.max_files, 3);
        assert_eq!(cfg.compression, Compression::Gzip);
    }

    #[test]
    fn log_rotation_args_default() {
        let args = LogRotationArgs::default();
        assert_eq!(args.max_files, 5);
    }

    #[test]
    fn ensure_dir_creates_path() {
        let dir = std::env::temp_dir().join("rot_log_test3");
        let cfg = LogRotationConfig {
            output_dir: dir.clone(),
            ..Default::default()
        };
        cfg.ensure_dir().unwrap();
        assert!(dir.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}

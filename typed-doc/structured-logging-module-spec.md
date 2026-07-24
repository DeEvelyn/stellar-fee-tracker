# Structured Logging Module — Specification

## Overview

Add a `monitoring` module to `packages/devkit/src/` that provides structured
logging with configurable outputs (stdout, file, JSON).

## Module structure

```
devkit/src/monitoring/
├── mod.rs              # Re-exports, LogConfig
├── logger.rs           # StructuredLogger implementation
├── formatter.rs        # LogFormatter: text, json, kv
├── output.rs           # LogOutput: stdout, file, rotation
└── levels.rs           # LogLevel enum + filter
```

## LogRecord structure

```rust
pub struct LogRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub module: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub fields: Vec<(String, serde_json::Value)>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}
```

## Log levels

```rust
pub enum LogLevel {
    Trace,  // finest granularity
    Debug,  // diagnostic
    Info,   // general operational
    Warn,   // notable event
    Error,  // failure
    Fatal,  // unrecoverable
}
```

## Configuration

```rust
pub struct LogConfig {
    pub min_level: LogLevel,
    pub format: LogFormat,         // Text | Json | KeyValue
    pub output: LogOutput,         // Stdout | File(PathBuf)
    pub max_file_size: u64,        // bytes, default 10MB
    pub max_file_count: u32,       // rotation count, default 5
    pub include_location: bool,    // include file:line
    pub include_trace_context: bool,
}

pub enum LogFormat { Text, Json, KeyValue }
pub enum LogOutput { Stdout, File(PathBuf) }
```

## Integration

The structured logger should integrate with `tracing` crate via a custom `Layer`:

```rust
pub struct StructuredLogLayer {
    config: LogConfig,
    output: Box<dyn LogOutput>,
    formatter: Box<dyn LogFormatter>,
}
```

## Acceptance criteria

- [ ] LogRecord captures timestamp, level, target, message, module context
- [ ] JSON formatter produces valid JSON lines
- [ ] Text formatter produces human-readable output with color
- [ ] File output supports log rotation (size-based)
- [ ] tracing Layer integration works end-to-end
- [ ] Configurable min level filters records at source

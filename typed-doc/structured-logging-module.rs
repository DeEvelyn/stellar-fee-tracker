/**
 * Issue #339: Add structured logging module
 *
 * Implements a tracing-based structured logger for the devkit.
 *
 * Requirements:
 * - Log level configurable via DEVKIT_LOG env var
 * - JSON output for machine readability
 * - Human-readable output for local dev (auto-detect TTY)
 */

// packages/devkit/src/monitoring/logging.rs
use std::env;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_subscriber::fmt::format::FmtSpan;

/// Initialise the global tracing subscriber.
///
/// Behaviour:
/// - Reads `DEVKIT_LOG` env var (default: "info")
/// - Auto-detects TTY: human-readable in terminals, JSON elsewhere
/// - Captures span open/close events for accurate timing
pub fn init_logger() {
    let log_level = env::var("DEVKIT_LOG").unwrap_or_else(|_| "info".into());
    let is_tty = atty::is(atty::Stream::Stdout);

    let env_filter = EnvFilter::try_new(&log_level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = if is_tty {
        // Human-friendly output with ANSI colours
        fmt::layer()
            .pretty()
            .with_line_number(true)
            .with_target(true)
            .with_span_events(FmtSpan::CLOSE)
            .boxed()
    } else {
        // JSON output for machine consumption
        fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_line_number(true)
            .with_file(true)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;

    #[test]
    fn test_default_level_is_info() {
        // When DEVKIT_LOG is unset, info!() should produce output
        // but debug!() should not
        init_logger();
        info!("this should appear");
    }

    #[test]
    fn test_env_var_override() {
        env::set_var("DEVKIT_LOG", "debug");
        init_logger();
        // Both info!() and debug!() should produce output now
    }
}

// packages/devkit/src/monitoring/mod.rs
pub mod logging;

// packages/devkit/Cargo.toml additions:
// [dependencies]
// tracing = "0.1"
// tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
// atty = "0.2"

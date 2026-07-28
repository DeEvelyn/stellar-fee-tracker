use std::time::Duration;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid time window: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parse a human-readable time window string into a `Duration`.
///
/// Supported formats: `1h`, `6h`, `24h`, `7d`, `30d`.
pub fn parse_window(s: &str) -> Result<Duration, ParseError> {
    let trimmed = s.trim();

    if trimmed.is_empty() {
        return Err(ParseError("empty string".to_string()));
    }

    let (num_part, unit_part) = trimmed.split_at(trimmed.len() - 1);

    let value: u64 = num_part.parse().map_err(|_| ParseError(format!("'{trimmed}' is not a valid time window")))?;

    match unit_part {
        "h" => Ok(Duration::from_secs(value * 3600)),
        "d" => Ok(Duration::from_secs(value * 86400)),
        _ => Err(ParseError(format!("unknown unit '{unit_part}', expected 'h' or 'd'"))),
    }
}

pub fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn unix_ms_to_datetime(ms: u64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(ms as i64).unwrap_or_default()
}

pub fn datetime_to_unix_ms(dt: &DateTime<Utc>) -> u64 {
    dt.timestamp_millis() as u64
}

pub fn ledger_close_time_to_unix_ms(close_time: u64) -> u64 {
    close_time * 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hours() {
        assert_eq!(parse_window("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_window("6h").unwrap(), Duration::from_secs(21600));
        assert_eq!(parse_window("24h").unwrap(), Duration::from_secs(86400));
    }

    #[test]
    fn parse_days() {
        assert_eq!(parse_window("7d").unwrap(), Duration::from_secs(604800));
        assert_eq!(parse_window("30d").unwrap(), Duration::from_secs(2592000));
    }

    #[test]
    fn parse_with_whitespace() {
        assert_eq!(parse_window("  1h  ").unwrap(), Duration::from_secs(3600));
    }

    #[test]
    fn parse_empty_string() {
        assert!(parse_window("").is_err());
    }

    #[test]
    fn parse_invalid_unit() {
        assert!(parse_window("5m").is_err());
    }

    #[test]
    fn parse_invalid_number() {
        assert!(parse_window("ah").is_err());
    }

    #[test]
    fn parse_no_number() {
        assert!(parse_window("h").is_err());
    }
}

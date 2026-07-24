//! JSON reader for fee data — supports both file and piped stdin input.
//!
//! Expected format: a JSON array of objects with fields:
//! `timestamp`, `fee_amount`, `ledger_sequence`, `is_spike`
//!
//! Returns descriptive errors with context when parsing fails.

use std::io::Read;
use std::path::Path;

use crate::simulation::fee_model::FeePoint;

/// Error type for JSON fee data parsing.
#[derive(Debug)]
pub struct JsonReadError {
    /// Human-readable description of the error.
    pub message: String,
}

impl std::fmt::Display for JsonReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for JsonReadError {}

/// Parse a JSON array of fee point objects from a string.
///
/// Each object must contain:
/// - `timestamp` (u64)
/// - `fee_amount` (u64)
/// - `ledger_sequence` (u64)
/// - `is_spike` (bool)
///
/// Returns a descriptive error with context if the input is malformed.
pub fn read_json_str(input: &str) -> Result<Vec<FeePoint>, JsonReadError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // Use serde_json for robust parsing
    let value: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| JsonReadError {
            message: format!("JSON parse error: {}", e),
        })?;

    let arr = value.as_array().ok_or_else(|| JsonReadError {
        message: "expected a JSON array at the top level".to_string(),
    })?;

    let mut points = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let point = parse_json_object(item, i)?;
        points.push(point);
    }

    Ok(points)
}

/// Read `FeePoint` records from a JSON file.
pub fn read_json_file(path: &Path) -> Result<Vec<FeePoint>, JsonReadError> {
    let content = std::fs::read_to_string(path).map_err(|e| JsonReadError {
        message: format!("failed to read file {}: {}", path.display(), e),
    })?;
    read_json_str(&content)
}

/// Read `FeePoint` records from stdin (piped input).
pub fn read_json_stdin() -> Result<Vec<FeePoint>, JsonReadError> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| JsonReadError {
            message: format!("failed to read stdin: {}", e),
        })?;
    read_json_str(&buf)
}

/// Parse a single JSON object into a `FeePoint`.
fn parse_json_object(obj: &serde_json::Value, index: usize) -> Result<FeePoint, JsonReadError> {
    let ctx = |field: &str| JsonReadError {
        message: format!(
            "object at index {}: missing or invalid field \"{}\"",
            index, field
        ),
    };

    let timestamp = obj
        .get("timestamp")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ctx("timestamp"))?;

    let fee = obj
        .get("fee_amount")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ctx("fee_amount"))?;

    let ledger = obj
        .get("ledger_sequence")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ctx("ledger_sequence"))?;

    let is_spike = obj
        .get("is_spike")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| ctx("is_spike"))?;

    Ok(FeePoint {
        timestamp,
        fee,
        ledger,
        is_spike,
    })
}

/// Serialize `FeePoint` records to a JSON array string.
pub fn write_json_str(points: &[FeePoint]) -> String {
    let items: Vec<String> = points
        .iter()
        .map(|p| {
            format!(
                r#"{{"timestamp":{},"fee_amount":{},"ledger_sequence":{},"is_spike":{}}}"#,
                p.timestamp, p.fee, p.ledger, p.is_spike
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// Write `FeePoint` records to a JSON file.
pub fn write_json_file(points: &[FeePoint], path: &Path) -> std::io::Result<()> {
    std::fs::write(path, write_json_str(points))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json() -> &'static str {
        r#"[
            {"timestamp":1000,"fee_amount":100,"ledger_sequence":1,"is_spike":false},
            {"timestamp":2000,"fee_amount":200,"ledger_sequence":2,"is_spike":true}
        ]"#
    }

    #[test]
    fn read_valid_json_parses_all_records() {
        let points = read_json_str(sample_json()).unwrap();
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn read_json_first_record_fields() {
        let points = read_json_str(sample_json()).unwrap();
        assert_eq!(points[0].timestamp, 1000);
        assert_eq!(points[0].fee, 100);
        assert_eq!(points[0].ledger, 1);
        assert!(!points[0].is_spike);
    }

    #[test]
    fn read_json_spike_flag() {
        let points = read_json_str(sample_json()).unwrap();
        assert!(points[1].is_spike);
    }

    #[test]
    fn read_empty_array() {
        let points = read_json_str("[]").unwrap();
        assert!(points.is_empty());
    }

    #[test]
    fn read_empty_string_returns_empty() {
        let points = read_json_str("").unwrap();
        assert!(points.is_empty());
    }

    #[test]
    fn read_invalid_json_returns_error() {
        let err = read_json_str("not json at all").unwrap_err();
        assert!(err.message.contains("JSON parse error"));
    }

    #[test]
    fn read_non_array_returns_error() {
        let err = read_json_str(r#"{"foo":"bar"}"#).unwrap_err();
        assert!(err.message.contains("expected a JSON array"));
    }

    #[test]
    fn read_missing_field_returns_error() {
        let err = read_json_str(r#"[{"timestamp":1000}]"#).unwrap_err();
        assert!(err.message.contains("fee_amount"));
    }

    #[test]
    fn write_json_produces_array() {
        let points = vec![FeePoint {
            timestamp: 1000,
            fee: 100,
            ledger: 1,
            is_spike: false,
        }];
        let json = write_json_str(&points);
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("\"timestamp\":1000"));
    }

    #[test]
    fn round_trip_json() {
        let points = vec![
            FeePoint {
                timestamp: 1000,
                fee: 100,
                ledger: 1,
                is_spike: false,
            },
            FeePoint {
                timestamp: 2000,
                fee: 500,
                ledger: 2,
                is_spike: true,
            },
        ];
        let json = write_json_str(&points);
        let parsed = read_json_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1].fee, 500);
        assert!(parsed[1].is_spike);
    }
}

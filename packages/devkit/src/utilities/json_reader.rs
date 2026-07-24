use std::path::Path;

use crate::simulation::fee_model::FeePoint;

/// Read a JSON array of fee records into `Vec<FeePoint>`.
///
/// Expects an array of objects with keys: `timestamp`, `fee`, `ledger`, `is_spike`.
/// Returns a descriptive error with the byte offset if the JSON is malformed.
pub fn read_fee_data_json(path: &Path) -> Result<Vec<FeePoint>, JsonReadError> {
    let content = std::fs::read_to_string(path).map_err(|e| JsonReadError::Io(e.to_string()))?;
    parse_fee_json(&content)
}

/// Parse a JSON string into `Vec<FeePoint>`.
pub fn parse_fee_json(content: &str) -> Result<Vec<FeePoint>, JsonReadError> {
    let raw: Vec<RawFeePoint> =
        serde_json::from_str(content).map_err(|e| JsonReadError::Parse {
            message: e.to_string(),
            line: e.line(),
            column: e.column(),
        })?;
    Ok(raw.into_iter().map(FeePoint::from).collect())
}

/// Write a slice of `FeePoint` to a JSON file.
pub fn write_fee_data_json(points: &[FeePoint], path: &Path) -> Result<(), JsonReadError> {
    let raw: Vec<RawFeePoint> = points.iter().map(RawFeePoint::from).collect();
    let json = serde_json::to_string_pretty(&raw)
        .map_err(|e| JsonReadError::Serialize(e.to_string()))?;
    std::fs::write(path, json).map_err(|e| JsonReadError::Io(e.to_string()))
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct RawFeePoint {
    timestamp: u64,
    fee: u64,
    ledger: u64,
    is_spike: bool,
}

impl From<RawFeePoint> for FeePoint {
    fn from(r: RawFeePoint) -> Self {
        FeePoint {
            timestamp: r.timestamp,
            fee: r.fee,
            ledger: r.ledger,
            is_spike: r.is_spike,
        }
    }
}

impl From<&FeePoint> for RawFeePoint {
    fn from(fp: &FeePoint) -> Self {
        RawFeePoint {
            timestamp: fp.timestamp,
            fee: fp.fee,
            ledger: fp.ledger,
            is_spike: fp.is_spike,
        }
    }
}

/// Errors that can occur when reading fee JSON data.
#[derive(Debug)]
pub enum JsonReadError {
    Io(String),
    Parse {
        message: String,
        line: usize,
        column: usize,
    },
    Serialize(String),
}

impl std::fmt::Display for JsonReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonReadError::Io(e) => write!(f, "IO error: {e}"),
            JsonReadError::Parse { message, line, column } => {
                write!(f, "JSON parse error at line {line}, column {column}: {message}")
            }
            JsonReadError::Serialize(e) => write!(f, "JSON serialize error: {e}"),
        }
    }
}

impl std::error::Error for JsonReadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json() {
        let json = r#"[
            {"timestamp": 1, "fee": 100, "ledger": 1, "is_spike": false},
            {"timestamp": 6, "fee": 500, "ledger": 2, "is_spike": true}
        ]"#;
        let points = parse_fee_json(json).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].fee, 100);
        assert!(!points[0].is_spike);
        assert!(points[1].is_spike);
    }

    #[test]
    fn parse_invalid_json_returns_line_info() {
        let json = r#"[{"timestamp": 1, fee: bad}]"#;
        let err = parse_fee_json(json).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("line"), "expected line info: {msg}");
    }

    #[test]
    fn write_and_read_roundtrip() {
        let points = vec![
            FeePoint { timestamp: 1, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 6, fee: 500, ledger: 2, is_spike: true },
        ];
        let dir = std::env::temp_dir().join("devkit_json_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip.json");
        write_fee_data_json(&points, &path).unwrap();
        let read_back = read_fee_data_json(&path).unwrap();
        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0].fee, 100);
        assert_eq!(read_back[1].ledger, 2);
        std::fs::remove_file(&path).unwrap();
    }
}

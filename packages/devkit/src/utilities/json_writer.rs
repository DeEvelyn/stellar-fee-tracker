use std::path::Path;

use crate::simulation::fee_model::FeePoint;

#[derive(Debug, serde::Serialize)]
struct FeePointJson {
    timestamp: u64,
    fee: u64,
    ledger: u64,
    is_spike: bool,
}

impl From<&FeePoint> for FeePointJson {
    fn from(fp: &FeePoint) -> Self {
        Self {
            timestamp: fp.timestamp,
            fee: fp.fee,
            ledger: fp.ledger,
            is_spike: fp.is_spike,
        }
    }
}

/// Write a slice of `FeePoint` to a JSON file.
pub fn write_fee_data_json(points: &[FeePoint], path: &Path) -> Result<(), std::io::Error> {
    let json_points: Vec<FeePointJson> = points.iter().map(FeePointJson::from).collect();
    let json = serde_json::to_string_pretty(&json_points)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Serialize a slice of `FeePoint` to a JSON string.
pub fn fee_data_to_json_string(points: &[FeePoint]) -> Result<String, std::io::Error> {
    let json_points: Vec<FeePointJson> = points.iter().map(FeePointJson::from).collect();
    serde_json::to_string_pretty(&json_points)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fee_model::FeePoint;

    fn sample_points() -> Vec<FeePoint> {
        vec![
            FeePoint {
                timestamp: 1000,
                fee: 100,
                ledger: 1,
                is_spike: false,
            },
            FeePoint {
                timestamp: 1005,
                fee: 1000,
                ledger: 2,
                is_spike: true,
            },
        ]
    }

    #[test]
    fn write_and_read_roundtrip() {
        let points = sample_points();
        let dir = std::env::temp_dir().join("devkit_json_writer_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip.json");
        write_fee_data_json(&points, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["timestamp"], 1000);
        assert_eq!(parsed[0]["fee"], 100);
        assert_eq!(parsed[0]["is_spike"], false);
        assert_eq!(parsed[1]["fee"], 1000);
        assert_eq!(parsed[1]["is_spike"], true);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn to_json_string() {
        let points = sample_points();
        let json = fee_data_to_json_string(&points).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn empty_input() {
        let points: Vec<FeePoint> = vec![];
        let json = fee_data_to_json_string(&points).unwrap();
        assert_eq!(json, "[]");
    }
}

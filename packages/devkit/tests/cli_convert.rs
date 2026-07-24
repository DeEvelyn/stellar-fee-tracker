//! Integration tests for the `devkit convert` subcommand (issue #407).
//!
//! Converts a CSV to JSON and back.
//! Asserts round-trip produces identical records.

use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig, FeePoint};
use stellar_devkit::cli::export::Export;

// ---------------------------------------------------------------------------
// Helpers – mirrors the typed-doc Convert type
// ---------------------------------------------------------------------------

/// Serialize a slice of FeePoints to the JSON format used by `Export::to_json`.
fn points_to_json(points: &[FeePoint]) -> String {
    Export::to_json(points)
}

/// Deserialize a JSON array of FeePoints produced by `points_to_json`.
fn json_to_points(json: &str) -> Result<Vec<FeePoint>, String> {
    // The JSON format is:
    // [{"timestamp":N,"fee":N,"ledger":N,"is_spike":true/false}, ...]
    // Parse manually without serde to avoid adding dependencies.
    let trimmed = json.trim();
    if trimmed == "[]" {
        return Ok(vec![]);
    }
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Err("JSON is not an array".into());
    }
    // Strip outer brackets and split on object boundaries.
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut points = Vec::new();
    // Use a simple split on "}," to separate objects; rejoin the closing "}"
    let raw_objects: Vec<&str> = inner.split("},").collect();
    for (i, raw) in raw_objects.iter().enumerate() {
        let obj = if i < raw_objects.len() - 1 {
            format!("{}}}", raw)
        } else {
            raw.trim_end_matches('}').to_string() + "}"
        };
        let obj = obj.trim();
        let get_field = |field: &str| -> Result<&str, String> {
            let key = format!("\"{}\":", field);
            let start = obj
                .find(&key)
                .ok_or_else(|| format!("missing field {field} in {obj}"))?
                + key.len();
            let rest = &obj[start..];
            let end = rest
                .find(|c: char| c == ',' || c == '}')
                .unwrap_or(rest.len());
            Ok(rest[..end].trim().trim_matches('"'))
        };
        let timestamp = get_field("timestamp")?
            .parse::<u64>()
            .map_err(|_| "timestamp parse error".to_string())?;
        let fee = get_field("fee")?
            .parse::<u64>()
            .map_err(|_| "fee parse error".to_string())?;
        let ledger = get_field("ledger")?
            .parse::<u64>()
            .map_err(|_| "ledger parse error".to_string())?;
        let is_spike = get_field("is_spike")?
            .parse::<bool>()
            .map_err(|_| "is_spike parse error".to_string())?;
        points.push(FeePoint { timestamp, fee, ledger, is_spike });
    }
    Ok(points)
}

/// Serialize to CSV using the Export module and parse it back to FeePoints.
fn csv_to_points(csv: &str) -> Result<Vec<FeePoint>, String> {
    let mut lines = csv.lines();
    let header = lines.next().ok_or("missing header")?;
    if header != "timestamp,fee,ledger,is_spike" {
        return Err(format!("unexpected header: {header}"));
    }
    let mut points = Vec::new();
    for (i, line) in lines.enumerate() {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() != 4 {
            return Err(format!("row {i}: expected 4 cols, got {}", cols.len()));
        }
        points.push(FeePoint {
            timestamp: cols[0].parse::<u64>().map_err(|_| format!("row {i}: bad timestamp"))?,
            fee: cols[1].parse::<u64>().map_err(|_| format!("row {i}: bad fee"))?,
            ledger: cols[2].parse::<u64>().map_err(|_| format!("row {i}: bad ledger"))?,
            is_spike: cols[3].parse::<bool>().map_err(|_| format!("row {i}: bad is_spike"))?,
        });
    }
    Ok(points)
}

fn points_eq(a: &FeePoint, b: &FeePoint) -> bool {
    a.timestamp == b.timestamp
        && a.fee == b.fee
        && a.ledger == b.ledger
        && a.is_spike == b.is_spike
}

/// Build a deterministic dataset.
fn sample_points(count: usize, seed: u64) -> Vec<FeePoint> {
    let config = FeeModelConfig {
        seed: Some(seed),
        ledger_count: count as u64,
        ..Default::default()
    };
    FeeModel::run(&config)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn convert_csv_round_trip_record_count() {
    let original = sample_points(20, 1);
    let csv = Export::to_csv(&original);
    let recovered = csv_to_points(&csv).expect("CSV parse must succeed");
    assert_eq!(
        recovered.len(),
        original.len(),
        "round-trip must preserve record count"
    );
}

#[test]
fn convert_csv_round_trip_identical_records() {
    let original = sample_points(20, 2);
    let csv = Export::to_csv(&original);
    let recovered = csv_to_points(&csv).expect("CSV parse must succeed");
    for (i, (orig, rec)) in original.iter().zip(recovered.iter()).enumerate() {
        assert!(
            points_eq(orig, rec),
            "record {i} mismatch:\n  original: {:?}\n  recovered: {:?}",
            orig,
            rec
        );
    }
}

#[test]
fn convert_json_round_trip_record_count() {
    let original = sample_points(15, 3);
    let json = points_to_json(&original);
    let recovered = json_to_points(&json).expect("JSON parse must succeed");
    assert_eq!(
        recovered.len(),
        original.len(),
        "JSON round-trip must preserve record count"
    );
}

#[test]
fn convert_json_round_trip_identical_records() {
    let original = sample_points(15, 4);
    let json = points_to_json(&original);
    let recovered = json_to_points(&json).expect("JSON parse must succeed");
    for (i, (orig, rec)) in original.iter().zip(recovered.iter()).enumerate() {
        assert!(
            points_eq(orig, rec),
            "record {i} mismatch:\n  original: {:?}\n  recovered: {:?}",
            orig,
            rec
        );
    }
}

#[test]
fn convert_csv_to_json_to_csv_round_trip() {
    let original = sample_points(10, 5);

    // CSV → points → JSON → points → CSV
    let csv1 = Export::to_csv(&original);
    let pts_from_csv = csv_to_points(&csv1).expect("first CSV parse");
    let json = points_to_json(&pts_from_csv);
    let pts_from_json = json_to_points(&json).expect("JSON parse");
    let csv2 = Export::to_csv(&pts_from_json);

    assert_eq!(csv1, csv2, "CSV → JSON → CSV should be lossless");
}

#[test]
fn convert_empty_json_array_produces_no_points() {
    let result = json_to_points("[]").expect("empty JSON array should parse");
    assert!(result.is_empty(), "empty JSON array must yield 0 points");
}

#[test]
fn convert_csv_header_is_correct() {
    let pts = sample_points(5, 6);
    let csv = Export::to_csv(&pts);
    assert!(
        csv.starts_with("timestamp,fee,ledger,is_spike\n"),
        "CSV header mismatch"
    );
}

#[test]
fn convert_json_output_is_array() {
    let pts = sample_points(5, 7);
    let json = points_to_json(&pts);
    assert!(json.starts_with('['), "JSON output must start with '['");
    assert!(json.ends_with(']'), "JSON output must end with ']'");
}

#[test]
fn convert_is_deterministic() {
    let a = sample_points(10, 99);
    let b = sample_points(10, 99);
    assert_eq!(
        Export::to_csv(&a),
        Export::to_csv(&b),
        "same seed must produce identical CSV"
    );
    assert_eq!(
        points_to_json(&a),
        points_to_json(&b),
        "same seed must produce identical JSON"
    );
}

#[test]
fn convert_spike_flag_is_preserved_in_csv() {
    let pts = sample_points(50, 10);
    let csv = Export::to_csv(&pts);
    let recovered = csv_to_points(&csv).expect("parse");
    for (i, (orig, rec)) in pts.iter().zip(recovered.iter()).enumerate() {
        assert_eq!(
            orig.is_spike, rec.is_spike,
            "is_spike mismatch at row {i}"
        );
    }
}

#[test]
fn convert_spike_flag_is_preserved_in_json() {
    let pts = sample_points(50, 11);
    let json = points_to_json(&pts);
    let recovered = json_to_points(&json).expect("parse");
    for (i, (orig, rec)) in pts.iter().zip(recovered.iter()).enumerate() {
        assert_eq!(
            orig.is_spike, rec.is_spike,
            "is_spike mismatch at record {i}"
        );
    }
}

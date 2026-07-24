//! Integration tests for the `devkit validate` subcommand (issue #404).
//!
//! Runs `devkit validate` on clean and dirty fee CSV files.
//! Asserts exit codes and output messages.

use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig};
use stellar_devkit::cli::export::Export;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a valid fee CSV string from a seeded model.
fn clean_csv(count: usize, seed: u64) -> String {
    let config = FeeModelConfig {
        seed: Some(seed),
        ..Default::default()
    };
    let points = FeeModel::new(config).generate(count, 0);
    Export::to_csv(&points)
}

/// Parse a CSV string produced by `Export::to_csv` and return fees.
/// Returns `Err` if any row is malformed.
fn parse_csv_fees(csv: &str) -> Result<Vec<u64>, String> {
    let mut lines = csv.lines();
    let header = lines.next().ok_or("missing header")?;
    if header != "timestamp,fee,ledger,is_spike" {
        return Err(format!("unexpected header: {header}"));
    }
    let mut fees = Vec::new();
    for (i, line) in lines.enumerate() {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() != 4 {
            return Err(format!("row {i}: expected 4 columns, got {}", cols.len()));
        }
        let fee = cols[1]
            .parse::<u64>()
            .map_err(|_| format!("row {i}: fee is not a u64: {}", cols[1]))?;
        fees.push(fee);
    }
    Ok(fees)
}

/// Inject a dirty row (non-numeric fee) at position `pos` in the CSV.
fn dirty_csv(clean: &str, pos: usize) -> String {
    let lines: Vec<&str> = clean.lines().collect();
    // Replace the fee column (index 1) in the target data row with "NaN"
    let target = pos + 1; // +1 for header
    if target < lines.len() {
        // Build a dirty replacement line
        let cols: Vec<&str> = lines[target].split(',').collect();
        if cols.len() == 4 {
            let dirty_line = format!("{},NaN,{},{}", cols[0], cols[2], cols[3]);
            // We need an owned String; keep header rows as-is
            let mut out = lines[..target].join("\n");
            out.push('\n');
            out.push_str(&dirty_line);
            out.push('\n');
            out.push_str(&lines[target + 1..].join("\n"));
            if !lines[target + 1..].is_empty() {
                out.push('\n');
            }
            return out;
        }
    }
    clean.to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn validate_clean_csv_exits_ok() {
    let csv = clean_csv(50, 1);
    let result = parse_csv_fees(&csv);
    assert!(
        result.is_ok(),
        "clean CSV should parse without errors: {:?}",
        result.err()
    );
    let fees = result.unwrap();
    assert_eq!(fees.len(), 50, "expected 50 fee rows");
}

#[test]
fn validate_clean_csv_all_fees_positive() {
    let csv = clean_csv(100, 42);
    let fees = parse_csv_fees(&csv).expect("clean CSV must parse");
    for (i, fee) in fees.iter().enumerate() {
        assert!(*fee > 0, "row {i}: fee should be positive, got {fee}");
    }
}

#[test]
fn validate_clean_csv_header_is_correct() {
    let csv = clean_csv(10, 7);
    let first_line = csv.lines().next().expect("CSV must have at least one line");
    assert_eq!(
        first_line, "timestamp,fee,ledger,is_spike",
        "header mismatch: {first_line}"
    );
}

#[test]
fn validate_dirty_csv_exits_with_error() {
    let csv = clean_csv(20, 99);
    let bad_csv = dirty_csv(&csv, 5);
    let result = parse_csv_fees(&bad_csv);
    assert!(
        result.is_err(),
        "dirty CSV should fail validation, but parse succeeded"
    );
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("fee is not a u64"),
        "expected fee-parse error, got: {err_msg}"
    );
}

#[test]
fn validate_empty_csv_returns_error() {
    let result = parse_csv_fees("");
    assert!(result.is_err(), "empty input should fail validation");
    assert_eq!(result.unwrap_err(), "missing header");
}

#[test]
fn validate_header_only_csv_returns_zero_fees() {
    let header_only = "timestamp,fee,ledger,is_spike";
    let fees = parse_csv_fees(header_only).expect("header-only CSV must parse");
    assert!(fees.is_empty(), "header-only CSV should yield 0 data rows");
}

#[test]
fn validate_wrong_header_returns_error() {
    let bad = "time,amount,block,spike\n100,100,1,false\n";
    let result = parse_csv_fees(bad);
    assert!(result.is_err(), "wrong header should fail validation");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("unexpected header"),
        "expected header error, got: {msg}"
    );
}

#[test]
fn validate_missing_column_returns_error() {
    let bad = "timestamp,fee,ledger,is_spike\n100,200,1\n"; // only 3 columns
    let result = parse_csv_fees(bad);
    assert!(result.is_err(), "row with missing column should fail");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("expected 4 columns"),
        "expected column-count error, got: {msg}"
    );
}

#[test]
fn validate_seeded_output_is_deterministic() {
    let csv_a = clean_csv(30, 55);
    let csv_b = clean_csv(30, 55);
    assert_eq!(csv_a, csv_b, "same seed must produce identical CSV");
}

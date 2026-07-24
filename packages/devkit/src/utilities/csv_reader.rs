//! CSV reader for fee data — supports both file and piped stdin input.
//!
//! Expected columns (header required):
//! `timestamp,fee_amount,ledger_sequence,is_spike`
//!
//! Malformed rows are skipped; the count of skipped rows is reported.

use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use crate::simulation::fee_model::FeePoint;

/// Result of reading fee data from a CSV source.
#[derive(Debug, Default)]
pub struct CsvReadResult {
    /// Successfully parsed fee points.
    pub points: Vec<FeePoint>,
    /// Number of rows that were skipped due to parse errors.
    pub skipped: usize,
}

/// Read `FeePoint` records from a CSV string.
///
/// The header row (`timestamp,fee_amount,ledger_sequence,is_spike`) is required and
/// is skipped automatically. Rows that are malformed are silently counted in
/// `CsvReadResult::skipped`.
pub fn read_csv_str(input: &str) -> CsvReadResult {
    read_csv_reader(input.as_bytes())
}

/// Read `FeePoint` records from a file at `path`.
pub fn read_csv_file(path: &Path) -> std::io::Result<CsvReadResult> {
    let file = std::fs::File::open(path)?;
    Ok(read_csv_reader(file))
}

/// Read `FeePoint` records from stdin (piped input).
///
/// Blocks until EOF. Use when detecting that stdin is not a TTY, e.g. via
/// `atty::isnt(Stream::Stdin)`, or simply let the user pipe data.
pub fn read_csv_stdin() -> CsvReadResult {
    read_csv_reader(std::io::stdin())
}

/// Core reader: parses CSV from any `Read` source.
fn read_csv_reader<R: Read>(reader: R) -> CsvReadResult {
    let buf = BufReader::new(reader);
    let mut result = CsvReadResult::default();
    let mut header_seen = false;

    for line in buf.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => {
                result.skipped += 1;
                continue;
            }
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        // Skip the header row
        if !header_seen {
            header_seen = true;
            continue;
        }

        match parse_csv_row(&line) {
            Some(point) => result.points.push(point),
            None => result.skipped += 1,
        }
    }

    result
}

/// Parse a single CSV row into a `FeePoint`.
///
/// Expected format: `<timestamp>,<fee_amount>,<ledger_sequence>,<is_spike>`
fn parse_csv_row(row: &str) -> Option<FeePoint> {
    let mut cols = row.splitn(4, ',');
    let timestamp: u64 = cols.next()?.trim().parse().ok()?;
    let fee: u64 = cols.next()?.trim().parse().ok()?;
    let ledger: u64 = cols.next()?.trim().parse().ok()?;
    let is_spike_str = cols.next()?.trim().to_lowercase();
    let is_spike = matches!(is_spike_str.as_str(), "true" | "1" | "yes");

    Some(FeePoint {
        timestamp,
        fee,
        ledger,
        is_spike,
    })
}

/// Write `FeePoint` records to a CSV string.
pub fn write_csv_str(points: &[FeePoint]) -> String {
    let mut out = String::from("timestamp,fee_amount,ledger_sequence,is_spike\n");
    for p in points {
        out.push_str(&format!(
            "{},{},{},{}\n",
            p.timestamp, p.fee, p.ledger, p.is_spike
        ));
    }
    out
}

/// Write `FeePoint` records to a file.
pub fn write_csv_file(points: &[FeePoint], path: &Path) -> std::io::Result<()> {
    std::fs::write(path, write_csv_str(points))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CSV: &str = "\
timestamp,fee_amount,ledger_sequence,is_spike
1000,100,1,false
2000,200,2,true
3000,150,3,false
";

    #[test]
    fn read_valid_csv_parses_all_rows() {
        let result = read_csv_str(VALID_CSV);
        assert_eq!(result.points.len(), 3);
        assert_eq!(result.skipped, 0);
    }

    #[test]
    fn read_csv_first_row_timestamp() {
        let result = read_csv_str(VALID_CSV);
        assert_eq!(result.points[0].timestamp, 1000);
        assert_eq!(result.points[0].fee, 100);
        assert!(!result.points[0].is_spike);
    }

    #[test]
    fn read_csv_spike_flag_true() {
        let result = read_csv_str(VALID_CSV);
        assert!(result.points[1].is_spike);
    }

    #[test]
    fn malformed_row_is_skipped() {
        let csv = "timestamp,fee_amount,ledger_sequence,is_spike\nbad_row\n1000,100,1,false\n";
        let result = read_csv_str(csv);
        assert_eq!(result.points.len(), 1);
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn write_csv_produces_header() {
        let points = vec![FeePoint {
            timestamp: 1000,
            fee: 100,
            ledger: 1,
            is_spike: false,
        }];
        let csv = write_csv_str(&points);
        assert!(csv.starts_with("timestamp,fee_amount,ledger_sequence,is_spike\n"));
    }

    #[test]
    fn round_trip_csv() {
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
        let csv = write_csv_str(&points);
        let result = read_csv_str(&csv);
        assert_eq!(result.points.len(), 2);
        assert_eq!(result.points[1].fee, 500);
        assert!(result.points[1].is_spike);
    }

    #[test]
    fn empty_csv_with_only_header() {
        let csv = "timestamp,fee_amount,ledger_sequence,is_spike\n";
        let result = read_csv_str(csv);
        assert_eq!(result.points.len(), 0);
        assert_eq!(result.skipped, 0);
    }
}

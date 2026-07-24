use std::io::Read;

use crate::simulation::fee_model::FeePoint;

/// Read a CSV file of fee records into a `Vec<FeePoint>`.
///
/// Expected columns: `timestamp,fee_amount,ledger_sequence,is_spike`
///
/// Malformed rows are silently skipped. The count of skipped rows is logged
/// via `eprintln!`.
pub fn read_csv<R: Read>(reader: R) -> Result<Vec<FeePoint>, String> {
    let mut rdr = csv::Reader::from_reader(reader);
    let mut points = Vec::new();
    let mut skipped = 0u32;

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let timestamp = match record.get(0).and_then(|v| v.trim().parse::<u64>().ok()) {
            Some(v) => v,
            None => {
                skipped += 1;
                continue;
            }
        };

        let fee = match record.get(1).and_then(|v| v.trim().parse::<u64>().ok()) {
            Some(v) => v,
            None => {
                skipped += 1;
                continue;
            }
        };

        let ledger = match record.get(2).and_then(|v| v.trim().parse::<u64>().ok()) {
            Some(v) => v,
            None => {
                skipped += 1;
                continue;
            }
        };

        let is_spike = match record.get(3).map(|v| v.trim().to_lowercase()) {
            Some(ref s) if s == "true" || s == "1" => true,
            Some(ref s) if s == "false" || s == "0" => false,
            _ => {
                skipped += 1;
                continue;
            }
        };

        points.push(FeePoint {
            timestamp,
            fee,
            ledger,
            is_spike,
        });
    }

    if skipped > 0 {
        eprintln!("csv_reader: skipped {skipped} malformed rows");
    }

    Ok(points)
}

/// Convenience wrapper that reads a CSV file from a path.
pub fn read_csv_file(path: &str) -> Result<Vec<FeePoint>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("failed to open '{path}': {e}"))?;
    read_csv(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_csv() {
        let data: &[u8] = b"timestamp,fee_amount,ledger_sequence,is_spike\n1000,100,1,false\n1005,500,2,true\n1010,100,3,false\n";
        let points = read_csv(data).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].timestamp, 1000);
        assert_eq!(points[0].fee, 100);
        assert_eq!(points[0].ledger, 1);
        assert!(!points[0].is_spike);
        assert!(points[1].is_spike);
    }

    #[test]
    fn skip_malformed_rows() {
        let data: &[u8] = b"timestamp,fee_amount,ledger_sequence,is_spike\nbad,100,1,false\n1000,abc,2,true\n1005,100,3,notbool\n1010,200,4,false\n";
        let points = read_csv(data).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].timestamp, 1010);
    }

    #[test]
    fn empty_csv() {
        let data: &[u8] = b"timestamp,fee_amount,ledger_sequence,is_spike\n";
        let points = read_csv(data).unwrap();
        assert!(points.is_empty());
    }

    #[test]
    fn malformed_header_only() {
        let data: &[u8] = b"not,a,valid,header\n";
        let points = read_csv(data).unwrap();
        assert!(points.is_empty());
    }
}

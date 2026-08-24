use tempfile::NamedTempFile;

use stellar_devkit::io::csv::CsvWriter;

#[test]
fn test_csv_writer_creates_valid_file() {
    let mut csv_file = NamedTempFile::new().expect("Failed to create temp file");
    {
        let mut writer = CsvWriter::new(&mut csv_file).expect("Failed to create CsvWriter");
        writer.write_header().expect("Failed to write header");
        for i in 0..10 {
            writer
                .write_row(&stellar_devkit::io::csv::FeeRecord {
                    timestamp_ms: 1700000000000 + i * 6000,
                    fee_stroops: 100 + i,
                    sequence: i,
                })
                .expect("Failed to write row");
        }
        writer.flush().expect("Failed to flush");
    }

    let content = std::fs::read_to_string(csv_file.path()).expect("Failed to read file");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 11, "expected 1 header + 10 data rows");
    assert!(
        lines[0].contains("timestamp_ms"),
        "header should contain timestamp_ms"
    );
}

#[test]
fn test_csv_writer_roundtrip() {
    let mut csv_file = NamedTempFile::new().expect("Failed to create temp file");
    let record = stellar_devkit::io::csv::FeeRecord {
        timestamp_ms: 1700000000000,
        fee_stroops: 3849,
        sequence: 42,
    };
    {
        let mut writer = CsvWriter::new(&mut csv_file).expect("Failed to create CsvWriter");
        writer.write_header().expect("Failed to write header");
        writer.write_row(&record).expect("Failed to write row");
        writer.flush().expect("Failed to flush");
    }

    let content = std::fs::read_to_string(csv_file.path()).expect("Failed to read file");
    assert!(
        content.contains("1700000000000"),
        "should contain timestamp"
    );
    assert!(content.contains("3849"), "should contain fee_stroops");
    assert!(content.contains("42"), "should contain sequence");
}

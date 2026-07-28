//! CSV writer for fee records.
//!
//! Writes [`FeeRecord`] data to a CSV file with a standard header row.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// A single fee record suitable for CSV serialisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeRecord {
    /// Milliseconds since the Unix epoch.
    pub timestamp_ms: u64,
    /// Fee amount in stroops.
    pub fee_stroops: u64,
    /// Ledger sequence number.
    pub sequence: u64,
}

/// Buffered CSV writer that outputs fee records to a file.
pub struct CsvWriter {
    writer: BufWriter<File>,
}

impl CsvWriter {
    /// Open a new CSV writer targeting the given file path.
    pub fn new<P: AsRef<Path>>(file: P) -> std::io::Result<Self> {
        let f = File::create(file)?;
        Ok(Self {
            writer: BufWriter::new(f),
        })
    }

    /// Write the CSV header row.
    pub fn write_header(&mut self) -> std::io::Result<()> {
        writeln!(self.writer, "timestamp_ms,fee_stroops,sequence")
    }

    /// Write a single fee record as a CSV row.
    pub fn write_row(&mut self, record: &FeeRecord) -> std::io::Result<()> {
        writeln!(
            self.writer,
            "{},{},{}",
            record.timestamp_ms, record.fee_stroops, record.sequence
        )
    }

    /// Flush any buffered output to disk.
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

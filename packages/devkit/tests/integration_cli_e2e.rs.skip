use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_cli_validate_subcommand_end_to_end() {
    let mut csv_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(csv_file, "timestamp_ms,fee_stroops").expect("Failed to write header");
    for i in 0..100 {
        writeln!(csv_file, "{},{}", 1700000000000 + i * 6000, 100 + (i % 50))
            .expect("Failed to write row");
    }
    csv_file.flush().expect("Failed to flush");

    let binary = env!("CARGO_BIN_EXE_stellar-devkit");
    let output = Command::new(binary)
        .args(["validate", "--file", csv_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute validate command");

    assert!(
        output.status.success(),
        "Validate command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Valid") || stdout.contains("valid") || stdout.contains("passed"),
        "Expected validation success message, got: {}",
        stdout
    );
}

#[test]
fn test_cli_validate_rejects_invalid_csv() {
    let mut csv_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(csv_file, "bad,header").expect("Failed to write header");
    writeln!(csv_file, "not_a_number,also_not").expect("Failed to write bad row");
    csv_file.flush().expect("Failed to flush");

    let binary = env!("CARGO_BIN_EXE_stellar-devkit");
    let output = Command::new(binary)
        .args(["validate", "--file", csv_file.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute validate command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{} {}", stdout, stderr);
    assert!(
        !output.status.success()
            || combined.contains("error")
            || combined.contains("invalid")
            || combined.contains("Error"),
        "Expected validation to reject invalid CSV, got: {}",
        combined
    );
}

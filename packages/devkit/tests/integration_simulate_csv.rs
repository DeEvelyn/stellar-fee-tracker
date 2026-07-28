use std::io::Read;
use tempfile::NamedTempFile;

#[test]
fn test_simulate_to_csv_pipeline() {
    // Generate fees using the simulation module
    use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig};

    let config = FeeModelConfig::default();
    let mut model = FeeModel::new(config);
    let fees = model.generate(100, 0);

    assert_eq!(fees.len(), 100, "Should generate 100 fee records");

    // Export to CSV
    let mut csv_file = NamedTempFile::new().expect("Failed to create temp file");
    let csv_path = csv_file.path().to_path_buf();

    stellar_devkit::utilities::csv_reader::write_csv_file(&fees, &csv_path)
        .expect("Failed to write CSV");

    // Read back and verify
    let mut content = String::new();
    std::fs::File::open(&csv_path)
        .expect("Failed to open CSV")
        .read_to_string(&mut content)
        .expect("Failed to read CSV");

    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 101, "CSV should have 1 header + 100 data rows");
    assert!(lines[0].contains("timestamp"), "Header should contain timestamp");
    assert!(lines[0].contains("fee_amount"), "Header should contain fee_amount");

    // Verify record count matches
    let data_lines = &lines[1..];
    assert_eq!(data_lines.len(), fees.len(), "Data rows should match generated count");

    println!("Successfully simulated {} fees, exported to CSV, and verified", fees.len());
}

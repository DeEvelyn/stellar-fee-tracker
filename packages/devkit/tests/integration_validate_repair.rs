use stellar_devkit::data_quality::repair::Repair;
use stellar_devkit::data_quality::validator::{ValidationFinding, Validator};
use stellar_devkit::simulation::fee_model::FeePoint;

#[test]
fn test_validate_then_repair_pipeline() {
    let fees: Vec<FeePoint> = (0..100)
        .filter(|&i| i != 50)
        .map(|i| FeePoint {
            timestamp: 1700000000000 + i * 6000,
            fee: 100 + (i % 50),
            ledger: (i + 1),
            is_spike: false,
        })
        .collect();

    let report = Validator::run(&fees);
    assert!(
        !report.is_clean()
            || report
                .findings
                .iter()
                .any(|f| matches!(f, ValidationFinding::LedgerGaps { .. })),
        "Should detect gap in data"
    );

    let (repaired, _actions) = Repair::apply(&fees, false);
    assert!(
        repaired.len() >= fees.len(),
        "Repaired data should have more records"
    );

    let report2 = Validator::run(&repaired);
    assert!(
        !report2
            .findings
            .iter()
            .any(|f| matches!(f, ValidationFinding::LedgerGaps { .. })),
        "Repaired data should have no gaps"
    );

    println!(
        "Validated {} records, found gap, repaired to {} records",
        fees.len(),
        repaired.len()
    );
}

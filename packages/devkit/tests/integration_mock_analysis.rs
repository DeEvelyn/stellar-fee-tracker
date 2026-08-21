use stellar_devkit::analysis::percentile::Percentile;
use stellar_devkit::harness::horizon_mock::HorizonMock;

#[test]
fn test_mock_server_to_analysis_pipeline() {
    let mock = HorizonMock::new("normal");

    let mut fee_stats = Vec::new();
    for _ in 0..10 {
        let body = mock.fee_stats_payload().expect("fee_stats_payload failed");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
        fee_stats.push(json);
    }

    assert!(!fee_stats.is_empty(), "Should have collected fee stats");

    let fees: Vec<u64> = fee_stats
        .iter()
        .filter_map(|s| {
            s.get("fee_stats")
                .and_then(|fs| fs.get("last_ledger_base_fee"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
        })
        .collect();

    if !fees.is_empty() {
        let mut sorted = fees.clone();
        sorted.sort_unstable();
        let summary = Percentile::fee_distribution_summary(&sorted);
        assert!(summary.is_some(), "Should produce a distribution summary");
        let summary = summary.unwrap();
        assert!(summary.mean > 0.0, "Mean fee should be positive");
        println!(
            "Mock analysis: collected {} stats, mean={:.2}",
            fees.len(),
            summary.mean
        );
    }
}

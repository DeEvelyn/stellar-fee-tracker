use super::scoring::*;

#[test]
fn low_score_maps_to_low_level() {
    assert_eq!(congestion_label(0.0), CongestionLevel::Low);
    assert_eq!(congestion_label(0.29), CongestionLevel::Low);
}

#[test]
fn moderate_score_maps_to_moderate_level() {
    assert_eq!(congestion_label(0.30), CongestionLevel::Moderate);
    assert_eq!(congestion_label(0.59), CongestionLevel::Moderate);
}

#[test]
fn high_score_maps_to_high_level() {
    assert_eq!(congestion_label(0.60), CongestionLevel::High);
    assert_eq!(congestion_label(0.85), CongestionLevel::High);
}

#[test]
fn critical_score_maps_to_critical_level() {
    assert_eq!(congestion_label(0.86), CongestionLevel::Critical);
    assert_eq!(congestion_label(1.0), CongestionLevel::Critical);
}

#[test]
fn baseline_ratio_produces_zero_fee_score() {
    let input = CongestionInput {
        fee_ratio_to_baseline: 1.0,
        capacity_usage: Some(0.0),
        spike_count: 0,
        trend: TrendDirection::Stable,
    };
    let score = congestion_score(&input);
    assert_eq!(
        score, 0.0,
        "at baseline with no capacity/spikes/trend → 0.0"
    );
}

#[test]
fn high_capacity_produces_high_score() {
    let input = CongestionInput {
        fee_ratio_to_baseline: 1.0,
        capacity_usage: Some(1.0),
        spike_count: 0,
        trend: TrendDirection::Stable,
    };
    let score = congestion_score(&input);
    // 0.45 * 1.0 + 0.25 * 0.0 + 0.20 * 0.0 + 0.10 * 0.0 = 0.45
    assert!(
        (score - 0.45).abs() < 0.01,
        "capacity=1.0 → ~0.45: {}",
        score
    );
}

#[test]
fn missing_capacity_redistributes_weight() {
    let input_no_cap = CongestionInput {
        fee_ratio_to_baseline: 5.0,
        capacity_usage: None,
        spike_count: 5,
        trend: TrendDirection::Rising,
    };
    let input_with_cap = CongestionInput {
        fee_ratio_to_baseline: 5.0,
        capacity_usage: Some(0.0),
        spike_count: 5,
        trend: TrendDirection::Rising,
    };

    let score_no_cap = congestion_score(&input_no_cap);
    let score_with_cap = congestion_score(&input_with_cap);

    // No-capacity score should be higher because fee weight increased
    assert!(
        score_no_cap > score_with_cap,
        "no-capacity score ({}) should exceed with-capacity ({})",
        score_no_cap,
        score_with_cap
    );
}

#[test]
fn rising_trend_increases_score() {
    let base = CongestionInput {
        fee_ratio_to_baseline: 2.0,
        capacity_usage: Some(0.3),
        spike_count: 2,
        trend: TrendDirection::Stable,
    };
    let rising = CongestionInput {
        trend: TrendDirection::Rising,
        ..base
    };

    assert!(congestion_score(&rising) > congestion_score(&base));
}

#[test]
fn falling_trend_decreases_score() {
    let base = CongestionInput {
        fee_ratio_to_baseline: 2.0,
        capacity_usage: Some(0.3),
        spike_count: 2,
        trend: TrendDirection::Stable,
    };
    let falling = CongestionInput {
        trend: TrendDirection::Falling,
        ..base
    };

    assert!(congestion_score(&falling) < congestion_score(&base));
}

#[test]
fn score_always_clamped_to_unit_range() {
    let extreme = CongestionInput {
        fee_ratio_to_baseline: 1000.0,
        capacity_usage: Some(1.0),
        spike_count: 100,
        trend: TrendDirection::Rising,
    };
    let score = congestion_score(&extreme);
    assert!(
        score >= 0.0 && score <= 1.0,
        "score out of range: {}",
        score
    );
}

#[test]
fn fee_ratio_5x_produces_moderate_fee_score() {
    // ln(5) / ln(20) ≈ 0.54
    let ratio_score = fee_ratio_to_score(5.0);
    assert!(
        (ratio_score - 0.54).abs() < 0.01,
        "5× ratio → ~0.54: {}",
        ratio_score
    );
}

#[test]
fn fee_ratio_below_1_is_zero() {
    assert_eq!(fee_ratio_to_score(0.5), 0.0);
    assert_eq!(fee_ratio_to_score(0.0), 0.0);
    assert_eq!(fee_ratio_to_score(1.0), 0.0);
}

#[test]
fn band_test_low_capacity_high_fee() {
    // 10× fee but only 10% capacity → Low (capacity dominates at 45% weight)
    let input = CongestionInput {
        fee_ratio_to_baseline: 10.0,
        capacity_usage: Some(0.1),
        spike_count: 0,
        trend: TrendDirection::Stable,
    };
    let score = congestion_score(&input);
    let level = congestion_label(score);
    // Low capacity means network isn't congested despite high fee ratio
    assert_eq!(level, CongestionLevel::Low);
}

#[test]
fn band_test_moderate_capacity_high_fee() {
    // 10× fee + 70% capacity → High
    let input = CongestionInput {
        fee_ratio_to_baseline: 10.0,
        capacity_usage: Some(0.7),
        spike_count: 0,
        trend: TrendDirection::Stable,
    };
    let score = congestion_score(&input);
    let level = congestion_label(score);
    assert!(
        level == CongestionLevel::Moderate || level == CongestionLevel::High,
        "10× fee + 70% cap → moderate/high: score={}, level={:?}",
        score,
        level
    );
}

#[test]
fn band_test_all_maxed_critical() {
    let input = CongestionInput {
        fee_ratio_to_baseline: 20.0,
        capacity_usage: Some(0.95),
        spike_count: 10,
        trend: TrendDirection::Rising,
    };
    let score = congestion_score(&input);
    let level = congestion_label(score);
    assert_eq!(level, CongestionLevel::Critical);
}

#[test]
fn low_baseline_testnet_does_not_trigger_high() {
    // Testnet-like: all fees at 100 stroops, current at 300 (3×)
    let input = CongestionInput {
        fee_ratio_to_baseline: 3.0,
        capacity_usage: Some(0.2),
        spike_count: 1,
        trend: TrendDirection::Stable,
    };
    let score = congestion_score(&input);
    let level = congestion_label(score);
    // 3× on testnet should be moderate at most
    assert!(
        level == CongestionLevel::Low || level == CongestionLevel::Moderate,
        "testnet 3× should be low/moderate: score={}, level={:?}",
        score,
        level
    );
}

#[test]
fn high_baseline_mainnet_does_not_trigger_false_critical() {
    // Mainnet-like: high absolute fees but current is at baseline
    let input = CongestionInput {
        fee_ratio_to_baseline: 1.0,
        capacity_usage: Some(0.3),
        spike_count: 0,
        trend: TrendDirection::Stable,
    };
    let score = congestion_score(&input);
    let level = congestion_label(score);
    assert_eq!(level, CongestionLevel::Low);
}

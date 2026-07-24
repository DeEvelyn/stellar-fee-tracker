use stellar_devkit::simulation::congestion_predictor::{
    congestion_label, congestion_score, CongestionInput, CongestionLabel, CongestionLevel,
    CongestionPredictor,
};

#[test]
fn low_tx_low_fee_is_low() {
    assert_eq!(CongestionPredictor::predict(50, 100), CongestionLevel::Low);
}

#[test]
fn moderate_tx_is_moderate() {
    assert_eq!(
        CongestionPredictor::predict(300, 100),
        CongestionLevel::Moderate
    );
}

#[test]
fn moderate_fee_is_moderate() {
    assert_eq!(
        CongestionPredictor::predict(50, 400),
        CongestionLevel::Moderate
    );
}

#[test]
fn high_tx_is_high() {
    assert_eq!(
        CongestionPredictor::predict(600, 100),
        CongestionLevel::High
    );
}

#[test]
fn high_fee_is_high() {
    assert_eq!(
        CongestionPredictor::predict(50, 2_000),
        CongestionLevel::High
    );
}

#[test]
fn critical_tx_is_critical() {
    assert_eq!(
        CongestionPredictor::predict(900, 100),
        CongestionLevel::Critical
    );
}

#[test]
fn critical_fee_is_critical() {
    assert_eq!(
        CongestionPredictor::predict(50, 10_000),
        CongestionLevel::Critical
    );
}

// ── Issue #253: Congestion score + label integration tests ─────────────────────

#[test]
fn congestion_score_scores_for_normal_input() {
    let input = CongestionInput {
        recent_fee_window: 50_000.0,
        capacity_usage: 0.1,
        spike_count: 0,
    };
    let score = congestion_score(&input);
    assert!((0.0..=1.0).contains(&score), "score {score} out of [0,1]");
    assert_eq!(congestion_label(score), CongestionLabel::Normal);
}

#[test]
fn congestion_score_scores_for_rising() {
    let input = CongestionInput {
        recent_fee_window: 150_000.0,
        capacity_usage: 0.4,
        spike_count: 2,
    };
    let score = congestion_score(&input);
    assert!(
        (0.3..0.6).contains(&score),
        "score {score} not in Rising band"
    );
    assert_eq!(congestion_label(score), CongestionLabel::Rising);
}

#[test]
fn congestion_score_scores_for_congested() {
    let input = CongestionInput {
        recent_fee_window: 300_000.0,
        capacity_usage: 0.7,
        spike_count: 5,
    };
    let score = congestion_score(&input);
    assert!(
        (0.6..=0.85).contains(&score),
        "score {score} not in Congested band"
    );
    assert_eq!(congestion_label(score), CongestionLabel::Congested);
}

#[test]
fn congestion_score_scores_for_critical() {
    let input = CongestionInput {
        recent_fee_window: 500_000.0,
        capacity_usage: 0.95,
        spike_count: 10,
    };
    let score = congestion_score(&input);
    assert!(score > 0.85, "score {score} not in Critical band");
    assert_eq!(congestion_label(score), CongestionLabel::Critical);
}

#[test]
fn congestion_label_exact_boundaries() {
    assert_eq!(congestion_label(0.2999), CongestionLabel::Normal);
    assert_eq!(congestion_label(0.3), CongestionLabel::Rising);
    assert_eq!(congestion_label(0.5999), CongestionLabel::Rising);
    assert_eq!(congestion_label(0.6), CongestionLabel::Congested);
    assert_eq!(congestion_label(0.85), CongestionLabel::Congested);
    assert_eq!(congestion_label(0.8501), CongestionLabel::Critical);
}

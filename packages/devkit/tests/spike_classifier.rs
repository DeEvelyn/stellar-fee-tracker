use stellar_devkit::analysis::spike_classifier::{SpikeClassifier, SpikeSeverity};

// ── classify ──────────────────────────────────────────────────────────────────

#[test]
fn classify_below_threshold_returns_none() {
    assert_eq!(SpikeClassifier::classify(199, 100), None);
}

#[test]
fn classify_low_severity() {
    assert_eq!(
        SpikeClassifier::classify(200, 100),
        Some(SpikeSeverity::Low)
    );
    assert_eq!(
        SpikeClassifier::classify(499, 100),
        Some(SpikeSeverity::Low)
    );
}

#[test]
fn classify_medium_severity() {
    assert_eq!(
        SpikeClassifier::classify(500, 100),
        Some(SpikeSeverity::Medium)
    );
    assert_eq!(
        SpikeClassifier::classify(999, 100),
        Some(SpikeSeverity::Medium)
    );
}

#[test]
fn classify_high_severity() {
    assert_eq!(
        SpikeClassifier::classify(1_000, 100),
        Some(SpikeSeverity::High)
    );
    assert_eq!(
        SpikeClassifier::classify(4_999, 100),
        Some(SpikeSeverity::High)
    );
}

#[test]
fn classify_critical_severity() {
    assert_eq!(
        SpikeClassifier::classify(5_001, 100),
        Some(SpikeSeverity::Critical)
    );
}

#[test]
fn classify_zero_baseline_returns_none() {
    assert_eq!(SpikeClassifier::classify(1_000, 0), None);
}

// ── detect ────────────────────────────────────────────────────────────────────

#[test]
fn detect_no_spikes_in_flat_sequence() {
    let fees = vec![100u64; 10];
    assert!(SpikeClassifier::detect(&fees, 100).is_empty());
}

#[test]
fn detect_single_spike_correct_count_and_severity() {
    // one Low spike surrounded by baseline
    let fees = vec![100, 100, 300, 100, 100];
    let events = SpikeClassifier::detect(&fees, 100);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].severity, SpikeSeverity::Low);
    assert_eq!(events[0].duration_ledgers, 1);
}

#[test]
fn detect_consecutive_spike_duration() {
    // three consecutive ledgers at 6× baseline → one Medium event of duration 3
    let fees = vec![100, 600, 600, 600, 100];
    let events = SpikeClassifier::detect(&fees, 100);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].severity, SpikeSeverity::Medium);
    assert_eq!(events[0].duration_ledgers, 3);
}

#[test]
fn detect_multiple_separate_spikes() {
    let fees = vec![100, 300, 100, 1_500, 100];
    let events = SpikeClassifier::detect(&fees, 100);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].severity, SpikeSeverity::Low);
    assert_eq!(events[1].severity, SpikeSeverity::High);
}

#[test]
fn detect_escalating_spike_uses_max_severity() {
    // run goes Low → Critical; event should be Critical
    let fees = vec![100, 300, 6_000, 100];
    let events = SpikeClassifier::detect(&fees, 100);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].severity, SpikeSeverity::Critical);
    assert_eq!(events[0].duration_ledgers, 2);
}

#[test]
fn detect_empty_slice_returns_empty() {
    assert!(SpikeClassifier::detect(&[], 100).is_empty());
}

// ── Issue #177: edge-case and IQR coverage ────────────────────────────────────

#[test]
fn iqr_outliers_empty_returns_empty() {
    assert!(SpikeClassifier::iqr_outliers(&[]).is_empty());
}

#[test]
fn iqr_outliers_fewer_than_4_returns_empty() {
    assert!(SpikeClassifier::iqr_outliers(&[10, 20, 30]).is_empty());
}

#[test]
fn iqr_outliers_all_equal_returns_empty() {
    // All identical values → IQR = 0, no outliers
    let fees = vec![100u64; 20];
    assert!(SpikeClassifier::iqr_outliers(&fees).is_empty());
}

#[test]
fn iqr_outliers_extreme_high_value_detected() {
    // One extreme high value among otherwise uniform data
    let mut fees = vec![100u64; 20];
    fees[7] = 1_000_000;
    let outliers = SpikeClassifier::iqr_outliers(&fees);
    assert!(
        outliers.contains(&7),
        "extreme high value at index 7 should be an outlier"
    );
}

#[test]
fn classify_exactly_2x_is_low() {
    assert_eq!(
        SpikeClassifier::classify(200, 100),
        Some(SpikeSeverity::Low)
    );
}

#[test]
fn classify_exactly_5x_is_medium() {
    assert_eq!(
        SpikeClassifier::classify(500, 100),
        Some(SpikeSeverity::Medium)
    );
}

#[test]
fn classify_exactly_10x_is_high() {
    assert_eq!(
        SpikeClassifier::classify(1_000, 100),
        Some(SpikeSeverity::High)
    );
}

#[test]
fn classify_above_50x_is_critical() {
    // 50× boundary: strictly > 50 → Critical
    assert_eq!(
        SpikeClassifier::classify(5_001, 100),
        Some(SpikeSeverity::Critical)
    );
    assert_eq!(
        SpikeClassifier::classify(10_000, 100),
        Some(SpikeSeverity::Critical)
    );
}

#[test]
fn classify_just_below_2x_is_none() {
    assert_eq!(SpikeClassifier::classify(199, 100), None);
}

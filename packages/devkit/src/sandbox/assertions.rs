//! Assertion helpers for sandbox-based fee validation.
//!
//! Provides `Result`-returning assertion functions that can be used in tests
//! to verify fee behaviour without panicking.

/// Assert that `fee` falls within `[min, max]` (inclusive).
pub fn assert_fee_in_range(fee: u64, min: u64, max: u64) -> Result<(), String> {
    if fee >= min && fee <= max {
        Ok(())
    } else {
        Err(format!(
            "Fee out of range: {fee} not in [{min}..{max}]"
        ))
    }
}

/// Assert that `fee` represents a spike over `baseline` by at least `threshold_pct`.
///
/// `threshold_pct` is a percentage value, e.g. `50.0` means the fee must be
/// at least 50 % above the baseline.
pub fn assert_spike_detected(
    fee: u64,
    baseline: u64,
    threshold_pct: f64,
) -> Result<(), String> {
    if baseline == 0 {
        return Err("baseline fee is zero — cannot compute spike".into());
    }
    let pct_increase = ((fee as f64 - baseline as f64) / baseline as f64) * 100.0;
    if pct_increase >= threshold_pct {
        Ok(())
    } else {
        Err(format!(
            "Spike not detected: fee {fee} is only {pct_increase:.2}% above baseline {baseline} (threshold: {threshold_pct}%)"
        ))
    }
}

/// Assert that the anomaly list is empty.
pub fn assert_no_anomalies(anomalies: &[String]) -> Result<(), String> {
    if anomalies.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Expected no anomalies, found {}: {}",
            anomalies.len(),
            anomalies.join("; ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_in_range_ok() {
        assert_fee_in_range(500, 100, 1000).unwrap();
    }

    #[test]
    fn fee_in_range_err() {
        assert_fee_in_range(99, 100, 1000).unwrap_err();
    }

    #[test]
    fn spike_detected_ok() {
        assert_spike_detected(1500, 1000, 50.0).unwrap();
    }

    #[test]
    fn spike_not_detected() {
        assert_spike_detected(1100, 1000, 50.0).unwrap_err();
    }

    #[test]
    fn spike_zero_baseline() {
        assert_spike_detected(100, 0, 50.0).unwrap_err();
    }

    #[test]
    fn no_anomalies_ok() {
        assert_no_anomalies(&[]).unwrap();
    }

    #[test]
    fn has_anomalies_err() {
        assert_no_anomalies(&["something went wrong".into()]).unwrap_err();
    }
}

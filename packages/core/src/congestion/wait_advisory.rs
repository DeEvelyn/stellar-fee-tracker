use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::scoring::{
    congestion_label, congestion_score, CongestionInput, CongestionLevel, TrendDirection,
};
use crate::analytics::percentile::percentile_nearest;
use crate::insights::types::FeeDataPoint;

/// Verdict on whether submitting a transaction now is advisable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitAdvisory {
    /// The recommendation: submit now or wait.
    pub recommendation: WaitRecommendation,
    /// Human-readable explanation of the verdict.
    pub reason: String,
    /// Current congestion level.
    pub congestion_level: CongestionLevel,
    /// Weighted congestion score [0.0, 1.0].
    pub congestion_score: f64,
    /// Current fee as a percentile of historical distribution (0–100).
    /// Higher = more expensive relative to recent history.
    pub current_fee_percentile: f64,
    /// Whether capacity-usage data was available for scoring.
    /// When absent, the confidence in the congestion score is degraded.
    pub capacity_usage_available: bool,
    /// Number of data points in the historical comparison window.
    pub history_sample_count: usize,
    /// The comparison window actually used (may be shorter than requested
    /// if insufficient history is available).
    pub actual_window_secs: u64,
    /// When this advisory was computed.
    pub computed_at: DateTime<Utc>,
}

/// The wait recommendation verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitRecommendation {
    /// Fees are low or moderate — submit now.
    SubmitNow,
    /// Fees are high — consider waiting for a better window.
    ConsiderWaiting,
    /// Fees are critically high — waiting is strongly advised.
    Wait,
}

/// Compute a wait advisory by comparing the current fee level against
/// the persisted multi-day history.
///
/// # Arguments
///
/// * `recent_fees` - Fee data points from the recent window (e.g., last 5 minutes).
///   The most recent fee is used as the "current fee."
/// * `historical_fees` - Fee data points from a longer window (e.g., last 24 hours).
///   Used to compute the baseline distribution for comparison.
/// * `capacity_usage` - Optional ledger capacity usage from Horizon (0.0–1.0).
/// * `spike_count` - Number of recent spikes in the last hour.
/// * `requested_window_secs` - The desired comparison window in seconds.
///   May be degraded if insufficient history is available.
///
/// # Returns
///
/// A `WaitAdvisory` with the recommendation and supporting data.
pub fn compute_wait_advisory(
    recent_fees: &[FeeDataPoint],
    historical_fees: &[FeeDataPoint],
    capacity_usage: Option<f64>,
    spike_count: u32,
    requested_window_secs: u64,
) -> WaitAdvisory {
    let computed_at = Utc::now();

    // Handle insufficient history
    if historical_fees.is_empty() {
        return WaitAdvisory {
            recommendation: WaitRecommendation::SubmitNow,
            reason: "Insufficient historical data for comparison; defaulting to submit".to_string(),
            congestion_level: CongestionLevel::Low,
            congestion_score: 0.0,
            current_fee_percentile: 50.0,
            capacity_usage_available: capacity_usage.is_some(),
            history_sample_count: 0,
            actual_window_secs: 0,
            computed_at,
        };
    }

    // Extract fee amounts
    let current_fee = recent_fees
        .last()
        .map(|p| p.fee_amount)
        .unwrap_or_else(|| historical_fees.last().map(|p| p.fee_amount).unwrap_or(100));

    let mut historical_u64: Vec<u64> = historical_fees.iter().map(|p| p.fee_amount).collect();
    historical_u64.sort_unstable();

    let historical_amounts: Vec<f64> = historical_u64.iter().map(|&v| v as f64).collect();

    // Compute current fee's percentile position in the historical distribution
    let current_fee_percentile = if historical_amounts.is_empty() {
        50.0
    } else {
        let count_below = historical_amounts
            .iter()
            .filter(|&&v| v < current_fee as f64)
            .count();
        (count_below as f64 / historical_amounts.len() as f64) * 100.0
    };

    // Compute relative metrics for congestion scoring using u64 percentile
    let historical_median = percentile_nearest(&historical_u64, 50);

    let fee_ratio = if historical_median > 0 {
        current_fee as f64 / historical_median as f64
    } else {
        1.0
    };

    // Determine trend from recent vs historical comparison
    let recent_amounts: Vec<f64> = recent_fees.iter().map(|p| p.fee_amount as f64).collect();
    let recent_avg = if recent_amounts.is_empty() {
        current_fee as f64
    } else {
        recent_amounts.iter().sum::<f64>() / recent_amounts.len() as f64
    };

    let historical_avg = historical_amounts.iter().sum::<f64>() / historical_amounts.len() as f64;

    let trend = if recent_avg > historical_avg * 1.15 {
        TrendDirection::Rising
    } else if recent_avg < historical_avg * 0.85 {
        TrendDirection::Falling
    } else {
        TrendDirection::Stable
    };

    // Compute congestion score
    let input = CongestionInput {
        fee_ratio_to_baseline: fee_ratio,
        capacity_usage,
        spike_count,
        trend,
    };
    let score = congestion_score(&input);
    let level = congestion_label(score);

    // Determine actual window used
    let actual_window_secs = if historical_fees.len() >= 10 {
        // Enough data — full window available
        requested_window_secs
    } else {
        // Limited data — degrade gracefully
        // Estimate based on the oldest point we have
        if let (Some(oldest), Some(newest)) = (historical_fees.first(), historical_fees.last()) {
            (newest.timestamp - oldest.timestamp).num_seconds().max(0) as u64
        } else {
            0
        }
    };

    // Compute recommendation
    let (recommendation, reason) = match level {
        CongestionLevel::Low => (
            WaitRecommendation::SubmitNow,
            format!(
                "Fees are below normal ({}th percentile, {:.1}× median). Submit now.",
                current_fee_percentile as u64, fee_ratio
            ),
        ),
        CongestionLevel::Moderate => {
            if current_fee_percentile > 70.0 {
                (
                    WaitRecommendation::ConsiderWaiting,
                    format!(
                        "Fees are moderately elevated ({}th percentile, {:.1}× median). \
                         Consider waiting for a better window.",
                        current_fee_percentile as u64, fee_ratio
                    ),
                )
            } else {
                (
                    WaitRecommendation::SubmitNow,
                    format!(
                        "Fees are moderate ({}th percentile, {:.1}× median). Safe to submit.",
                        current_fee_percentile as u64, fee_ratio
                    ),
                )
            }
        }
        CongestionLevel::High => (
            WaitRecommendation::ConsiderWaiting,
            format!(
                "Fees are significantly elevated ({}th percentile, {:.1}× median, score {:.2}). \
                 Consider waiting for congestion to ease.",
                current_fee_percentile as u64, fee_ratio, score
            ),
        ),
        CongestionLevel::Critical => (
            WaitRecommendation::Wait,
            format!(
                "Fees are critically high ({}th percentile, {:.1}× median, score {:.2}). \
                 Waiting is strongly advised to avoid excessive costs.",
                current_fee_percentile as u64, fee_ratio, score
            ),
        ),
    };

    WaitAdvisory {
        recommendation,
        reason,
        congestion_level: level,
        congestion_score: score,
        current_fee_percentile,
        capacity_usage_available: capacity_usage.is_some(),
        history_sample_count: historical_fees.len(),
        actual_window_secs,
        computed_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_point(fee: u64, minutes_ago: i64) -> FeeDataPoint {
        FeeDataPoint {
            fee_amount: fee,
            timestamp: Utc::now() - Duration::minutes(minutes_ago),
            transaction_hash: format!("tx_{}", fee),
            ledger_sequence: 1000 + minutes_ago as u64,
        }
    }

    #[test]
    fn clearly_low_fees_submit_now() {
        // Historical: all at 100 stroops. Current: 100. Very cheap.
        let historical: Vec<FeeDataPoint> = (0..60).map(|i| make_point(100, 120 - i)).collect();
        let recent = vec![make_point(100, 0)];

        let advisory = compute_wait_advisory(&recent, &historical, None, 0, 86400);
        assert_eq!(advisory.recommendation, WaitRecommendation::SubmitNow);
        assert_eq!(advisory.congestion_level, CongestionLevel::Low);
        assert!(advisory.congestion_score < 0.3);
    }

    #[test]
    fn clearly_congested_should_wait() {
        // Historical: all at 100 stroops. Current: 2000 (20× median).
        // 10 spikes, rising trend, capacity 0.8.
        let historical: Vec<FeeDataPoint> = (0..60).map(|i| make_point(100, 120 - i)).collect();
        let recent = vec![make_point(2000, 0)];

        let advisory = compute_wait_advisory(&recent, &historical, Some(0.8), 10, 86400);
        assert_eq!(advisory.recommendation, WaitRecommendation::Wait);
        assert_eq!(advisory.congestion_level, CongestionLevel::Critical);
        assert!(advisory.congestion_score > 0.85);
    }

    #[test]
    fn moderate_fees_submit_now_when_low_percentile() {
        // Historical: varies 50–200. Current: 150 (moderate, but within normal range).
        let mut historical: Vec<FeeDataPoint> = Vec::new();
        for i in 0..60 {
            let fee = 50 + (i % 5) as u64 * 25; // 50, 75, 100, 125, 150 cycling
            historical.push(make_point(fee, 120 - i));
        }
        let recent = vec![make_point(150, 0)];

        let advisory = compute_wait_advisory(&recent, &historical, None, 0, 86400);
        assert_eq!(advisory.recommendation, WaitRecommendation::SubmitNow);
    }

    #[test]
    fn insufficient_history_defaults_to_submit() {
        let advisory = compute_wait_advisory(&[], &[], None, 0, 86400);
        assert_eq!(advisory.recommendation, WaitRecommendation::SubmitNow);
        assert_eq!(advisory.history_sample_count, 0);
        assert!(advisory.reason.contains("Insufficient"));
    }

    #[test]
    fn mixed_signal_high_fee_low_spike() {
        // Historical: all at 100. Current: 800 (8× median, high).
        // But no spikes. Capacity available at 0.5.
        let historical: Vec<FeeDataPoint> = (0..60).map(|i| make_point(100, 120 - i)).collect();
        let recent = vec![make_point(800, 0)];

        let advisory = compute_wait_advisory(&recent, &historical, Some(0.5), 0, 86400);
        // High fee ratio should push to at least ConsiderWaiting
        assert!(
            advisory.recommendation == WaitRecommendation::ConsiderWaiting
                || advisory.recommendation == WaitRecommendation::Wait,
            "high fee should recommend waiting: {:?}",
            advisory.recommendation
        );
    }

    #[test]
    fn capacity_degradation_deducted_confidence() {
        let historical: Vec<FeeDataPoint> = (0..60).map(|i| make_point(100, 120 - i)).collect();
        let recent = vec![make_point(300, 0)];

        let with_capacity = compute_wait_advisory(&recent, &historical, Some(0.8), 2, 86400);
        let without_capacity = compute_wait_advisory(&recent, &historical, None, 2, 86400);

        assert!(with_capacity.capacity_usage_available);
        assert!(!without_capacity.capacity_usage_available);
        // Scores should differ because capacity weight is redistributed
        assert_ne!(
            with_capacity.congestion_score,
            without_capacity.congestion_score
        );
    }

    #[test]
    fn actual_window_degrades_with_insufficient_history() {
        // Only 3 data points spanning 10 minutes, but requesting 24h window
        let historical = vec![make_point(100, 10), make_point(110, 5), make_point(105, 0)];
        let recent = vec![make_point(105, 0)];

        let advisory = compute_wait_advisory(&recent, &historical, None, 0, 86400);
        // With only 10 min of data, actual window should be much less than 24h
        assert!(advisory.actual_window_secs < 86400);
        assert!(advisory.actual_window_secs > 0);
    }

    #[test]
    fn falling_fees_recommend_submit() {
        // Historical: all at 200. Recent: dropping to 80.
        let historical: Vec<FeeDataPoint> = (0..60).map(|i| make_point(200, 120 - i)).collect();
        let recent = vec![make_point(200, 5), make_point(150, 3), make_point(80, 0)];

        let advisory = compute_wait_advisory(&recent, &historical, None, 0, 86400);
        assert_eq!(advisory.recommendation, WaitRecommendation::SubmitNow);
    }
}

use serde::{Deserialize, Serialize};

/// Unified congestion level used consistently across the entire API surface.
///
/// This replaces the previous vocabulary fragmentation:
/// - `TrendIndicator` (Normal/Rising/Congested/Declining) — internal spike analysis
/// - `CongestionLabel` (Normal/Rising/Congested/Critical) — devkit
/// - `CongestionLevel` (Low/Moderate/High/Critical) — devkit raw
/// - API.md labels (Low/Moderate/High/Critical) — documentation
///
/// `CongestionLevel` is the single canonical vocabulary for all external surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CongestionLevel {
    Low,
    Moderate,
    High,
    Critical,
}

impl CongestionLevel {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            CongestionLevel::Low => "low",
            CongestionLevel::Moderate => "moderate",
            CongestionLevel::High => "high",
            CongestionLevel::Critical => "critical",
        }
    }
}

/// Input data for the weighted congestion scoring function.
///
/// All fee-based metrics should be computed relative to observed history
/// rather than absolute stroop cutoffs, because mainnet and testnet have
/// structurally different baseline fee levels.
pub struct CongestionInput {
    /// Ratio of current average fee to historical median fee.
    /// 1.0 = at the baseline; 5.0 = five times the baseline.
    pub fee_ratio_to_baseline: f64,

    /// Ledger capacity usage as a fraction (0.0–1.0).
    /// `None` when Horizon doesn't provide this signal — the scoring
    /// function will redistribute the capacity weight to other signals.
    pub capacity_usage: Option<f64>,

    /// Number of fee spikes observed in the recent window (e.g., last hour).
    pub spike_count: u32,

    /// Recent fee trend direction.
    pub trend: TrendDirection,
}

/// Trend direction for congestion scoring input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Rising,
    Stable,
    Falling,
}

/// Returns a congestion score in [0.0, 1.0] based on weighted inputs.
///
/// All thresholds are relative to observed history, not absolute stroop values.
///
/// Default weights (when capacity usage is available):
/// - `capacity_usage`: 45 %
/// - `fee_ratio_to_baseline`: 25 %
/// - `spike_count`: 20 %
/// - `trend`: 10 %
///
/// When capacity usage is absent, its weight is redistributed:
/// - `fee_ratio_to_baseline`: 45 %
/// - `spike_count`: 30 %
/// - `trend`: 25 %
pub fn congestion_score(input: &CongestionInput) -> f64 {
    let fee_score = fee_ratio_to_score(input.fee_ratio_to_baseline);
    let spike_score = (input.spike_count as f64 / 10.0).clamp(0.0, 1.0);
    let trend_score = match input.trend {
        TrendDirection::Rising => 0.6,
        TrendDirection::Falling => -0.2,
        TrendDirection::Stable => 0.0,
    };

    match input.capacity_usage {
        Some(capacity) => {
            let capacity = capacity.clamp(0.0, 1.0);
            0.45 * capacity + 0.25 * fee_score + 0.20 * spike_score + 0.10 * trend_score
        }
        None => {
            // Redistribute capacity weight proportionally to other signals.
            // No silent zero-weighting — caller must check degraded confidence.
            0.45 * fee_score + 0.30 * spike_score + 0.25 * trend_score
        }
    }
    .clamp(0.0, 1.0)
}

/// Maps a fee ratio (current/historical baseline) to a [0.0, 1.0] score.
///
/// Uses a logarithmic scale anchored to ln(20) ≈ 3.0 so that:
/// - 1× baseline  → 0.0 (at baseline)
/// - 2× baseline  → 0.23
/// - 5× baseline  → 0.54
/// - 10× baseline → 0.77
/// - 20× baseline → 1.0 (max)
pub(crate) fn fee_ratio_to_score(ratio: f64) -> f64 {
    if ratio <= 1.0 {
        0.0
    } else {
        (ratio.ln() / 20_f64.ln()).clamp(0.0, 1.0)
    }
}

/// Maps a congestion score [0.0, 1.0] to a human-readable label.
///
/// Boundaries are calibrated to produce meaningful distinctions:
/// - < 0.3 → Low (normal conditions)
/// - < 0.6 → Moderate (elevated but manageable)
/// - ≤ 0.85 → High (significant congestion)
/// - > 0.85 → Critical (severe congestion)
pub fn congestion_label(score: f64) -> CongestionLevel {
    match score {
        s if s < 0.3 => CongestionLevel::Low,
        s if s < 0.6 => CongestionLevel::Moderate,
        s if s <= 0.85 => CongestionLevel::High,
        _ => CongestionLevel::Critical,
    }
}

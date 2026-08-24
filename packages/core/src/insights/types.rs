//! Core data types for fee insights

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// A single fee data point from the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDataPoint {
    pub fee_amount: u64,
    pub timestamp: DateTime<Utc>,
    pub transaction_hash: String,
    pub ledger_sequence: u64,
}

/// Complete insights data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentInsights {
    pub rolling_averages: RollingAverages,
    pub extremes: FeeExtremes,
    pub congestion_trends: CongestionTrends,
    pub last_updated: DateTime<Utc>,
    pub data_quality: DataQuality,
}

/// Rolling averages across different time windows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingAverages {
    pub short_term: AverageResult,  // 5 minutes
    pub medium_term: AverageResult, // 1 hour
    pub long_term: AverageResult,   // 24 hours
}

/// Result of a rolling average calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AverageResult {
    pub value: f64,
    pub sample_count: usize,
    pub is_partial: bool,
    pub calculated_at: DateTime<Utc>,
    pub time_window: TimeWindow,
}

/// Time window configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TimeWindow {
    pub name: String,
    pub duration: Duration,
    pub min_samples: usize,
}

/// Fee extremes (min/max) tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeExtremes {
    pub current_min: ExtremeValue,
    pub current_max: ExtremeValue,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// An extreme fee value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtremeValue {
    pub value: u64,
    pub timestamp: DateTime<Utc>,
    pub transaction_hash: String,
}

/// Congestion trend analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CongestionTrends {
    /// Deprecated: use `congestion_level` instead.
    pub current_trend: TrendIndicator,
    /// Unified congestion level — canonical vocabulary for all API surfaces.
    pub congestion_level: CongestionLevel,
    pub recent_spikes: Vec<FeeSpike>,
    pub trend_strength: TrendStrength,
    pub predicted_duration: Option<Duration>,
}

/// A detected fee spike
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeSpike {
    pub peak_fee: u64,
    pub baseline_fee: f64,
    pub spike_ratio: f64,
    pub start_time: DateTime<Utc>,
    pub duration: Duration,
    pub severity: SpikeSeverity,
}

/// Trend indicator for congestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendIndicator {
    Normal,
    Rising,
    Congested,
    Declining,
}

/// Unified congestion level — the canonical vocabulary for all API surfaces.
///
/// Maps from `TrendIndicator` for backward compatibility, but this is the
/// single label set that should be used in all responses going forward:
/// - `/insights/congestion`
/// - `/fees/recommend`
/// - `/fees/trend`
/// - `/fees/wait-advisory` (new)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CongestionLevel {
    Low,
    Moderate,
    High,
    Critical,
}

impl CongestionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            CongestionLevel::Low => "low",
            CongestionLevel::Moderate => "moderate",
            CongestionLevel::High => "high",
            CongestionLevel::Critical => "critical",
        }
    }
}

impl From<&TrendIndicator> for CongestionLevel {
    fn from(trend: &TrendIndicator) -> Self {
        match trend {
            TrendIndicator::Normal => CongestionLevel::Low,
            TrendIndicator::Rising => CongestionLevel::Moderate,
            TrendIndicator::Congested => CongestionLevel::High,
            TrendIndicator::Declining => CongestionLevel::Low,
        }
    }
}

/// Strength of a congestion trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendStrength {
    Weak,
    Moderate,
    Strong,
}

/// Severity classification for fee spikes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpikeSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
}

/// Data quality indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuality {
    pub completeness: f64, // 0.0 to 1.0
    pub freshness: Duration,
    pub has_gaps: bool,
    pub last_gap: Option<DateTime<Utc>>,
}

/// Update result from processing fee data
#[derive(Debug, Clone)]
pub struct InsightsUpdate {
    pub insights: CurrentInsights,
    #[allow(dead_code)]
    pub processing_time: Duration,
    pub data_points_processed: usize,
}

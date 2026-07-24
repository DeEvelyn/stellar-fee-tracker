#[derive(Debug, Clone)]
pub struct RecommendationConfig {
    pub default_confidence: f64,
    pub default_ledgers: u8,
    pub history_window_secs: u64,
    pub cache_ttl_secs: u64,
    pub min_sample_count: usize,
}

impl Default for RecommendationConfig {
    fn default() -> Self {
        Self {
            default_confidence: 0.95,
            default_ledgers: 2,
            history_window_secs: 3600,
            cache_ttl_secs: 10,
            min_sample_count: 50,
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Urgency {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendRequest {
    pub target_ledgers: Option<u32>,
    pub urgency: Option<Urgency>,
    pub max_fee: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeAlternative {
    pub fee: String,
    pub estimated_wait_ledgers: u32,
    pub confidence: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendResponse {
    pub recommended_fee: String,
    pub fee_in_stroops: u64,
    pub estimated_wait_ledgers: u32,
    pub confidence: f64,
    pub network_condition: String,
    pub alternatives: Vec<FeeAlternative>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendHistoryEntry {
    pub id: i64,
    pub requested_at: DateTime<Utc>,
    pub target_ledgers: u32,
    pub urgency: String,
    pub recommended_fee: u64,
    pub actual_confirmed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendHistoryResponse {
    pub entries: Vec<RecommendHistoryEntry>,
}


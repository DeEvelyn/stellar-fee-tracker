use serde::Deserialize;

/// Parsed representation of Horizon /fee_stats response.
#[derive(Debug, Clone, Deserialize)]
pub struct HorizonFeeStats {
    pub last_ledger_base_fee: u64,
    pub ledger_capacity_usage: f64,
    pub min: Option<u64>,
    pub mode: Option<u64>,
    pub max: Option<u64>,
    pub p10: Option<u64>,
    pub p20: Option<u64>,
    pub p30: Option<u64>,
    pub min_accepted_fee: Option<u64>,
    pub max_accepted_fee: Option<u64>,
    pub transaction_count_estimate: Option<u64>,
    #[serde(default)]
    pub fee_charged: Option<FeeLevel>,
    #[serde(default)]
    pub max_fee: Option<FeeLevel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeeLevel {
    pub min: Option<u64>,
    pub mode: Option<u64>,
    pub max: Option<u64>,
    pub p10: Option<u64>,
    pub p20: Option<u64>,
    pub p30: Option<u64>,
    pub p40: Option<u64>,
    pub p50: Option<u64>,
    pub p60: Option<u64>,
    pub p70: Option<u64>,
    pub p80: Option<u64>,
    pub p90: Option<u64>,
    pub p95: Option<u64>,
    pub p99: Option<u64>,
    pub transaction_count: Option<u64>,
    pub fee_charged: Option<FeeChargedDetail>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeeChargedDetail {
    pub max: Option<u64>,
    pub min: Option<u64>,
    pub mode: Option<u64>,
    pub p10: Option<u64>,
    pub p20: Option<u64>,
    pub p30: Option<u64>,
    pub p40: Option<u64>,
    pub p50: Option<u64>,
    pub p60: Option<u64>,
    pub p70: Option<u64>,
    pub p80: Option<u64>,
    pub p90: Option<u64>,
    pub p95: Option<u64>,
    pub p99: Option<u64>,
    pub transaction_count: Option<u64>,
    pub transaction_count_terminal: Option<u64>,
}

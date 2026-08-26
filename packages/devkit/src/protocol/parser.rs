use super::fee_stats::HorizonFeeStats;
use crate::error::DevkitError;
use serde_json::Value;

/// Parse raw Horizon /fee_stats JSON into HorizonFeeStats.
pub fn parse_fee_stats(json: &str) -> Result<HorizonFeeStats, DevkitError> {
    let value: Value = serde_json::from_str(json)
        .map_err(|e| DevkitError::Protocol(format!("JSON parse error: {e}")))?;

    let stats: HorizonFeeStats = serde_json::from_value(value.clone())
        .map_err(|e| DevkitError::Protocol(format!("field deserialization error: {e}")))?;

    validate_fee_stats(&stats)?;

    Ok(stats)
}

/// Validate parsed HorizonFeeStats against known Stellar network constraints.
pub fn validate_fee_stats(stats: &HorizonFeeStats) -> Result<(), DevkitError> {
    if stats.last_ledger_base_fee < 100 {
        return Err(DevkitError::Protocol(format!(
            "last_ledger_base_fee must be >= 100, got {}",
            stats.last_ledger_base_fee
        )));
    }

    if !(0.0..=1.0).contains(&stats.ledger_capacity_usage) {
        return Err(DevkitError::Protocol(format!(
            "ledger_capacity_usage must be in [0.0, 1.0], got {}",
            stats.ledger_capacity_usage
        )));
    }

    if let Some(ref fl) = stats.fee_charged {
        if let (Some(p10), Some(p50), Some(p90), Some(p99)) = (fl.p10, fl.p50, fl.p90, fl.p99) {
            if !(p10 <= p50 && p50 <= p90 && p90 <= p99) {
                return Err(DevkitError::Protocol(format!(
                    "fee_charged percentiles must be monotonic: p10={p10} p50={p50} p90={p90} p99={p99}"
                )));
            }
        }
        if let (Some(min), Some(mode), Some(max)) = (fl.min, fl.mode, fl.max) {
            if !(min <= mode && mode <= max) {
                return Err(DevkitError::Protocol(format!(
                    "fee_charged min/mode/max must be ordered: min={min} mode={mode} max={max}"
                )));
            }
        }
    }

    Ok(())
}

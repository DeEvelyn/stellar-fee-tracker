//! In-memory fee history store.
//!
//! `FeeHistoryStore` holds a bounded window of `FeeDataPoint` values
//! collected across polling cycles. When the store is full the oldest
//! entry is evicted before the new one is inserted (ring-buffer semantics
//! backed by `VecDeque`).
//!
//! The store itself is not `Sync` — callers wrap it in
//! `Arc<RwLock<FeeHistoryStore>>` so it can be shared between the Tokio
//! polling task and the Axum handler threads.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::insights::types::FeeDataPoint;
use crate::services::horizon::FeeStatsResponse;

/// Default maximum number of data points retained in memory.
pub const DEFAULT_CAPACITY: usize = 10_000;

/// Capacity-bounded in-memory store for `FeeDataPoint` values.
#[derive(Debug)]
pub struct FeeHistoryStore {
    data: VecDeque<FeeDataPoint>,
    capacity: usize,
}

impl FeeHistoryStore {
    /// Create a new store with the given maximum capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Append a new data point, evicting the oldest if the store is full.
    pub fn push(&mut self, point: FeeDataPoint) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(point);
    }

    /// Return all data points with a timestamp >= `since`, oldest first.
    pub fn get_since(&self, since: DateTime<Utc>) -> Vec<FeeDataPoint> {
        self.data
            .iter()
            .filter(|p| p.timestamp >= since)
            .cloned()
            .collect()
    }

    /// Return the `n` most recent data points, oldest first.
    /// If fewer than `n` points exist, all points are returned.
    #[allow(dead_code)]
    pub fn get_last_n(&self, n: usize) -> Vec<FeeDataPoint> {
        let skip = self.data.len().saturating_sub(n);
        self.data.iter().skip(skip).cloned().collect()
    }

    /// Number of data points currently held.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// `true` when the store contains no data points.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Remove all data points from the store.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// A point-in-time snapshot of Horizon's `/fee_stats` aggregate data
/// (Issue #550). One snapshot corresponds to exactly one ledger, which
/// makes persistence idempotent: re-polling the same ledger updates the
/// existing row instead of duplicating it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStatsSnapshot {
    pub ledger: u64,
    pub base_fee: u64,
    pub min_fee_charged: u64,
    pub max_fee_charged: u64,
    pub mode_fee_charged: u64,
    pub mean_fee_charged: f64,
    pub median_fee_charged: u64,
    pub p10_fee_charged: u64,
    pub p95_fee_charged: u64,
    pub p99_fee_charged: u64,
    pub max_fee: u64,
    pub ledger_capacity_usage: Option<f64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

fn parse_u64_field(raw: &str, field: &str) -> Result<u64, AppError> {
    raw.trim().parse::<u64>().map_err(|_| {
        AppError::Parse(format!("Invalid {} in fee_stats response: '{}'", field, raw))
    })
}

fn parse_f64_field(raw: &str, field: &str) -> Result<f64, AppError> {
    raw.trim().parse::<f64>().map_err(|_| {
        AppError::Parse(format!("Invalid {} in fee_stats response: '{}'", field, raw))
    })
}

impl TryFrom<&FeeStatsResponse> for FeeStatsSnapshot {
    type Error = AppError;

    fn try_from(response: &FeeStatsResponse) -> Result<Self, Self::Error> {
        let fee = &response.fee_charged;

        let ledger_capacity_usage = response
            .ledger_capacity_usage
            .as_deref()
            .map(|raw| parse_f64_field(raw, "ledger_capacity_usage"))
            .transpose()?;

        Ok(Self {
            ledger: parse_u64_field(&response.last_ledger, "last_ledger")?,
            base_fee: parse_u64_field(
                &response.last_ledger_base_fee,
                "last_ledger_base_fee",
            )?,
            min_fee_charged: parse_u64_field(&fee.min, "fee_charged.min")?,
            max_fee_charged: parse_u64_field(&fee.max, "fee_charged.max")?,
            mode_fee_charged: parse_u64_field(&fee.mode, "fee_charged.mode")?,
            mean_fee_charged: parse_f64_field(&fee.mean, "fee_charged.mean")?,
            median_fee_charged: parse_u64_field(&fee.median, "fee_charged.median")?,
            p10_fee_charged: parse_u64_field(&fee.p10, "fee_charged.p10")?,
            p95_fee_charged: parse_u64_field(&fee.p95, "fee_charged.p95")?,
            p99_fee_charged: parse_u64_field(&fee.p99, "fee_charged.p99")?,
            max_fee: parse_u64_field(&response.max_fee.max, "max_fee.max")?,
            ledger_capacity_usage,
            timestamp: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_point(fee_amount: u64, minutes_ago: i64) -> FeeDataPoint {
        FeeDataPoint {
            fee_amount,
            timestamp: Utc::now() - Duration::minutes(minutes_ago),
            transaction_hash: format!("hash_{}", fee_amount),
            ledger_sequence: fee_amount,
        }
    }

    // ---- push / capacity ----

    #[test]
    fn push_adds_point_to_store() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 1));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn push_evicts_oldest_when_at_capacity() {
        let mut store = FeeHistoryStore::new(3);
        store.push(make_point(100, 3)); // oldest
        store.push(make_point(200, 2));
        store.push(make_point(300, 1));
        // store is now full — next push evicts 100
        store.push(make_point(400, 0));

        assert_eq!(store.len(), 3);
        let all = store.get_last_n(10);
        assert_eq!(all[0].fee_amount, 200); // 100 was evicted
        assert_eq!(all[2].fee_amount, 400);
    }

    #[test]
    fn push_exactly_at_capacity_does_not_evict() {
        let mut store = FeeHistoryStore::new(3);
        store.push(make_point(1, 2));
        store.push(make_point(2, 1));
        store.push(make_point(3, 0));
        assert_eq!(store.len(), 3);
    }

    // ---- is_empty / clear ----

    #[test]
    fn new_store_is_empty() {
        let store = FeeHistoryStore::new(10);
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn clear_empties_the_store() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 1));
        store.push(make_point(200, 0));
        store.clear();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    // ---- get_since ----

    #[test]
    fn get_since_returns_points_on_or_after_cutoff() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 60)); // 60 min ago
        store.push(make_point(200, 30)); // 30 min ago
        store.push(make_point(300, 5)); // 5 min ago

        let cutoff = Utc::now() - Duration::minutes(31);
        let result = store.get_since(cutoff);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].fee_amount, 200);
        assert_eq!(result[1].fee_amount, 300);
    }

    #[test]
    fn get_since_returns_empty_when_all_points_are_before_cutoff() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 120));

        let cutoff = Utc::now() - Duration::minutes(60);
        assert!(store.get_since(cutoff).is_empty());
    }

    #[test]
    fn get_since_returns_all_when_all_points_are_after_cutoff() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 5));
        store.push(make_point(200, 2));

        let cutoff = Utc::now() - Duration::hours(1);
        assert_eq!(store.get_since(cutoff).len(), 2);
    }

    // ---- get_last_n ----

    #[test]
    fn get_last_n_returns_n_most_recent() {
        let mut store = FeeHistoryStore::new(10);
        for i in 1..=5 {
            store.push(make_point(i * 100, (6 - i) as i64));
        }

        let result = store.get_last_n(3);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].fee_amount, 300);
        assert_eq!(result[1].fee_amount, 400);
        assert_eq!(result[2].fee_amount, 500);
    }

    #[test]
    fn get_last_n_returns_all_when_n_exceeds_store_size() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 2));
        store.push(make_point(200, 1));

        let result = store.get_last_n(100);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn get_last_n_zero_returns_empty() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 1));
        assert!(store.get_last_n(0).is_empty());
    }

    #[test]
    fn get_last_n_on_empty_store_returns_empty() {
        let store = FeeHistoryStore::new(10);
        assert!(store.get_last_n(5).is_empty());
    }

    // ---- FeeStatsSnapshot ----

    fn fee_stats_response_json() -> String {
        r#"{
            "last_ledger": "50000001",
            "last_ledger_base_fee": "100",
            "ledger_capacity_usage": "0.97",
            "fee_charged": {
                "min": "100",
                "max": "5000",
                "mode": "213",
                "mean": "250.75",
                "median": "200",
                "p10": "100",
                "p20": "100",
                "p30": "120",
                "p40": "140",
                "p50": "150",
                "p60": "200",
                "p70": "300",
                "p80": "400",
                "p90": "500",
                "p95": "800",
                "p99": "1200"
            },
            "max_fee": {
                "min": "100",
                "max": "10000",
                "mode": "10000",
                "mean": "9876.5",
                "median": "10000"
            }
        }"#
        .to_string()
    }

    #[test]
    fn fee_stats_snapshot_converts_from_response() {
        let response: FeeStatsResponse =
            serde_json::from_str(&fee_stats_response_json()).unwrap();
        let snapshot = FeeStatsSnapshot::try_from(&response).unwrap();

        assert_eq!(snapshot.ledger, 50_000_001);
        assert_eq!(snapshot.base_fee, 100);
        assert_eq!(snapshot.min_fee_charged, 100);
        assert_eq!(snapshot.max_fee_charged, 5000);
        assert_eq!(snapshot.mode_fee_charged, 213);
        assert!((snapshot.mean_fee_charged - 250.75).abs() < f64::EPSILON);
        assert_eq!(snapshot.median_fee_charged, 200);
        assert_eq!(snapshot.p10_fee_charged, 100);
        assert_eq!(snapshot.p95_fee_charged, 800);
        assert_eq!(snapshot.p99_fee_charged, 1200);
        assert_eq!(snapshot.max_fee, 10_000);
        assert!((snapshot.ledger_capacity_usage.unwrap() - 0.97).abs() < f64::EPSILON);
    }

    #[test]
    fn fee_stats_snapshot_capacity_usage_is_optional() {
        let json = fee_stats_response_json().replace("\"0.97\"", "null");
        let response: FeeStatsResponse = serde_json::from_str(&json).unwrap();
        let snapshot = FeeStatsSnapshot::try_from(&response).unwrap();
        assert!(snapshot.ledger_capacity_usage.is_none());
    }

    #[test]
    fn fee_stats_snapshot_rejects_invalid_numeric_fields() {
        let json = fee_stats_response_json().replace("\"50000001\"", "\"not-a-number\"");
        let response: FeeStatsResponse = serde_json::from_str(&json).unwrap();
        let err = FeeStatsSnapshot::try_from(&response).unwrap_err();
        assert!(
            err.to_string().contains("last_ledger"),
            "error should name the offending field: {}",
            err
        );
    }
}

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

use crate::insights::types::FeeDataPoint;

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

    /// Return the first data point matching the given transaction hash, if any.
    pub fn get_by_hash(&self, hash: &str) -> Option<FeeDataPoint> {
        self.data
            .iter()
            .find(|p| p.transaction_hash == hash)
            .cloned()
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

    // ---- get_by_hash ----

    #[test]
    fn get_by_hash_returns_matching_point() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 2));
        store.push(make_point(200, 1));
        let result = store.get_by_hash("hash_200");
        assert!(result.is_some());
        assert_eq!(result.unwrap().fee_amount, 200);
    }

    #[test]
    fn get_by_hash_returns_none_when_not_found() {
        let mut store = FeeHistoryStore::new(10);
        store.push(make_point(100, 1));
        assert!(store.get_by_hash("nonexistent").is_none());
    }

    #[test]
    fn get_by_hash_on_empty_store_returns_none() {
        let store = FeeHistoryStore::new(10);
        assert!(store.get_by_hash("anything").is_none());
    }
}

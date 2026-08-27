//! Storage statistics reporter for the devkit.
//!
//! [`StatsReporter::collect`] queries a [`FeeStore`] backend and returns a
//! snapshot of key storage metrics. It is backend-agnostic and works against
//! the [`FeeStore`] trait, so it can be used with any implementation.
//!
//! ## Integration seam – `estimated_disk_bytes`
//!
//! The [`FeeStore`] trait has no method that exposes the on-disk size of the
//! database file, so [`StorageStats::estimated_disk_bytes`] is always `None`
//! for now. A future extension could add an optional `disk_bytes()` method
//! to the trait (or a separate `SqliteFeeStore` trait) and populate this
//! field for the SQLite backend specifically.

use chrono::Utc;

use crate::error::DevkitError;
use crate::storage::query::{QueryParams, SortOrder};
use crate::storage::traits::FeeStore;

/// A snapshot of storage metrics collected from a [`FeeStore`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageStats {
    /// Total number of records present in the store.
    pub total_records: usize,

    /// The `timestamp_ms` of the oldest record, or `None` if the store is empty.
    pub oldest_timestamp_ms: Option<i64>,

    /// The `timestamp_ms` of the newest record, or `None` if the store is empty.
    pub newest_timestamp_ms: Option<i64>,

    /// Estimated size of the storage backend on disk in bytes.
    ///
    /// Currently always `None` because the [`FeeStore`] trait does not expose
    /// a disk-size method. See the module-level documentation for the planned
    /// extension point.
    pub estimated_disk_bytes: Option<u64>,

    /// Number of records whose `timestamp_ms` falls within the last 24 hours.
    pub records_last_24h: usize,
}

/// Collects [`StorageStats`] by querying a [`FeeStore`].
pub struct StatsReporter;

impl StatsReporter {
    /// Query `store` and return a [`StorageStats`] snapshot.
    ///
    /// Four trait calls are made:
    /// 1. `count(default)` – total records
    /// 2. `query(Asc, limit=1)` – oldest record
    /// 3. `query(Desc, limit=1)` – newest record
    /// 4. `count(from = now − 24 h)` – records in last 24 hours
    pub async fn collect(store: &dyn FeeStore) -> Result<StorageStats, DevkitError> {
        // 1. Total record count
        let total_records = store.count(QueryParams::default()).await?;

        // 2. Oldest record – ascending order, limit 1
        let oldest_timestamp_ms = {
            let params = QueryParams {
                order: SortOrder::Asc,
                limit: Some(1),
                ..Default::default()
            };
            let rows = store.query(params).await?;
            rows.into_iter().next().map(|r| r.timestamp_ms)
        };

        // 3. Newest record – descending order, limit 1
        let newest_timestamp_ms = {
            let params = QueryParams {
                order: SortOrder::Desc,
                limit: Some(1),
                ..Default::default()
            };
            let rows = store.query(params).await?;
            rows.into_iter().next().map(|r| r.timestamp_ms)
        };

        // 4. Records in the last 24 hours
        let records_last_24h = {
            let since = Utc::now() - chrono::Duration::hours(24);
            let params = QueryParams {
                from: Some(since),
                ..Default::default()
            };
            store.count(params).await?
        };

        Ok(StorageStats {
            total_records,
            oldest_timestamp_ms,
            newest_timestamp_ms,
            // The FeeStore trait has no disk-size method; always None.
            // See module-level documentation for the planned extension point.
            estimated_disk_bytes: None,
            records_last_24h,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DevkitError;
    use crate::storage::query::{QueryParams, SortOrder};
    use crate::storage::traits::{FeeRecord, FeeStore};
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::RwLock;

    // -----------------------------------------------------------------------
    // Minimal in-process mock that supports all FeeStore methods
    // -----------------------------------------------------------------------

    struct MockStore {
        records: RwLock<Vec<FeeRecord>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                records: RwLock::new(Vec::new()),
            }
        }

        fn with_records(records: Vec<FeeRecord>) -> Self {
            Self {
                records: RwLock::new(records),
            }
        }
    }

    /// Build a minimal `FeeRecord` given only the timestamp (milliseconds
    /// since the Unix epoch).
    fn make_record(timestamp_ms: i64) -> FeeRecord {
        FeeRecord {
            fee_amount: 100,
            ledger_sequence: 1,
            timestamp_ms,
            transaction_hash: None,
            is_spike: false,
            created_at: String::from("2024-01-01T00:00:00Z"),
        }
    }

    #[async_trait]
    impl FeeStore for MockStore {
        async fn insert(&self, rec: FeeRecord) -> Result<(), DevkitError> {
            self.records.write().unwrap().push(rec);
            Ok(())
        }

        async fn insert_batch(&self, recs: Vec<FeeRecord>) -> Result<usize, DevkitError> {
            let n = recs.len();
            self.records.write().unwrap().extend(recs);
            Ok(n)
        }

        async fn query(&self, params: QueryParams) -> Result<Vec<FeeRecord>, DevkitError> {
            let guard = self.records.read().unwrap();

            // Collect matching records (clone only the fields we need for
            // filtering; we reconstruct full records below).
            let mut filtered: Vec<&FeeRecord> = guard
                .iter()
                .filter(|r| {
                    // from / to filtering based on timestamp_ms
                    if let Some(from) = params.from {
                        let from_ms = from.timestamp_millis();
                        if r.timestamp_ms < from_ms {
                            return false;
                        }
                    }
                    if let Some(to) = params.to {
                        let to_ms = to.timestamp_millis();
                        if r.timestamp_ms > to_ms {
                            return false;
                        }
                    }
                    // Optional fee range filters
                    if let Some(min) = params.min_fee {
                        if r.fee_amount < min {
                            return false;
                        }
                    }
                    if let Some(max) = params.max_fee {
                        if r.fee_amount > max {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            // Sort by timestamp_ms
            match params.order {
                SortOrder::Asc => filtered.sort_by_key(|r| r.timestamp_ms),
                SortOrder::Desc => filtered.sort_by_key(|r| std::cmp::Reverse(r.timestamp_ms)),
            }

            // Apply limit
            if let Some(limit) = params.limit {
                filtered.truncate(limit);
            }

            // Build owned results
            let result = filtered
                .into_iter()
                .map(|r| FeeRecord {
                    fee_amount: r.fee_amount,
                    ledger_sequence: r.ledger_sequence,
                    timestamp_ms: r.timestamp_ms,
                    transaction_hash: r.transaction_hash.clone(),
                    is_spike: r.is_spike,
                    created_at: r.created_at.clone(),
                })
                .collect();

            Ok(result)
        }

        async fn count(&self, params: QueryParams) -> Result<usize, DevkitError> {
            Ok(self.query(params).await?.len())
        }

        async fn delete_before(&self, before: DateTime<Utc>) -> Result<usize, DevkitError> {
            let cutoff_ms = before.timestamp_millis();
            let mut guard = self.records.write().unwrap();
            let before_len = guard.len();
            guard.retain(|r| r.timestamp_ms >= cutoff_ms);
            Ok(before_len - guard.len())
        }

        async fn latest(&self) -> Result<Option<FeeRecord>, DevkitError> {
            let guard = self.records.read().unwrap();
            let rec = guard
                .iter()
                .max_by_key(|r| r.timestamp_ms)
                .map(|r| FeeRecord {
                    fee_amount: r.fee_amount,
                    ledger_sequence: r.ledger_sequence,
                    timestamp_ms: r.timestamp_ms,
                    transaction_hash: r.transaction_hash.clone(),
                    is_spike: r.is_spike,
                    created_at: r.created_at.clone(),
                });
            Ok(rec)
        }
    }

    // -----------------------------------------------------------------------
    // test_empty_store
    // -----------------------------------------------------------------------

    /// An empty store should report zero counts and no timestamps.
    #[tokio::test]
    async fn test_empty_store() {
        let store = MockStore::new();
        let stats = StatsReporter::collect(&store).await.unwrap();

        assert_eq!(stats.total_records, 0, "total_records should be 0");
        assert!(
            stats.oldest_timestamp_ms.is_none(),
            "oldest_timestamp_ms should be None for empty store"
        );
        assert!(
            stats.newest_timestamp_ms.is_none(),
            "newest_timestamp_ms should be None for empty store"
        );
        assert_eq!(stats.records_last_24h, 0, "records_last_24h should be 0");
        assert!(
            stats.estimated_disk_bytes.is_none(),
            "estimated_disk_bytes is always None"
        );
    }

    // -----------------------------------------------------------------------
    // test_single_record
    // -----------------------------------------------------------------------

    /// A store with one fresh record should report total=1,
    /// oldest==newest==that record's timestamp, and records_last_24h=1.
    #[tokio::test]
    async fn test_single_record() {
        // Use a timestamp that is clearly within the last 24 hours
        let now_ms = Utc::now().timestamp_millis();
        let store = MockStore::with_records(vec![make_record(now_ms)]);

        let stats = StatsReporter::collect(&store).await.unwrap();

        assert_eq!(stats.total_records, 1, "total_records should be 1");
        assert_eq!(
            stats.oldest_timestamp_ms,
            Some(now_ms),
            "oldest_timestamp_ms should equal the record's timestamp"
        );
        assert_eq!(
            stats.newest_timestamp_ms,
            Some(now_ms),
            "newest_timestamp_ms should equal the record's timestamp"
        );
        assert_eq!(
            stats.records_last_24h, 1,
            "records_last_24h should be 1 for a fresh record"
        );
    }

    // -----------------------------------------------------------------------
    // test_24h_window
    // -----------------------------------------------------------------------

    /// Records older than 24 h must NOT be counted in `records_last_24h`;
    /// records within the window must be counted.
    #[tokio::test]
    async fn test_24h_window() {
        let now = Utc::now();

        // Two records well outside the 24-hour window
        let old1_ms = (now - chrono::Duration::hours(48)).timestamp_millis();
        let old2_ms = (now - chrono::Duration::hours(36)).timestamp_millis();

        // Three records inside the 24-hour window
        let recent1_ms = (now - chrono::Duration::hours(12)).timestamp_millis();
        let recent2_ms = (now - chrono::Duration::hours(6)).timestamp_millis();
        let recent3_ms = (now - chrono::Duration::minutes(30)).timestamp_millis();

        let store = MockStore::with_records(vec![
            make_record(old1_ms),
            make_record(old2_ms),
            make_record(recent1_ms),
            make_record(recent2_ms),
            make_record(recent3_ms),
        ]);

        let stats = StatsReporter::collect(&store).await.unwrap();

        assert_eq!(stats.total_records, 5, "total_records should be 5");
        assert_eq!(
            stats.oldest_timestamp_ms,
            Some(old1_ms),
            "oldest record should be old1"
        );
        assert_eq!(
            stats.newest_timestamp_ms,
            Some(recent3_ms),
            "newest record should be recent3"
        );
        assert_eq!(
            stats.records_last_24h, 3,
            "only the 3 recent records fall within the last 24 hours"
        );
        assert!(stats.estimated_disk_bytes.is_none());
    }
}

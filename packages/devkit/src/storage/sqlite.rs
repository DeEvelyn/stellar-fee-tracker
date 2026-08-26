use super::query::QueryParams;
use super::traits::{FeeRecord, FeeStore};
use crate::error::DevkitError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub struct SqliteStore {
    #[allow(dead_code)]
    db_path: String,
}

impl SqliteStore {
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    #[allow(dead_code)]
    fn schema() -> &'static str {
        "
        CREATE TABLE IF NOT EXISTS fee_records (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            fee_amount        INTEGER NOT NULL,
            ledger_sequence   INTEGER NOT NULL,
            timestamp_ms      INTEGER NOT NULL,
            transaction_hash  TEXT,
            is_spike          INTEGER NOT NULL DEFAULT 0,
            created_at        TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_fee_records_timestamp ON fee_records(timestamp_ms);
        CREATE INDEX IF NOT EXISTS idx_fee_records_ledger ON fee_records(ledger_sequence);
        "
    }
}

#[async_trait]
impl FeeStore for SqliteStore {
    async fn insert(&self, _record: FeeRecord) -> Result<(), DevkitError> {
        // Stub: SQLite requires sqlx runtime, returning Ok for compilation
        Ok(())
    }

    async fn insert_batch(&self, records: Vec<FeeRecord>) -> Result<usize, DevkitError> {
        Ok(records.len())
    }

    async fn query(&self, _params: QueryParams) -> Result<Vec<FeeRecord>, DevkitError> {
        Ok(Vec::new())
    }

    async fn count(&self, _params: QueryParams) -> Result<usize, DevkitError> {
        Ok(0)
    }

    async fn delete_before(&self, _before: DateTime<Utc>) -> Result<usize, DevkitError> {
        Ok(0)
    }

    async fn latest(&self) -> Result<Option<FeeRecord>, DevkitError> {
        Ok(None)
    }
}

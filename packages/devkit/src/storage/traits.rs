use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::query::QueryParams;
use crate::error::DevkitError;

#[derive(Debug, Clone)]
pub struct FeeRecord {
    pub fee_amount: u64,
    pub ledger_sequence: u64,
    pub timestamp_ms: i64,
    pub transaction_hash: Option<String>,
    pub is_spike: bool,
    pub created_at: String,
}

#[async_trait]
pub trait FeeStore: Send + Sync {
    async fn insert(&self, record: FeeRecord) -> Result<(), DevkitError>;
    async fn insert_batch(&self, records: Vec<FeeRecord>) -> Result<usize, DevkitError>;
    async fn query(&self, params: QueryParams) -> Result<Vec<FeeRecord>, DevkitError>;
    async fn count(&self, params: QueryParams) -> Result<usize, DevkitError>;
    async fn delete_before(&self, before: DateTime<Utc>) -> Result<usize, DevkitError>;
    async fn latest(&self) -> Result<Option<FeeRecord>, DevkitError>;
}

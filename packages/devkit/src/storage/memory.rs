use super::query::{QueryParams, SortOrder};
use super::traits::{FeeRecord, FeeStore};
use crate::error::DevkitError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::RwLock;

pub struct MemoryStore {
    records: RwLock<Vec<FeeRecord>>,
    max_capacity: usize,
}

impl MemoryStore {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            records: RwLock::new(Vec::new()),
            max_capacity,
        }
    }
}

#[async_trait]
impl FeeStore for MemoryStore {
    async fn insert(&self, record: FeeRecord) -> Result<(), DevkitError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| DevkitError::Storage(e.to_string()))?;
        if records.len() >= self.max_capacity {
            records.remove(0);
        }
        records.push(record);
        Ok(())
    }

    async fn insert_batch(&self, records: Vec<FeeRecord>) -> Result<usize, DevkitError> {
        let mut store = self
            .records
            .write()
            .map_err(|e| DevkitError::Storage(e.to_string()))?;
        let count = records.len();
        for record in records {
            if store.len() >= self.max_capacity {
                store.remove(0);
            }
            store.push(record);
        }
        Ok(count)
    }

    async fn query(&self, params: QueryParams) -> Result<Vec<FeeRecord>, DevkitError> {
        let records = self
            .records
            .read()
            .map_err(|e| DevkitError::Storage(e.to_string()))?;
        let mut result: Vec<FeeRecord> = records
            .iter()
            .filter(|r| {
                if let Some(ref from) = params.from {
                    let ts =
                        chrono::DateTime::from_timestamp_millis(r.timestamp_ms).unwrap_or_default();
                    if ts < *from {
                        return false;
                    }
                }
                if let Some(ref to) = params.to {
                    let ts =
                        chrono::DateTime::from_timestamp_millis(r.timestamp_ms).unwrap_or_default();
                    if ts > *to {
                        return false;
                    }
                }
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
                if let Some(from) = params.ledger_from {
                    if r.ledger_sequence < from {
                        return false;
                    }
                }
                if let Some(to) = params.ledger_to {
                    if r.ledger_sequence > to {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        match params.order {
            SortOrder::Desc => result.sort_by_key(|r| std::cmp::Reverse(r.timestamp_ms)),
            SortOrder::Asc => result.sort_by_key(|r| r.timestamp_ms),
        }

        if let Some(limit) = params.limit {
            result.truncate(limit);
        }

        Ok(result)
    }

    async fn count(&self, params: QueryParams) -> Result<usize, DevkitError> {
        self.query(params).await.map(|v| v.len())
    }

    async fn delete_before(&self, before: DateTime<Utc>) -> Result<usize, DevkitError> {
        let mut records = self
            .records
            .write()
            .map_err(|e| DevkitError::Storage(e.to_string()))?;
        let before_ms = before.timestamp_millis();
        let before_len = records.len();
        records.retain(|r| r.timestamp_ms >= before_ms);
        Ok(before_len - records.len())
    }

    async fn latest(&self) -> Result<Option<FeeRecord>, DevkitError> {
        let records = self
            .records
            .read()
            .map_err(|e| DevkitError::Storage(e.to_string()))?;
        Ok(records.last().cloned())
    }
}

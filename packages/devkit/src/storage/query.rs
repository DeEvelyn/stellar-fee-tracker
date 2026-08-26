use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default)]
pub struct QueryParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub min_fee: Option<u64>,
    pub max_fee: Option<u64>,
    pub ledger_from: Option<u64>,
    pub ledger_to: Option<u64>,
    pub limit: Option<usize>,
    pub order: SortOrder,
}

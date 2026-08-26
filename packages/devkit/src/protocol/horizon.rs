use super::fee_stats::HorizonFeeStats;
use crate::error::DevkitError;

pub struct HorizonClient {
    pub base_url: String,
    pub timeout_ms: u64,
}

impl HorizonClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            timeout_ms: 10_000,
        }
    }

    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub async fn fetch_fee_stats(&self) -> Result<HorizonFeeStats, DevkitError> {
        Err(DevkitError::Protocol("client not implemented".into()))
    }
}

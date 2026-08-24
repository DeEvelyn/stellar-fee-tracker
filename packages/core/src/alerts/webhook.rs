use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const REQUEST_TIMEOUT_SECONDS: u64 = 10;
const MAX_ATTEMPTS: usize = 2;
const RETRY_DELAY_SECONDS: u64 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    pub event: String,
    pub severity: String,
    pub peak_fee: u64,
    pub baseline_fee: f64,
    pub spike_ratio: f64,
    pub start_time: DateTime<Utc>,
    pub duration_seconds: i64,
    pub network: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WebhookDelivery {
    client: reqwest::Client,
    url: String,
}

#[derive(Debug, Error)]
pub enum WebhookError {
    #[error("request failed: {0}")]
    Request(String),
    #[error("unexpected HTTP status: {0}")]
    Status(u16),
}

impl WebhookDelivery {
    pub fn new(url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client, url }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    #[allow(dead_code)]
    pub async fn send(&self, payload: &AlertPayload) -> Result<(), WebhookError> {
        self.send_with_retry(payload).await
    }

    pub async fn send_with_retry(&self, payload: &AlertPayload) -> Result<(), WebhookError> {
        let mut last_error: Option<WebhookError> = None;

        for attempt in 1..=MAX_ATTEMPTS {
            match self
                .client
                .post(&self.url)
                .json(payload)
                .send()
                .await
                .map_err(|err| WebhookError::Request(err.to_string()))
            {
                Ok(response) if response.status().is_success() => {
                    tracing::info!("Webhook delivered");
                    return Ok(());
                }
                Ok(response) => {
                    last_error = Some(WebhookError::Status(response.status().as_u16()));
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }

            if attempt < MAX_ATTEMPTS {
                tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECONDS)).await;
            }
        }

        tracing::error!("Webhook delivery failed after 2 attempts");
        Err(last_error.unwrap_or_else(|| {
            WebhookError::Request("Webhook delivery failed with unknown error".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{body_json, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    fn build_payload() -> AlertPayload {
        AlertPayload {
            event: "fee_spike_detected".to_string(),
            severity: "Major".to_string(),
            peak_fee: 5000,
            baseline_fee: 130.5,
            spike_ratio: 38.3,
            start_time: DateTime::parse_from_rfc3339("2025-01-14T10:45:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration_seconds: 120,
            network: "mainnet".to_string(),
            timestamp: DateTime::parse_from_rfc3339("2025-01-14T10:47:00Z")
                .unwrap()
                .with_timezone(&Utc),
        }
    }

    #[tokio::test]
    async fn send_posts_expected_payload() {
        let server = MockServer::start().await;
        let payload = build_payload();

        Mock::given(method("POST"))
            .and(path("/hook"))
            .and(body_json(payload.clone()))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let delivery = WebhookDelivery::new(format!("{}/hook", server.uri()));
        delivery.send(&payload).await.unwrap();
    }

    #[tokio::test]
    async fn send_retries_on_non_2xx_response() {
        let server = MockServer::start().await;
        let payload = build_payload();

        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(500))
            .expect(2)
            .mount(&server)
            .await;

        let delivery = WebhookDelivery::new(format!("{}/hook", server.uri()));
        let result = delivery.send(&payload).await;
        assert!(result.is_err());
    }
}

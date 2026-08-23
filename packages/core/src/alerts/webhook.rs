use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const REQUEST_TIMEOUT_SECONDS: u64 = 10;
const MAX_ATTEMPTS: usize = 2;
const RETRY_DELAY_SECONDS: u64 = 2;

/// Webhook payload shape, generalized across all Issue #556 alert types
/// (spike | recovery | good_window | stale_data). `event`, `network`, and
/// `timestamp` apply to every type; the rest are per-type and omitted
/// from the JSON body entirely when not applicable
/// (`skip_serializing_if`), so an existing `spike` webhook consumer sees
/// byte-identical JSON to before this change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_fee: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_fee: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spike_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i64>,
    pub network: String,
    pub timestamp: DateTime<Utc>,
    /// For `recovery`, the identity of the spike it resolves. Matches
    /// `AlertEvent.correlation_id` in the DB history.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// For `good_window`, the congestion trend that triggered it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub congestion_trend: Option<String>,
    /// For `good_window`, the trend strength that triggered it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend_strength: Option<String>,
    /// For `stale_data`, how many seconds stale the data currently is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness_seconds: Option<i64>,
    /// For `stale_data`, the configured threshold that was crossed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staleness_threshold_seconds: Option<i64>,
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
            severity: Some("Major".to_string()),
            peak_fee: Some(5000),
            baseline_fee: Some(130.5),
            spike_ratio: Some(38.3),
            start_time: Some(
                DateTime::parse_from_rfc3339("2025-01-14T10:45:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            duration_seconds: Some(120),
            network: "mainnet".to_string(),
            timestamp: DateTime::parse_from_rfc3339("2025-01-14T10:47:00Z")
                .unwrap()
                .with_timezone(&Utc),
            correlation_id: None,
            congestion_trend: None,
            trend_strength: None,
            freshness_seconds: None,
            staleness_threshold_seconds: None,
        }
    }

    #[test]
    fn spike_payload_omits_none_fields_from_json() {
        // Backward compatibility check (Issue #556): a spike payload's
        // JSON shape must be byte-identical to before the AlertPayload
        // generalization — no new keys should appear for an existing
        // spike-only consumer.
        let payload = build_payload();
        let json = serde_json::to_value(&payload).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("correlation_id"));
        assert!(!obj.contains_key("congestion_trend"));
        assert!(!obj.contains_key("trend_strength"));
        assert!(!obj.contains_key("freshness_seconds"));
        assert!(!obj.contains_key("staleness_threshold_seconds"));
        assert_eq!(obj.get("severity").unwrap(), "Major");
    }

    #[test]
    fn good_window_payload_omits_spike_only_fields() {
        let payload = AlertPayload {
            event: "good_submission_window".to_string(),
            severity: None,
            peak_fee: None,
            baseline_fee: None,
            spike_ratio: None,
            start_time: None,
            duration_seconds: None,
            network: "mainnet".to_string(),
            timestamp: DateTime::parse_from_rfc3339("2025-01-14T10:47:00Z")
                .unwrap()
                .with_timezone(&Utc),
            correlation_id: None,
            congestion_trend: Some("Declining".to_string()),
            trend_strength: Some("Strong".to_string()),
            freshness_seconds: None,
            staleness_threshold_seconds: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("severity"));
        assert!(!obj.contains_key("peak_fee"));
        assert!(!obj.contains_key("start_time"));
        assert_eq!(obj.get("congestion_trend").unwrap(), "Declining");
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

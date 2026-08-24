pub mod webhook;

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::Mutex;

use crate::insights::{InsightsUpdate, SpikeSeverity};
use crate::repository::{AlertEvent, FeeRepository};

use self::webhook::{AlertPayload, WebhookDelivery};

#[derive(Clone)]
pub struct AlertManager {
    webhook_delivery: Option<WebhookDelivery>,
    alert_threshold: SpikeSeverity,
    network: String,
    seen_spikes: Arc<Mutex<HashSet<String>>>,
    repository: Option<Arc<FeeRepository>>,
}

impl AlertManager {
    pub fn new(
        webhook_url: Option<String>,
        alert_threshold: SpikeSeverity,
        network: String,
        repository: Option<Arc<FeeRepository>>,
    ) -> Self {
        let webhook_delivery = webhook_url.map(WebhookDelivery::new);
        Self {
            webhook_delivery,
            alert_threshold,
            network,
            seen_spikes: Arc::new(Mutex::new(HashSet::new())),
            repository,
        }
    }

    /// Rehydrate the in-memory dedup set from recent alert_events in the DB
    /// so that restarts do not immediately re-fire already-dispatched alerts.
    pub async fn rehydrate_seen_spikes(&self) {
        let Some(repo) = &self.repository else {
            return;
        };
        let events = match repo.query_alert_history(200, None, None).await {
            Ok(events) => events,
            Err(err) => {
                tracing::warn!("Failed to rehydrate alert dedup state: {}", err);
                return;
            }
        };
        let mut seen = self.seen_spikes.lock().await;
        for event in &events {
            let ts = chrono::DateTime::parse_from_rfc3339(&event.triggered_at)
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let id = format!("{}:{}:{}", event.severity, ts, event.peak_fee);
            seen.insert(id);
        }
        tracing::info!("Rehydrated {} seen spike IDs from alert_events", seen.len());
    }

    pub async fn check_and_dispatch(&self, update: &InsightsUpdate) {
        // Load enabled alert configs from the DB so CRUD changes take effect
        // immediately. Falls back to the in-memory webhook delivery if no
        // repository is available (legacy single-URL mode).
        let db_configs: Vec<(String, SpikeSeverity)> = if let Some(repo) = &self.repository {
            match repo.list_alert_configs().await {
                Ok(configs) => configs
                    .into_iter()
                    .filter(|c| c.enabled)
                    .filter_map(|c| {
                        let sev = match c.threshold.as_str() {
                            "Minor" => SpikeSeverity::Minor,
                            "Moderate" => SpikeSeverity::Moderate,
                            "Major" => SpikeSeverity::Major,
                            "Critical" => SpikeSeverity::Critical,
                            _ => return None,
                        };
                        Some((c.webhook_url, sev))
                    })
                    .collect(),
                Err(err) => {
                    tracing::warn!("Failed to load alert configs: {}", err);
                    vec![]
                }
            }
        } else {
            vec![]
        };

        // If no DB configs, fall back to legacy single-webhook mode
        let use_legacy = db_configs.is_empty();

        // Build the set of IDs that are still active in the congestion window.
        let active_ids: HashSet<String> = update
            .insights
            .congestion_trends
            .recent_spikes
            .iter()
            .map(|s| {
                format!(
                    "{}:{}:{}",
                    severity_to_str(&s.severity),
                    s.start_time.timestamp(),
                    s.peak_fee
                )
            })
            .collect();

        {
            let mut seen = self.seen_spikes.lock().await;
            seen.retain(|id| active_ids.contains(id));
        }

        for spike in &update.insights.congestion_trends.recent_spikes {
            let spike_id = format!(
                "{}:{}:{}",
                severity_to_str(&spike.severity),
                spike.start_time.timestamp(),
                spike.peak_fee
            );

            if use_legacy {
                // Legacy mode: single webhook, single threshold
                if !meets_threshold(&spike.severity, &self.alert_threshold) {
                    continue;
                }
                let should_dispatch = {
                    let mut seen = self.seen_spikes.lock().await;
                    seen.insert(spike_id)
                };
                if !should_dispatch {
                    continue;
                }
                if let Some(delivery) = self.webhook_delivery.clone() {
                    self.dispatch_spike(delivery, spike).await;
                }
            } else {
                // DB-config mode: dispatch to every enabled webhook whose
                // threshold this spike meets.
                for (webhook_url, threshold) in &db_configs {
                    if !meets_threshold(&spike.severity, threshold) {
                        continue;
                    }
                    let dedup_key = format!("{}:{}", spike_id, webhook_url);
                    let should_dispatch = {
                        let mut seen = self.seen_spikes.lock().await;
                        seen.insert(dedup_key)
                    };
                    if !should_dispatch {
                        continue;
                    }
                    let delivery = WebhookDelivery::new(webhook_url.clone());
                    self.dispatch_spike(delivery, spike).await;
                }
            }
        }
    }

    async fn dispatch_spike(&self, delivery: WebhookDelivery, spike: &crate::insights::FeeSpike) {
        let payload = AlertPayload {
            event: "fee_spike_detected".to_string(),
            severity: severity_to_str(&spike.severity).to_string(),
            peak_fee: spike.peak_fee,
            baseline_fee: spike.baseline_fee,
            spike_ratio: spike.spike_ratio,
            start_time: spike.start_time,
            duration_seconds: spike.duration.num_seconds().max(0),
            network: self.network.clone(),
            timestamp: Utc::now(),
        };

        let webhook_url = delivery.url().to_string();
        let severity_str = severity_to_str(&spike.severity).to_string();
        let delivery = delivery.clone();
        let repository = self.repository.clone();
        tokio::spawn(async move {
            let delivered = match delivery.send_with_retry(&payload).await {
                Ok(()) => true,
                Err(err) => {
                    tracing::error!("Webhook dispatch failed: {}", err);
                    false
                }
            };

            if let Some(repository) = repository {
                let event = AlertEvent {
                    id: None,
                    config_id: None,
                    severity: severity_str,
                    peak_fee: payload.peak_fee as i64,
                    baseline_fee: payload.baseline_fee,
                    spike_ratio: payload.spike_ratio,
                    webhook_url,
                    delivered,
                    triggered_at: Utc::now().to_rfc3339(),
                };
                if let Err(err) = repository.log_alert_event(&event).await {
                    tracing::error!("Failed to log alert event: {}", err);
                }
            }
        });
    }
}

fn severity_rank(severity: &SpikeSeverity) -> u8 {
    match severity {
        SpikeSeverity::Minor => 0,
        SpikeSeverity::Moderate => 1,
        SpikeSeverity::Major => 2,
        SpikeSeverity::Critical => 3,
    }
}

fn meets_threshold(severity: &SpikeSeverity, threshold: &SpikeSeverity) -> bool {
    severity_rank(severity) >= severity_rank(threshold)
}

fn severity_to_str(severity: &SpikeSeverity) -> &'static str {
    match severity {
        SpikeSeverity::Minor => "Minor",
        SpikeSeverity::Moderate => "Moderate",
        SpikeSeverity::Major => "Major",
        SpikeSeverity::Critical => "Critical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, Utc};
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::insights::{
        AverageResult, CongestionTrends, CurrentInsights, DataQuality, FeeSpike, RollingAverages,
        SpikeSeverity, TimeWindow, TrendIndicator, TrendStrength,
    };

    fn build_update_with_spike(severity: SpikeSeverity) -> InsightsUpdate {
        let now = DateTime::parse_from_rfc3339("2025-01-14T10:47:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let spike = FeeSpike {
            peak_fee: 5000,
            baseline_fee: 130.5,
            spike_ratio: 38.3,
            start_time: DateTime::parse_from_rfc3339("2025-01-14T10:45:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration: Duration::seconds(120),
            severity,
        };
        let window = TimeWindow {
            name: "1h".to_string(),
            duration: Duration::hours(1),
            min_samples: 1,
        };
        let avg = AverageResult {
            value: 130.5,
            sample_count: 10,
            is_partial: false,
            calculated_at: now,
            time_window: window.clone(),
        };

        InsightsUpdate {
            insights: CurrentInsights {
                rolling_averages: RollingAverages {
                    short_term: avg.clone(),
                    medium_term: avg.clone(),
                    long_term: avg,
                },
                extremes: crate::insights::FeeExtremes {
                    current_min: crate::insights::ExtremeValue {
                        value: 100,
                        timestamp: now,
                        transaction_hash: "min".to_string(),
                    },
                    current_max: crate::insights::ExtremeValue {
                        value: 5000,
                        timestamp: now,
                        transaction_hash: "max".to_string(),
                    },
                    period_start: now - Duration::hours(1),
                    period_end: now,
                },
                congestion_trends: CongestionTrends {
                    current_trend: TrendIndicator::Rising,
                    recent_spikes: vec![spike],
                    trend_strength: TrendStrength::Strong,
                    predicted_duration: None,
                },
                last_updated: now,
                data_quality: DataQuality {
                    completeness: 1.0,
                    freshness: Duration::seconds(5),
                    has_gaps: false,
                    last_gap: None,
                },
            },
            processing_time: Duration::milliseconds(1),
            data_points_processed: 1,
        }
    }

    #[tokio::test]
    async fn spike_above_threshold_dispatches_webhook() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let manager = AlertManager::new(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Major,
            "mainnet".to_string(),
            None,
        );
        let update = build_update_with_spike(SpikeSeverity::Critical);

        manager.check_and_dispatch(&update).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn spike_below_threshold_is_not_dispatched() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let manager = AlertManager::new(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Critical,
            "mainnet".to_string(),
            None,
        );
        let update = build_update_with_spike(SpikeSeverity::Major);

        manager.check_and_dispatch(&update).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn same_spike_is_dispatched_once() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let manager = AlertManager::new(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Major,
            "mainnet".to_string(),
            None,
        );
        let update = build_update_with_spike(SpikeSeverity::Major);

        manager.check_and_dispatch(&update).await;
        manager.check_and_dispatch(&update).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

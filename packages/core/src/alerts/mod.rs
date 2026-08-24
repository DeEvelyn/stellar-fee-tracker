pub mod webhook;

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::Mutex;

use crate::insights::{InsightsUpdate, SpikeSeverity};
use crate::repository::{AlertEvent, FeeRepository};
use crate::insights::{FeeSpike, InsightsUpdate, SpikeSeverity, TrendIndicator, TrendStrength};

use self::webhook::{AlertPayload, WebhookDelivery};

/// Alert type string constants, kept as local consts (mirroring
/// `crate::repository::VALID_ALERT_TYPES`) so this module doesn't need a
/// dependency on the repository/DB layer it has never needed before.
const ALERT_TYPE_SPIKE: &str = "spike";
const ALERT_TYPE_RECOVERY: &str = "recovery";
const ALERT_TYPE_GOOD_WINDOW: &str = "good_window";
const ALERT_TYPE_STALE_DATA: &str = "stale_data";

/// The spike condition `AlertManager` is currently alerting on, if any.
/// Used for both the flapping/hysteresis fix (Issue #556) and to give a
/// `recovery` alert something to correlate back to.
#[derive(Debug, Clone)]
struct ActiveSpike {
    identity: String,
    severity: SpikeSeverity,
}

#[derive(Clone)]
pub struct AlertManager {
    webhook_delivery: Option<WebhookDelivery>,
    alert_threshold: SpikeSeverity,
    network: String,
    seen_spikes: Arc<Mutex<HashSet<String>>>,
    repository: Option<Arc<FeeRepository>>,
    enabled_alert_types: Arc<HashSet<String>>,
    stale_data_threshold_seconds: i64,
    active_spike: Arc<Mutex<Option<ActiveSpike>>>,
    good_window_active: Arc<Mutex<bool>>,
    stale_data_active: Arc<Mutex<bool>>,
}

impl AlertManager {
    pub fn new(
        webhook_url: Option<String>,
        alert_threshold: SpikeSeverity,
        network: String,
        repository: Option<Arc<FeeRepository>>,
    ) -> Self {
        let default_types: HashSet<String> = [
            ALERT_TYPE_SPIKE,
            ALERT_TYPE_RECOVERY,
            ALERT_TYPE_GOOD_WINDOW,
            ALERT_TYPE_STALE_DATA,
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self::new_with_config(webhook_url, alert_threshold, network, default_types, 300)
    }

    /// Like `new`, but with explicit control over which alert types are
    /// enabled and the staleness threshold (Issue #556). `new` delegates
    /// here with all four types enabled and a 300s staleness threshold,
    /// so existing callers/tests are unaffected by this addition.
    pub fn new_with_config(
        webhook_url: Option<String>,
        alert_threshold: SpikeSeverity,
        network: String,
        enabled_alert_types: HashSet<String>,
        stale_data_threshold_seconds: i64,
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
            enabled_alert_types: Arc::new(enabled_alert_types),
            stale_data_threshold_seconds,
            active_spike: Arc::new(Mutex::new(None)),
            good_window_active: Arc::new(Mutex::new(false)),
            stale_data_active: Arc::new(Mutex::new(false)),
        }
    }

    fn type_enabled(&self, alert_type: &str) -> bool {
        self.enabled_alert_types.contains(alert_type)
    }

    pub async fn check_and_dispatch(&self, update: &InsightsUpdate) {
        let Some(delivery) = self.webhook_delivery.clone() else {
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
        // Each category is evaluated independently, and every category's
        // *state* is updated unconditionally regardless of whether that
        // type is currently enabled — only the actual webhook dispatch is
        // gated by `type_enabled`. This is what guarantees a disabled
        // alert type never blocks another enabled type from firing
        // (Issue #556 edge case), and that re-enabling a type later
        // reflects real current conditions instead of replaying a stale
        // transition.
        self.check_spike_and_recovery(update, &delivery).await;
        self.check_good_window(update, &delivery).await;
        self.check_stale_data(update, &delivery).await;
    }

    async fn check_spike_and_recovery(&self, update: &InsightsUpdate, delivery: &WebhookDelivery) {
        // Prune seen_spikes to spikes still present in this tick's window
        // — same bounding behavior as before Issue #556.
        let active_ids: HashSet<String> = update
            .insights
            .congestion_trends
            .recent_spikes
            .iter()
            .map(spike_identity)
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
        // The single highest-severity spike meeting the threshold this
        // tick, if any. Using "highest qualifying spike" rather than
        // "every qualifying spike" is what gives us hysteresis: while a
        // spike condition is active, a fee oscillating near the threshold
        // keeps producing new FeeSpike identities (different start_time /
        // peak_fee each crossing), but we only treat the *transition*
        // into/out of an active spike condition as alert-worthy, not
        // every individual spike object that shows up while still active.
        let qualifying = update
            .insights
            .congestion_trends
            .recent_spikes
            .iter()
            .filter(|s| meets_threshold(&s.severity, &self.alert_threshold))
            .max_by_key(|s| severity_rank(&s.severity));

        let mut active_guard = self.active_spike.lock().await;
        // Clone out to an owned value so matching against it doesn't hold
        // a borrow of `active_guard`, letting us freely reassign
        // `*active_guard` in every arm below.
        let current_active = active_guard.clone();

        match (qualifying, current_active) {
            (Some(spike), None) => {
                // Inactive -> active: a new spike condition starts.
                let spike_id = spike_identity(spike);
                {
                    let mut seen = self.seen_spikes.lock().await;
                    seen.insert(spike_id.clone());
                }
                *active_guard = Some(ActiveSpike {
                    identity: spike_id.clone(),
                    severity: spike.severity.clone(),
                });
                drop(active_guard);

                if self.type_enabled(ALERT_TYPE_SPIKE) {
                    let payload = AlertPayload {
                        event: "fee_spike_detected".to_string(),
                        severity: Some(severity_to_str(&spike.severity).to_string()),
                        peak_fee: Some(spike.peak_fee),
                        baseline_fee: Some(spike.baseline_fee),
                        spike_ratio: Some(spike.spike_ratio),
                        start_time: Some(spike.start_time),
                        duration_seconds: Some(spike.duration.num_seconds().max(0)),
                        network: self.network.clone(),
                        timestamp: Utc::now(),
                        correlation_id: Some(spike_id),
                        congestion_trend: None,
                        trend_strength: None,
                        freshness_seconds: None,
                        staleness_threshold_seconds: None,
                    };
                    self.dispatch(delivery, payload);
                }
            }
            (Some(spike), Some(_)) => {
                // Still active: keep bookkeeping current, but do not fire
                // a new alert. This is the flapping/cooldown fix — an
                // already-active spike condition does not re-fire no
                // matter how many new FeeSpike identities the underlying
                // detector produces while the fee stays above threshold.
                let spike_id = spike_identity(spike);
                {
                    let mut seen = self.seen_spikes.lock().await;
                    seen.insert(spike_id.clone());
                }
                *active_guard = Some(ActiveSpike {
                    identity: spike_id,
                    severity: spike.severity.clone(),
                });
            }
            (None, Some(current)) => {
                // Active -> inactive: conditions cleared, spike resolved.
                let correlation_id = current.identity.clone();
                let recovered_severity = severity_to_str(&current.severity).to_string();
                *active_guard = None;
                drop(active_guard);

                if self.type_enabled(ALERT_TYPE_RECOVERY) {
                    let payload = AlertPayload {
                        event: "fee_spike_recovered".to_string(),
                        severity: Some(recovered_severity),
                        peak_fee: None,
                        baseline_fee: None,
                        spike_ratio: None,
                        start_time: None,
                        duration_seconds: None,
                        network: self.network.clone(),
                        timestamp: Utc::now(),
                        correlation_id: Some(correlation_id),
                        congestion_trend: None,
                        trend_strength: None,
                        freshness_seconds: None,
                        staleness_threshold_seconds: None,
                    };
                    self.dispatch(delivery, payload);
                }
            }
            (None, None) => {
                // Nothing active, nothing qualifying: steady state.
            }
        }
    }

    async fn check_good_window(&self, update: &InsightsUpdate, delivery: &WebhookDelivery) {
        let trends = &update.insights.congestion_trends;
        // A "good window" is congestion visibly easing, not merely "not
        // currently spiking" — Normal doesn't imply anything improved, so
        // only Declining counts, and only at Moderate/Strong strength to
        // keep this meaningful rather than noisy (a barely-Weak decline
        // isn't a real signal to act on).
        let is_good_window = matches!(trends.current_trend, TrendIndicator::Declining)
            && !matches!(trends.trend_strength, TrendStrength::Weak);

        let mut active = self.good_window_active.lock().await;
        let was_active = *active;
        *active = is_good_window;
        let just_activated = is_good_window && !was_active;
        drop(active);

        if just_activated && self.type_enabled(ALERT_TYPE_GOOD_WINDOW) {
            let payload = AlertPayload {
                event: "good_submission_window".to_string(),
                severity: None,
                peak_fee: None,
                baseline_fee: None,
                spike_ratio: None,
                start_time: None,
                duration_seconds: None,
                network: self.network.clone(),
                timestamp: Utc::now(),
                correlation_id: None,
                congestion_trend: Some(trend_to_str(&trends.current_trend).to_string()),
                trend_strength: Some(strength_to_str(&trends.trend_strength).to_string()),
                freshness_seconds: None,
                staleness_threshold_seconds: None,
            };
            self.dispatch(delivery, payload);
        }
    }

    async fn check_stale_data(&self, update: &InsightsUpdate, delivery: &WebhookDelivery) {
        let freshness_seconds = update.insights.data_quality.freshness.num_seconds();
        let is_stale = freshness_seconds > self.stale_data_threshold_seconds;

        let mut active = self.stale_data_active.lock().await;
        let was_active = *active;
        *active = is_stale;
        let just_activated = is_stale && !was_active;
        drop(active);

        if just_activated && self.type_enabled(ALERT_TYPE_STALE_DATA) {
            let payload = AlertPayload {
                event: "data_pipeline_stale".to_string(),
                severity: None,
                peak_fee: None,
                baseline_fee: None,
                spike_ratio: None,
                start_time: None,
                duration_seconds: None,
                network: self.network.clone(),
                timestamp: Utc::now(),
                correlation_id: None,
                congestion_trend: None,
                trend_strength: None,
                freshness_seconds: Some(freshness_seconds),
                staleness_threshold_seconds: Some(self.stale_data_threshold_seconds),
            };
            self.dispatch(delivery, payload);
        }
    }

    fn dispatch(&self, delivery: &WebhookDelivery, payload: AlertPayload) {
        let delivery = delivery.clone();
        tokio::spawn(async move {
            if let Err(err) = delivery.send_with_retry(&payload).await {
                tracing::error!("Webhook dispatch failed: {}", err);
            }
        });
    }
}

fn spike_identity(spike: &FeeSpike) -> String {
    format!(
        "{}:{}:{}",
        severity_to_str(&spike.severity),
        spike.start_time.timestamp(),
        spike.peak_fee
    )
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

fn trend_to_str(trend: &TrendIndicator) -> &'static str {
    match trend {
        TrendIndicator::Normal => "Normal",
        TrendIndicator::Rising => "Rising",
        TrendIndicator::Congested => "Congested",
        TrendIndicator::Declining => "Declining",
    }
}

fn strength_to_str(strength: &TrendStrength) -> &'static str {
    match strength {
        TrendStrength::Weak => "Weak",
        TrendStrength::Moderate => "Moderate",
        TrendStrength::Strong => "Strong",
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
        AverageResult, CongestionLevel, CongestionTrends, CurrentInsights, DataQuality, FeeSpike,
        RollingAverages, SpikeSeverity, TimeWindow, TrendIndicator, TrendStrength,
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
                    congestion_level: CongestionLevel::Moderate,
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

    // ---- Issue #556: hysteresis, recovery, good_window, stale_data ----

    #[allow(clippy::too_many_arguments)]
    fn build_update_custom(
        spikes: Vec<FeeSpike>,
        trend: TrendIndicator,
        trend_strength: TrendStrength,
        freshness_seconds: i64,
    ) -> InsightsUpdate {
        let now = DateTime::parse_from_rfc3339("2025-01-14T10:47:00Z")
            .unwrap()
            .with_timezone(&Utc);
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
                    current_trend: trend,
                    recent_spikes: spikes,
                    trend_strength,
                    predicted_duration: None,
                },
                last_updated: now,
                data_quality: DataQuality {
                    completeness: 1.0,
                    freshness: Duration::seconds(freshness_seconds),
                    has_gaps: false,
                    last_gap: None,
                },
            },
            processing_time: Duration::milliseconds(1),
            data_points_processed: 1,
        }
    }

    fn make_spike(severity: SpikeSeverity, start_time: DateTime<Utc>, peak_fee: u64) -> FeeSpike {
        FeeSpike {
            peak_fee,
            baseline_fee: 130.5,
            spike_ratio: peak_fee as f64 / 130.5,
            start_time,
            duration: Duration::seconds(120),
            severity,
        }
    }

    fn all_types() -> HashSet<String> {
        [
            ALERT_TYPE_SPIKE,
            ALERT_TYPE_RECOVERY,
            ALERT_TYPE_GOOD_WINDOW,
            ALERT_TYPE_STALE_DATA,
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    #[tokio::test]
    async fn flapping_spike_does_not_double_fire() {
        // Fixed insight-sequence fixture: two ticks, each with a
        // DIFFERENT spike identity (different start_time/peak_fee) but
        // both above threshold — simulates a fee oscillating near the
        // threshold rather than one stable spike. Hysteresis means only
        // the first transition into "active" should dispatch a webhook.
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
        );

        let t1 = DateTime::parse_from_rfc3339("2025-01-14T10:45:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let t2 = DateTime::parse_from_rfc3339("2025-01-14T10:46:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let update1 = build_update_custom(
            vec![make_spike(SpikeSeverity::Major, t1, 5000)],
            TrendIndicator::Rising,
            TrendStrength::Strong,
            5,
        );
        let update2 = build_update_custom(
            vec![make_spike(SpikeSeverity::Critical, t2, 6000)],
            TrendIndicator::Rising,
            TrendStrength::Strong,
            5,
        );

        manager.check_and_dispatch(&update1).await;
        manager.check_and_dispatch(&update2).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn recovery_fires_once_conditions_clear() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(2) // one spike_detected + one spike_recovered
            .mount(&server)
            .await;

        let manager = AlertManager::new(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Major,
            "mainnet".to_string(),
        );

        let t1 = DateTime::parse_from_rfc3339("2025-01-14T10:45:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let spiking = build_update_custom(
            vec![make_spike(SpikeSeverity::Major, t1, 5000)],
            TrendIndicator::Rising,
            TrendStrength::Strong,
            5,
        );
        let cleared = build_update_custom(vec![], TrendIndicator::Normal, TrendStrength::Weak, 5);

        manager.check_and_dispatch(&spiking).await;
        manager.check_and_dispatch(&cleared).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn stale_data_fires_once_on_crossing_then_suppresses() {
        // Fixture simulating a stalled poll loop: freshness stays above
        // the threshold across two consecutive ticks. Only the crossing
        // (first tick) should fire.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let manager = AlertManager::new_with_config(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Major,
            "mainnet".to_string(),
            all_types(),
            300,
        );

        let stale_once =
            build_update_custom(vec![], TrendIndicator::Normal, TrendStrength::Weak, 400);
        let stale_again =
            build_update_custom(vec![], TrendIndicator::Normal, TrendStrength::Weak, 500);

        manager.check_and_dispatch(&stale_once).await;
        manager.check_and_dispatch(&stale_again).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn stale_data_below_threshold_does_not_fire() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let manager = AlertManager::new_with_config(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Major,
            "mainnet".to_string(),
            all_types(),
            300,
        );

        let fresh = build_update_custom(vec![], TrendIndicator::Normal, TrendStrength::Weak, 60);
        manager.check_and_dispatch(&fresh).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn good_window_fires_on_declining_trend() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let manager = AlertManager::new_with_config(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Major,
            "mainnet".to_string(),
            all_types(),
            300,
        );

        let declining = build_update_custom(
            vec![],
            TrendIndicator::Declining,
            TrendStrength::Moderate,
            5,
        );
        manager.check_and_dispatch(&declining).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn good_window_weak_strength_does_not_fire() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;

        let manager = AlertManager::new_with_config(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Major,
            "mainnet".to_string(),
            all_types(),
            300,
        );

        let weak = build_update_custom(vec![], TrendIndicator::Declining, TrendStrength::Weak, 5);
        manager.check_and_dispatch(&weak).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn disabled_type_does_not_block_other_enabled_types() {
        // Only stale_data enabled; spike disabled. An update carrying
        // BOTH a qualifying spike AND stale data should still fire
        // exactly the stale_data alert (Issue #556 edge case: "a
        // disabled alert type must not block other enabled types from
        // firing").
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let only_stale_data: HashSet<String> = [ALERT_TYPE_STALE_DATA]
            .into_iter()
            .map(String::from)
            .collect();

        let manager = AlertManager::new_with_config(
            Some(format!("{}/hook", server.uri())),
            SpikeSeverity::Major,
            "mainnet".to_string(),
            only_stale_data,
            300,
        );

        let t1 = DateTime::parse_from_rfc3339("2025-01-14T10:45:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let update = build_update_custom(
            vec![make_spike(SpikeSeverity::Critical, t1, 9000)],
            TrendIndicator::Rising,
            TrendStrength::Strong,
            400, // stale
        );

        manager.check_and_dispatch(&update).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

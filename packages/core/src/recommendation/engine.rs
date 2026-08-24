use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::analytics::percentile::{percentile_nearest, percentile_shift};
use crate::analytics::trend::{analyze_trend, TrendDirection};
use crate::error::AppError;
use crate::insights::{FeeDataPoint, FeeInsightsEngine, TrendIndicator};
use crate::repository::{FeeRepository, Recommendation};
use crate::store::FeeHistoryStore;

use super::cache::RecommendationCache;
use super::types::{
    FeeAlternative, RecommendRequest, RecommendResponse, RecommendationConfig,
    RecommendationExplanation, Urgency,
};

const MAX_TARGET_LEDGERS: u32 = 100;

pub struct FeeRecommendationEngine {
    fee_store: Arc<RwLock<FeeHistoryStore>>,
    insights_engine: Option<Arc<RwLock<FeeInsightsEngine>>>,
    repository: Option<Arc<FeeRepository>>,
    config: RecommendationConfig,
    cache: RwLock<RecommendationCache>,
}

impl FeeRecommendationEngine {
    pub fn new(
        fee_store: Arc<RwLock<FeeHistoryStore>>,
        insights_engine: Option<Arc<RwLock<FeeInsightsEngine>>>,
        config: RecommendationConfig,
    ) -> Self {
        Self {
            fee_store,
            insights_engine,
            repository: None,
            config,
            cache: RwLock::new(RecommendationCache::new(10)),
        }
    }

    pub fn with_repository(mut self, repository: Arc<FeeRepository>) -> Self {
        self.repository = Some(repository);
        self
    }

    pub async fn recommend(
        &self,
        request: &RecommendRequest,
    ) -> Result<RecommendResponse, AppError> {
        let target_ledgers = request
            .target_ledgers
            .unwrap_or(self.config.default_ledgers as u32)
            .clamp(1, MAX_TARGET_LEDGERS);

        let urgency = request.urgency.clone().unwrap_or(Urgency::Medium);

        // Normalize key for cache lookup
        let network_condition = self.detect_network_condition().await;
        let cache_key = (
            target_ledgers,
            urgency_to_label(&urgency).to_string(),
            network_condition.clone(),
        );

        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let fee_store = self.fee_store.read().await;
        let recent_points: Vec<FeeDataPoint> = fee_store.get_since(
            Utc::now() - chrono::Duration::seconds(self.config.history_window_secs as i64),
        );
        let sample_count = recent_points.len();

        // Cold-start check: trigger fallback when below min_sample_count (not just empty)
        if sample_count < self.config.min_sample_count {
            let result = self
                .fallback_recommendation(target_ledgers, &urgency)
                .await?;
            let mut cache = self.cache.write().await;
            cache.set(cache_key, result.clone());
            return Ok(result);
        let recent_points: Vec<FeeDataPoint> =
            fee_store.get_since(Utc::now() - chrono::Duration::hours(1));

        if recent_points.is_empty() {
            return self.fallback_recommendation(target_ledgers, &urgency).await;
        }

        let fees: Vec<u64> = recent_points.iter().map(|p| p.fee_amount).collect();
        let sorted = {
            let mut s = fees.clone();
            s.sort_unstable();
            s
        };

        // Multi-window percentile comparison for network condition
        let short_window_secs = (self.config.history_window_secs / 12).max(300);
        let short_points: Vec<FeeDataPoint> =
            fee_store.get_since(Utc::now() - chrono::Duration::seconds(short_window_secs as i64));
        let short_fees: Vec<u64> = short_points.iter().map(|p| p.fee_amount).collect();
        let short_sorted = {
            let mut s = short_fees.clone();
            s.sort_unstable();
            s
        };
        drop(fee_store);

        let (percentile, _label) = urgency_percentile(&urgency);
        let (percentile, basis_label) = urgency_percentile(&urgency);
        let base_fee = percentile_value(&sorted, percentile);

        // Compute explanation using multi-window comparison
        let explanation = self
            .compute_explanation(&short_sorted, &sorted, percentile, short_window_secs)
            .await;

        let max_fee = request.max_fee.as_ref().and_then(|s| s.parse::<u64>().ok());

        // Apply multi-window adjustment instead of spike-based multiplier
        let adjusted = match &explanation {
            Some(exp) => self.multi_window_adjustment(base_fee, exp).await,
            None => base_fee,
        };
        let adjusted = base_fee;

        let final_fee = match max_fee {
            Some(max) => adjusted.min(max),
            None => adjusted,
        };

        let confidence = self
            .estimate_inclusion_probability(final_fee, &sorted, target_ledgers as u8)
            .await;

        let wait_ledgers = self.estimate_wait_ledgers(final_fee, &sorted, target_ledgers);

        let alternatives = self.generate_alternatives(&sorted).await;

        let data_quality = self.get_data_quality().await;

        let result = RecommendResponse {
            recommended_fee: final_fee.to_string(),
            fee_in_stroops: final_fee,
            estimated_wait_ledgers: wait_ledgers,
            confidence,
            network_condition: network_condition.clone(),
            alternatives,
            cold_start: false,
            data_quality: Some(data_quality),
            explanation,
        };

        // Persist recommendation asynchronously — fire-and-forget.
        if let Some(repo) = &self.repository {
            let rec = Recommendation {
                id: None,
                recommended_fee: final_fee as i64,
                confidence,
                target_ledgers: target_ledgers as i64,
                network_condition: network_condition.clone(),
                percentile_basis: urgency_to_label(&urgency).to_string(),
                input_confidence: self.config.default_confidence,
                input_ledgers: self.config.default_ledgers as i64,
                sample_count: sample_count as i64,
                computed_at: Utc::now().to_rfc3339(),
            };
            let repo = repo.clone();
            tokio::spawn(async move {
                if let Err(err) = repo.insert_recommendation(&rec).await {
                    tracing::warn!("Failed to persist recommendation: {}", err);
                }
            });
        if let Some(repository) = &self.repository {
            let rec = Recommendation {
                id: None,
                recommended_fee: result.fee_in_stroops as i64,
                confidence: result.confidence,
                target_ledgers: target_ledgers as i64,
                network_condition: result.network_condition.clone(),
                percentile_basis: basis_label.to_string(),
                input_confidence: result.confidence,
                input_ledgers: target_ledgers as i64,
                sample_count: sorted.len() as i64,
                computed_at: Utc::now().to_rfc3339(),
            };
            if let Err(err) = repository.insert_recommendation(&rec).await {
                tracing::warn!("Failed to persist recommendation: {}", err);
            }
        }

        let mut cache = self.cache.write().await;
        cache.set(cache_key, result.clone());

        Ok(result)
    }

    #[allow(dead_code)]
    pub fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.cache.try_write() {
            cache.invalidate_all();
        }
    }

    #[allow(dead_code)]
    pub async fn get_last_n_fees(&self, n: usize) -> Vec<FeeDataPoint> {
        self.fee_store.read().await.get_last_n(n)
    }

    async fn fallback_recommendation(
        &self,
        target_ledgers: u32,
        urgency: &Urgency,
    ) -> Result<RecommendResponse, AppError> {
        let (base_fee, _label) = match urgency {
            Urgency::Low => (100u64, "lowest possible"),
            Urgency::Medium => (500u64, "standard"),
            Urgency::High => (1000u64, "high priority"),
            Urgency::Urgent => (5000u64, "urgent"),
        };

        let alternatives = vec![
            FeeAlternative {
                fee: "100".to_string(),
                estimated_wait_ledgers: target_ledgers.max(5),
                confidence: 0.9,
                label: "economy".to_string(),
            },
            FeeAlternative {
                fee: base_fee.to_string(),
                estimated_wait_ledgers: target_ledgers,
                confidence: 0.95,
                label: "standard".to_string(),
            },
        ];

        Ok(RecommendResponse {
            recommended_fee: base_fee.to_string(),
            fee_in_stroops: base_fee,
            estimated_wait_ledgers: target_ledgers,
            confidence: 0.85,
            network_condition: "unknown".to_string(),
            alternatives,
            cold_start: true,
            data_quality: None,
            explanation: None,
        })
    }

    async fn get_data_quality(&self) -> crate::insights::types::DataQuality {
        match &self.insights_engine {
            Some(engine) => {
                let engine = engine.read().await;
                engine.get_current_insights().data_quality
            }
            None => crate::insights::types::DataQuality {
                completeness: 0.0,
                freshness: chrono::Duration::seconds(0),
                has_gaps: true,
                last_gap: None,
            },
        }
    }

    async fn compute_explanation(
        &self,
        short_sorted: &[u64],
        long_sorted: &[u64],
        percentile: usize,
        short_window_secs: u64,
    ) -> Option<RecommendationExplanation> {
        if short_sorted.is_empty() || long_sorted.is_empty() {
            return None;
        }

        let shift = percentile_shift(short_sorted, long_sorted, percentile as u8)?;
        let short_median = percentile_nearest(short_sorted, 50);
        let long_median = percentile_nearest(long_sorted, 50);

        // Determine adjustment multiplier and reason
        let (adjustment, reason) = if shift > 30.0 {
            (
                1.25,
                format!(
                    "Short-term fees {}% above long-term baseline — network heating up",
                    shift.round()
                ),
            )
        } else if shift > 10.0 {
            (
                1.10,
                format!(
                    "Short-term fees {}% above long-term baseline — moderate pressure",
                    shift.round()
                ),
            )
        } else if shift < -15.0 {
            (
                0.92,
                format!(
                    "Short-term fees {}% below long-term baseline — fees cooling down",
                    shift.abs().round()
                ),
            )
        } else {
            (
                1.0,
                format!(
                    "Short-term fees within normal range of long-term baseline ({:+.0}%)",
                    shift
                ),
            )
        };

        // Also check trend direction for extra context
        let short_fees_f64: Vec<f64> = short_sorted.iter().map(|&f| f as f64).collect();
        let trend = analyze_trend(&short_fees_f64);

        let final_reason = match trend.direction {
            TrendDirection::Upward if trend.r_squared > 0.6 => {
                format!(
                    "{}; short-term trend is upward (slope={:.2}, R²={:.2})",
                    reason, trend.slope, trend.r_squared
                )
            }
            TrendDirection::Downward if trend.r_squared > 0.6 => {
                format!(
                    "{}; short-term trend is downward (slope={:.2}, R²={:.2})",
                    reason, trend.slope, trend.r_squared
                )
            }
            _ => reason,
        };

        Some(RecommendationExplanation {
            short_window_pct: percentile as u8,
            long_window_pct: percentile as u8,
            short_window_size: format!("{}s", short_window_secs),
            long_window_size: format!("{}s", self.config.history_window_secs),
            short_window_median: short_median,
            long_window_median: long_median,
            percentile_shift: shift,
            adjustment_applied: adjustment,
            adjustment_reason: final_reason,
        })
    }

    async fn multi_window_adjustment(
        &self,
        base_fee: u64,
        explanation: &RecommendationExplanation,
    ) -> u64 {
        (base_fee as f64 * explanation.adjustment_applied) as u64
    }

    #[allow(dead_code)]
    async fn network_condition_adjustment(&self, base_fee: u64) -> u64 {
        let condition = self.detect_network_condition().await;
        match condition.as_str() {
            "congested" => (base_fee as f64 * 1.30) as u64,
            "rising" => (base_fee as f64 * 1.15) as u64,
            "declining" => (base_fee as f64 * 0.95).max(100.0) as u64,
            _ => base_fee,
        }
    }

    async fn detect_network_condition(&self) -> String {
        match &self.insights_engine {
            Some(engine) => {
                let engine = engine.read().await;
                let insights = engine.get_current_insights();
                match insights.congestion_trends.current_trend {
                    TrendIndicator::Normal => "normal".to_string(),
                    TrendIndicator::Rising => "rising".to_string(),
                    TrendIndicator::Congested => "congested".to_string(),
                    TrendIndicator::Declining => "declining".to_string(),
                }
            }
            None => "unknown".to_string(),
        }
    }

    pub async fn estimate_inclusion_probability(
        &self,
        candidate_fee: u64,
        sorted_fees: &[u64],
        target_ledgers: u8,
    ) -> f64 {
        if sorted_fees.is_empty() {
            return 0.5;
        }

        let below_or_equal = sorted_fees.iter().filter(|&&f| f <= candidate_fee).count();
        let p_one = below_or_equal as f64 / sorted_fees.len() as f64;

        if target_ledgers <= 1 {
            return p_one.clamp(0.0, 1.0);
        }

        let p_n = 1.0 - (1.0 - p_one).powi(target_ledgers as i32);
        p_n.clamp(0.0, 1.0)
    }

    pub async fn find_fee_for_confidence(
        &self,
        sorted_fees: &[u64],
        target_confidence: f64,
        target_ledgers: u8,
    ) -> (u64, f64) {
        if sorted_fees.is_empty() {
            return (100, 0.0);
        }

        let mut lo = 0usize;
        let mut hi = sorted_fees.len() - 1;

        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let prob = self
                .estimate_inclusion_probability(sorted_fees[mid], sorted_fees, target_ledgers)
                .await;
            if prob >= target_confidence {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }

        let fee = sorted_fees[lo];
        let achieved = self
            .estimate_inclusion_probability(fee, sorted_fees, target_ledgers)
            .await;

        if achieved >= target_confidence {
            (fee, achieved)
        } else {
            let p99 = percentile_value(sorted_fees, 99);
            let extrapolated = (p99 as f64 * 1.1) as u64;
            let ext_prob = self
                .estimate_inclusion_probability(extrapolated, sorted_fees, target_ledgers)
                .await;
            (extrapolated, ext_prob)
        }
    }

    async fn generate_alternatives(&self, sorted_fees: &[u64]) -> Vec<FeeAlternative> {
        let tiers = [
            ("economy", 0.70, 5u8),
            ("standard", 0.90, 2u8),
            ("fast", 0.99, 1u8),
        ];

        let mut alternatives = Vec::with_capacity(tiers.len());
        for &(label, confidence, ledgers) in &tiers {
            let (fee, achieved) = self
                .find_fee_for_confidence(sorted_fees, confidence, ledgers)
                .await;
            alternatives.push(FeeAlternative {
                fee: fee.to_string(),
                estimated_wait_ledgers: ledgers as u32,
                confidence: achieved,
                label: label.to_string(),
            });
        }
        alternatives
    }

    fn estimate_wait_ledgers(&self, fee: u64, recent_fees: &[u64], target: u32) -> u32 {
        let p50 = percentile_value(recent_fees, 50);
        let p90 = percentile_value(recent_fees, 90);

        if fee >= p90 {
            1.min(target)
        } else if fee >= p50 {
            2.min(target)
        } else {
            5.min(target).max(1)
        }
    }
}

fn urgency_to_label(urgency: &Urgency) -> &str {
    match urgency {
        Urgency::Low => "low",
        Urgency::Medium => "medium",
        Urgency::High => "high",
        Urgency::Urgent => "urgent",
    }
}

fn urgency_percentile(urgency: &Urgency) -> (usize, &str) {
    match urgency {
        Urgency::Low => (30, "economy"),
        Urgency::Medium => (60, "standard"),
        Urgency::High => (80, "fast"),
        Urgency::Urgent => (95, "urgent"),
    }
}

fn percentile_value(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 100;
    }
    let n = sorted.len();
    let rank = ((percentile * n).saturating_add(99) / 100).max(1);
    sorted[rank - 1]
}

#[allow(dead_code)]
fn find_fee_for_target_ledgers(fees: &[u64], target_ledgers: u32, p50: u64, p99: u64) -> u64 {
    if fees.is_empty() {
        return 100;
    }

    if target_ledgers <= 1 {
        return p99;
    }
    if target_ledgers <= 3 {
        return (p50 + p99) / 2;
    }

    p50
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted_fees() -> Vec<u64> {
        vec![100, 100, 100, 150, 200, 250, 300, 500, 800, 1000]
    }

    #[test]
    fn urgency_percentile_low() {
        assert_eq!(urgency_percentile(&Urgency::Low), (30, "economy"));
    }

    #[test]
    fn urgency_percentile_medium() {
        assert_eq!(urgency_percentile(&Urgency::Medium), (60, "standard"));
    }

    #[test]
    fn urgency_percentile_high() {
        assert_eq!(urgency_percentile(&Urgency::High), (80, "fast"));
    }

    #[test]
    fn urgency_percentile_urgent() {
        assert_eq!(urgency_percentile(&Urgency::Urgent), (95, "urgent"));
    }

    #[test]
    fn percentile_value_returns_correct_value() {
        let fees = sorted_fees();
        assert_eq!(percentile_value(&fees, 50), 200);
        assert_eq!(percentile_value(&fees, 90), 800);
        assert_eq!(percentile_value(&fees, 99), 1000);
    }

    #[test]
    fn percentile_value_empty_returns_default() {
        assert_eq!(percentile_value(&[], 50), 100);
    }

    #[test]
    fn find_fee_for_target_ledgers_returns_p99_for_immediate() {
        let fees = sorted_fees();
        assert_eq!(find_fee_for_target_ledgers(&fees, 1, 200, 1000), 1000);
    }

    #[test]
    fn find_fee_for_target_ledgers_returns_mid_for_short_wait() {
        let fees = sorted_fees();
        let fee = find_fee_for_target_ledgers(&fees, 2, 200, 1000);
        assert_eq!(fee, 600);
    }

    #[test]
    fn find_fee_for_target_ledgers_returns_p50_for_long_wait() {
        let fees = sorted_fees();
        assert_eq!(find_fee_for_target_ledgers(&fees, 10, 200, 1000), 200);
    }

    #[test]
    fn find_fee_for_target_ledgers_empty_returns_default() {
        assert_eq!(find_fee_for_target_ledgers(&[], 1, 200, 1000), 100);
    }

    #[tokio::test]
    async fn detect_network_condition_without_engine_returns_unknown() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let adjusted = engine.network_condition_adjustment(200).await;
        assert_eq!(adjusted, 200);
        let engine = FeeRecommendationEngine::new(store, None);
        let condition = engine.detect_network_condition().await;
        assert_eq!(condition, "unknown");
    }

    #[tokio::test]
    async fn estimate_inclusion_probability_returns_high_for_sufficient_fee() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let fees = sorted_fees();
        let prob = engine.estimate_inclusion_probability(500, &fees, 1).await;
        assert!(prob > 0.5);
    }

    #[tokio::test]
    async fn estimate_inclusion_probability_returns_low_for_insufficient_fee() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let fees = sorted_fees();
        let prob = engine.estimate_inclusion_probability(50, &fees, 1).await;
        assert!(prob <= 0.6);
    }

    #[tokio::test]
    async fn estimate_inclusion_probability_multi_ledger() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let fees = sorted_fees();
        let p1 = engine.estimate_inclusion_probability(200, &fees, 1).await;
        let p5 = engine.estimate_inclusion_probability(200, &fees, 5).await;
        assert!(p5 > p1, "multi-ledger should increase probability");
    }

    #[tokio::test]
    async fn generate_alternatives_returns_three_tiers() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let fees = sorted_fees();
        let alternatives = engine.generate_alternatives(&fees).await;
        assert_eq!(alternatives.len(), 3);
        assert_eq!(alternatives[0].label, "economy");
        assert_eq!(alternatives[1].label, "standard");
        assert_eq!(alternatives[2].label, "fast");
    }

    #[tokio::test]
    async fn find_fee_for_confidence_returns_reasonable_value() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let fees = sorted_fees();
        let (fee, conf) = engine.find_fee_for_confidence(&fees, 0.9, 2).await;
        assert!(fee >= 100);
        assert!((0.0..=1.0).contains(&conf));
    }

    #[tokio::test]
    async fn fallback_recommendation_returns_reasonable_values() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let result = engine
            .fallback_recommendation(1, &Urgency::Medium)
            .await
            .unwrap();
        assert_eq!(result.fee_in_stroops, 500);
        assert_eq!(result.alternatives.len(), 2);
    }

    #[tokio::test]
    async fn cold_start_below_min_sample_count_triggers_fallback() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let mut store_w = store.write().await;
        let base = Utc::now() - chrono::Duration::minutes(55);
        for i in 0..10u64 {
            store_w.push(FeeDataPoint {
                fee_amount: 100 + i * 10,
                timestamp: base + chrono::Duration::minutes(i as i64 * 5),
                transaction_hash: format!("tx_{}", i),
                ledger_sequence: 100 + i,
            });
        }
        drop(store_w);

        let config = RecommendationConfig {
            min_sample_count: 50,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(result.cold_start, "should be cold start with < 50 samples");
        assert_eq!(result.fee_in_stroops, 500);
    }

    #[tokio::test]
    async fn not_cold_start_at_exact_min_sample_count() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let mut store_w = store.write().await;
        let base = Utc::now() - chrono::Duration::minutes(55);
        for i in 0..50u64 {
            store_w.push(FeeDataPoint {
                fee_amount: 100 + (i % 10) * 10,
                timestamp: base + chrono::Duration::minutes(i as i64),
                transaction_hash: format!("tx_{}", i),
                ledger_sequence: 100 + i,
            });
        }
        drop(store_w);

        let config = RecommendationConfig {
            min_sample_count: 50,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(
            !result.cold_start,
            "should NOT be cold start with exactly 50 samples"
        );
    }

    #[tokio::test]
    async fn custom_min_sample_count_changes_threshold() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let mut store_w = store.write().await;
        let base = Utc::now() - chrono::Duration::minutes(55);
        for i in 0..15u64 {
            store_w.push(FeeDataPoint {
                fee_amount: 100 + i * 10,
                timestamp: base + chrono::Duration::minutes(i as i64 * 3),
                transaction_hash: format!("tx_{}", i),
                ledger_sequence: 100 + i,
            });
        }
        drop(store_w);

        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
    async fn recommend_no_congestion_double_counting() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let mut store_w = store.write().await;
        let base = Utc::now() - chrono::Duration::hours(2);
        let fees = vec![100, 100, 200, 200, 300, 300, 400, 500, 600, 700];
        for (i, f) in fees.iter().enumerate() {
            store_w.push(FeeDataPoint {
                fee_amount: *f,
                timestamp: base + chrono::Duration::minutes(i as i64),
                transaction_hash: format!("tx_{}", i),
                ledger_sequence: 100 + i as u64,
            });
        }
        drop(store_w);
        let engine = FeeRecommendationEngine::new(store, None);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(
            !result.cold_start,
            "should NOT be cold start when min_sample_count=10 and we have 15"
        );
    }

    #[tokio::test]
    async fn all_identical_fees_does_not_panic() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let mut store_w = store.write().await;
        let base = Utc::now() - chrono::Duration::minutes(55);
        for i in 0..50u64 {
            store_w.push(FeeDataPoint {
                fee_amount: 200,
                timestamp: base + chrono::Duration::minutes(i as i64),
                transaction_hash: format!("tx_{}", i),
                ledger_sequence: 100 + i,
            });
        }
        drop(store_w);

        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(!result.cold_start);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[tokio::test]
    async fn data_quality_surfaced_in_warm_response() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let mut store_w = store.write().await;
        let base = Utc::now() - chrono::Duration::minutes(55);
        for i in 0..50u64 {
            store_w.push(FeeDataPoint {
                fee_amount: 100 + (i % 10) * 10,
                timestamp: base + chrono::Duration::minutes(i as i64),
                transaction_hash: format!("tx_{}", i),
                ledger_sequence: 100 + i,
            });
        }
        drop(store_w);

        let engine = FeeRecommendationEngine::new(store, None, RecommendationConfig::default());
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(!result.cold_start);
        assert!(result.data_quality.is_some());
        let dq = result.data_quality.unwrap();
        assert_eq!(
            dq.completeness, 0.0,
            "no insights engine means zero completeness"
        assert!(result.fee_in_stroops >= 100);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[tokio::test]
    async fn recommend_target_ledgers_1_returns_higher_fee() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let mut store_w = store.write().await;
        let base = Utc::now() - chrono::Duration::hours(2);
        let fees = vec![100, 100, 200, 200, 300, 300, 400, 500, 600, 700];
        for (i, f) in fees.iter().enumerate() {
            store_w.push(FeeDataPoint {
                fee_amount: *f,
                timestamp: base + chrono::Duration::minutes(i as i64),
                transaction_hash: format!("tx_{}", i),
                ledger_sequence: 100 + i as u64,
            });
        }
        drop(store_w);
        let engine = FeeRecommendationEngine::new(store, None);
        let req_immediate = RecommendRequest {
            target_ledgers: Some(1),
            urgency: Some(Urgency::Urgent),
            max_fee: None,
        };
        let req_long = RecommendRequest {
            target_ledgers: Some(10),
            urgency: Some(Urgency::Low),
            max_fee: None,
        };
        let r1 = engine.recommend(&req_immediate).await.unwrap();
        let r10 = engine.recommend(&req_long).await.unwrap();
        assert!(
            r1.fee_in_stroops >= r10.fee_in_stroops,
            "immediate should recommend higher fee than long wait"
        );
    }

    async fn make_store_with_fees(fees: Vec<(i64, u64)>) -> Arc<RwLock<FeeHistoryStore>> {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let base = Utc::now() - chrono::Duration::minutes(50);
        {
            let mut store_w = store.write().await;
            for (minute_offset, fee) in fees {
                store_w.push(FeeDataPoint {
                    fee_amount: fee,
                    timestamp: base + chrono::Duration::minutes(minute_offset),
                    transaction_hash: format!("tx_{}", minute_offset),
                    ledger_sequence: 100 + minute_offset as u64,
                });
            }
        }
        store
    }

    #[tokio::test]
    async fn regression_rising_fees_higher_adjustment() {
        // Long window: stable at 100, short window: rising to 200+
        let mut fees = Vec::new();
        for i in 0..30 {
            fees.push((i, 100));
        }
        for i in 30..50 {
            fees.push((i, 200));
        }
        let store = make_store_with_fees(fees).await;
        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(!result.cold_start);
        let exp = result.explanation.unwrap();
        assert!(
            exp.adjustment_applied > 1.0,
            "rising fees should increase adjustment: {}",
            exp.adjustment_applied
        );
        assert!(exp.adjustment_reason.contains("above"));
    }

    #[tokio::test]
    async fn regression_falling_fees_lower_adjustment() {
        // Long window: stable at 200, short window: dropping to 100
        let mut fees = Vec::new();
        for i in 0..30 {
            fees.push((i, 200));
        }
        for i in 30..50 {
            fees.push((i, 100));
        }
        let store = make_store_with_fees(fees).await;
        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(!result.cold_start);
        let exp = result.explanation.unwrap();
        assert!(
            exp.adjustment_applied < 1.0,
            "falling fees should decrease adjustment: {}",
            exp.adjustment_applied
        );
        assert!(exp.adjustment_reason.contains("below"));
    }

    #[tokio::test]
    async fn regression_stable_fees_no_adjustment() {
        // All fees same — no shift
        let fees: Vec<(i64, u64)> = (0..50).map(|i| (i, 150)).collect();
        let store = make_store_with_fees(fees).await;
        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        let exp = result.explanation.unwrap();
        assert_eq!(
            exp.adjustment_applied, 1.0,
            "stable fees should have no adjustment"
        );
    }

    #[tokio::test]
    async fn regression_cold_start_no_explanation() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let config = RecommendationConfig {
            min_sample_count: 50,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(result.cold_start);
        assert!(result.explanation.is_none());
    }

    #[tokio::test]
    async fn regression_explanation_matches_numeric_fields() {
        let mut fees = Vec::new();
        for i in 0..30 {
            fees.push((i, 100));
        }
        for i in 30..50 {
            fees.push((i, 180));
        }
        let store = make_store_with_fees(fees).await;
        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        let exp = result.explanation.unwrap();
        // Verify explanation is internally consistent:
        // 1. percentile_shift positive means short > long, adjustment > 1.0
        // 2. percentile_shift negative means short < long, adjustment < 1.0
        // 3. percentile_shift zero means no change, adjustment == 1.0
        if exp.percentile_shift > 0.0 {
            assert!(
                exp.adjustment_applied > 1.0,
                "positive shift should yield adjustment > 1.0: shift={}, adj={}",
                exp.percentile_shift,
                exp.adjustment_applied
            );
        } else if exp.percentile_shift < 0.0 {
            assert!(
                exp.adjustment_applied < 1.0,
                "negative shift should yield adjustment < 1.0: shift={}, adj={}",
                exp.percentile_shift,
                exp.adjustment_applied
            );
        } else {
            assert_eq!(exp.adjustment_applied, 1.0);
        }
        // short/base stats should be populated
        assert!(
            exp.short_window_median > 0,
            "short_window_median should be populated"
        );
        assert!(
            exp.long_window_median > 0,
            "long_window_median should be populated"
        );
    }

    async fn make_store_with_fees(fees: Vec<(i64, u64)>) -> Arc<RwLock<FeeHistoryStore>> {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let base = Utc::now() - chrono::Duration::minutes(50);
        {
            let mut store_w = store.write().await;
            for (minute_offset, fee) in fees {
                store_w.push(FeeDataPoint {
                    fee_amount: fee,
                    timestamp: base + chrono::Duration::minutes(minute_offset),
                    transaction_hash: format!("tx_{}", minute_offset),
                    ledger_sequence: 100 + minute_offset as u64,
                });
            }
        }
        store
    }

    #[tokio::test]
    async fn regression_rising_fees_higher_adjustment() {
        // Long window: stable at 100, short window: rising to 200+
        let mut fees = Vec::new();
        for i in 0..30 {
            fees.push((i, 100));
        }
        for i in 30..50 {
            fees.push((i, 200));
        }
        let store = make_store_with_fees(fees).await;
        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(!result.cold_start);
        let exp = result.explanation.unwrap();
        assert!(
            exp.adjustment_applied > 1.0,
            "rising fees should increase adjustment: {}",
            exp.adjustment_applied
        );
        assert!(exp.adjustment_reason.contains("above"));
    }

    #[tokio::test]
    async fn regression_falling_fees_lower_adjustment() {
        // Long window: stable at 200, short window: dropping to 100
        let mut fees = Vec::new();
        for i in 0..30 {
            fees.push((i, 200));
        }
        for i in 30..50 {
            fees.push((i, 100));
        }
        let store = make_store_with_fees(fees).await;
        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(!result.cold_start);
        let exp = result.explanation.unwrap();
        assert!(
            exp.adjustment_applied < 1.0,
            "falling fees should decrease adjustment: {}",
            exp.adjustment_applied
        );
        assert!(exp.adjustment_reason.contains("below"));
    }

    #[tokio::test]
    async fn regression_stable_fees_no_adjustment() {
        // All fees same — no shift
        let fees: Vec<(i64, u64)> = (0..50).map(|i| (i, 150)).collect();
        let store = make_store_with_fees(fees).await;
        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        let exp = result.explanation.unwrap();
        assert_eq!(
            exp.adjustment_applied, 1.0,
            "stable fees should have no adjustment"
        );
    }

    #[tokio::test]
    async fn regression_cold_start_no_explanation() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let config = RecommendationConfig {
            min_sample_count: 50,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        assert!(result.cold_start);
        assert!(result.explanation.is_none());
    }

    #[tokio::test]
    async fn regression_explanation_matches_numeric_fields() {
        let mut fees = Vec::new();
        for i in 0..30 {
            fees.push((i, 100));
        }
        for i in 30..50 {
            fees.push((i, 180));
        }
        let store = make_store_with_fees(fees).await;
        let config = RecommendationConfig {
            min_sample_count: 10,
            ..Default::default()
        };
        let engine = FeeRecommendationEngine::new(store, None, config);
        let req = RecommendRequest {
            target_ledgers: Some(2),
            urgency: Some(Urgency::Medium),
            max_fee: None,
        };
        let result = engine.recommend(&req).await.unwrap();
        let exp = result.explanation.unwrap();
        // Verify explanation is internally consistent:
        // 1. percentile_shift positive means short > long, adjustment > 1.0
        // 2. percentile_shift negative means short < long, adjustment < 1.0
        // 3. percentile_shift zero means no change, adjustment == 1.0
        if exp.percentile_shift > 0.0 {
            assert!(
                exp.adjustment_applied > 1.0,
                "positive shift should yield adjustment > 1.0: shift={}, adj={}",
                exp.percentile_shift,
                exp.adjustment_applied
            );
        } else if exp.percentile_shift < 0.0 {
            assert!(
                exp.adjustment_applied < 1.0,
                "negative shift should yield adjustment < 1.0: shift={}, adj={}",
                exp.percentile_shift,
                exp.adjustment_applied
            );
        } else {
            assert_eq!(exp.adjustment_applied, 1.0);
        }
        // short/base stats should be populated
        assert!(
            exp.short_window_median > 0,
            "short_window_median should be populated"
        );
        assert!(
            exp.long_window_median > 0,
            "long_window_median should be populated"
        );
    }
}

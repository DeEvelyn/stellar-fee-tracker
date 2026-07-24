use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::insights::{FeeDataPoint, FeeInsightsEngine, TrendIndicator};
use crate::store::FeeHistoryStore;

use super::cache::RecommendationCache;
use super::types::{FeeAlternative, RecommendRequest, RecommendResponse, Urgency};

const DEFAULT_TARGET_LEDGERS: u32 = 1;
const MAX_TARGET_LEDGERS: u32 = 100;

pub struct FeeRecommendationEngine {
    fee_store: Arc<RwLock<FeeHistoryStore>>,
    insights_engine: Option<Arc<RwLock<FeeInsightsEngine>>>,
    cache: RwLock<RecommendationCache>,
}

impl FeeRecommendationEngine {
    pub fn new(
        fee_store: Arc<RwLock<FeeHistoryStore>>,
        insights_engine: Option<Arc<RwLock<FeeInsightsEngine>>>,
    ) -> Self {
        Self {
            fee_store,
            insights_engine,
            cache: RwLock::new(RecommendationCache::new(10)),
        }
    }

    pub async fn recommend(&self, request: &RecommendRequest) -> Result<RecommendResponse, AppError> {
        let target_ledgers = request
            .target_ledgers
            .unwrap_or(DEFAULT_TARGET_LEDGERS)
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
        let recent_points: Vec<FeeDataPoint> = fee_store.get_since(Utc::now() - chrono::Duration::hours(1));

        if recent_points.is_empty() {
            return self.fallback_recommendation(target_ledgers, &urgency).await;
        }

        let fees: Vec<u64> = recent_points.iter().map(|p| p.fee_amount).collect();
        let sorted = {
            let mut s = fees.clone();
            s.sort_unstable();
            s
        };

        let (percentile, _label) = urgency_percentile(&urgency);
        let base_fee = percentile_value(&sorted, percentile);

        let max_fee = request
            .max_fee
            .as_ref()
            .and_then(|s| s.parse::<u64>().ok());

        let adjusted = self.network_condition_adjustment(base_fee).await;

        let final_fee = match max_fee {
            Some(max) => adjusted.min(max),
            None => adjusted,
        };

        let confidence = self
            .estimate_inclusion_probability(final_fee, &sorted, target_ledgers as u8)
            .await;

        let wait_ledgers = self.estimate_wait_ledgers(final_fee, &sorted, target_ledgers);

        let alternatives = self.generate_alternatives(&sorted).await;

        let result = RecommendResponse {
            recommended_fee: final_fee.to_string(),
            fee_in_stroops: final_fee,
            estimated_wait_ledgers: wait_ledgers,
            confidence,
            network_condition: network_condition.clone(),
            alternatives,
        };

        let mut cache = self.cache.write().await;
        cache.set(cache_key, result.clone());

        Ok(result)
    }

    pub fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.cache.try_write() {
            cache.invalidate_all();
        }
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
        })
    }

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

    async fn generate_alternatives(
        &self,
        sorted_fees: &[u64],
    ) -> Vec<FeeAlternative> {
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

fn find_fee_for_target_ledgers(
    fees: &[u64],
    target_ledgers: u32,
    p50: u64,
    p99: u64,
) -> u64 {
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
    async fn network_condition_adjustment_normal() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None);
        let adjusted = engine.network_condition_adjustment(200).await;
        assert_eq!(adjusted, 200);
    }

    #[tokio::test]
    async fn estimate_inclusion_probability_returns_high_for_sufficient_fee() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None);
        let fees = sorted_fees();
        let prob = engine.estimate_inclusion_probability(500, &fees, 1).await;
        assert!(prob > 0.5);
    }

    #[tokio::test]
    async fn estimate_inclusion_probability_returns_low_for_insufficient_fee() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None);
        let fees = sorted_fees();
        let prob = engine.estimate_inclusion_probability(50, &fees, 1).await;
        assert!(prob <= 0.6);
    }

    #[tokio::test]
    async fn estimate_inclusion_probability_multi_ledger() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None);
        let fees = sorted_fees();
        let p1 = engine.estimate_inclusion_probability(200, &fees, 1).await;
        let p5 = engine.estimate_inclusion_probability(200, &fees, 5).await;
        assert!(p5 > p1, "multi-ledger should increase probability");
    }

    #[tokio::test]
    async fn generate_alternatives_returns_three_tiers() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None);
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
        let engine = FeeRecommendationEngine::new(store, None);
        let fees = sorted_fees();
        let (fee, conf) = engine.find_fee_for_confidence(&fees, 0.9, 2).await;
        assert!(fee >= 100);
        assert!(conf >= 0.0 && conf <= 1.0);
    }

    #[tokio::test]
    async fn fallback_recommendation_returns_reasonable_values() {
        let store = Arc::new(RwLock::new(FeeHistoryStore::new(100)));
        let engine = FeeRecommendationEngine::new(store, None);
        let result = engine
            .fallback_recommendation(1, &Urgency::Medium)
            .await
            .unwrap();
        assert_eq!(result.fee_in_stroops, 500);
        assert_eq!(result.alternatives.len(), 2);
    }
}


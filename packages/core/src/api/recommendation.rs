use std::sync::Arc;

use axum::{extract::State, Json};
use chrono::{DateTime, Utc};

use crate::error::AppError;
use crate::metrics::AppMetrics;
use crate::middleware::validation::validate_recommend_request;
use crate::recommendation::engine::FeeRecommendationEngine;
use crate::recommendation::types::{
    RecommendHistoryEntry, RecommendHistoryResponse, RecommendRequest, RecommendResponse, Urgency,
};
use crate::repository::FeeRepository;

pub type RecommendationState = Arc<RecommendationApiState>;

pub struct RecommendationApiState {
    pub engine: FeeRecommendationEngine,
    pub metrics: Option<Arc<AppMetrics>>,
    pub repository: Option<Arc<FeeRepository>>,
}

#[derive(serde::Serialize)]
pub struct FeeRecommendResponse {
    pub recommended_fee: u64,
    pub congestion: String,
    pub basis: String,
    pub base_fee: u64,
    pub avg_fee: u64,
    pub timestamp: String,
}

pub async fn recommend(
    State(state): State<RecommendationState>,
    Json(body): Json<RecommendRequest>,
) -> Result<Json<RecommendResponse>, AppError> {
    validate_recommend_request(&body).map_err(|(status, err_json)| {
    validate_recommend_request(&body).map_err(|(_status, err_json)| {
        AppError::Parse(
            err_json["error"]
                .as_str()
                .unwrap_or("Validation error")
                .to_string(),
        )
    })?;

    let result = state.engine.recommend(&body).await?;

    if let Some(metrics) = &state.metrics {
        metrics.recommendations_total.inc();
    }

    Ok(Json(result))
}

pub async fn get_recommend(
    State(state): State<RecommendationState>,
) -> Result<Json<FeeRecommendResponse>, AppError> {
    use chrono::Utc;

    let request = RecommendRequest {
        target_ledgers: Some(2),
        urgency: Some(Urgency::Medium),
        max_fee: None,
    };

    let result = state.engine.recommend(&request).await?;
    let recent = state.engine.get_last_n_fees(100).await;

    if recent.is_empty() {
        return Err(AppError::Network(
            "No fee data available yet. Wait for the first poll cycle.".to_string(),
        ));
    }

    let mut sorted: Vec<u64> = recent.iter().map(|p| p.fee_amount).collect();
    sorted.sort_unstable();

    let base_fee = sorted[0];
    let sum: u64 = sorted.iter().sum();
    let avg_fee = sum / sorted.len() as u64;

    let ratio = avg_fee as f64 / base_fee as f64;

    let percentile_value = |sorted: &[u64], pct: usize| -> u64 {
        let idx = (pct * sorted.len()).div_ceil(100).min(sorted.len() - 1);
        sorted[idx]
    };

    let (congestion, basis, recommended_fee) = if ratio < 1.5 {
        (
            "LOW".to_string(),
            "p20".to_string(),
            percentile_value(&sorted, 20),
        )
    } else if ratio <= 3.0 {
        (
            "MEDIUM".to_string(),
            "p50".to_string(),
            percentile_value(&sorted, 50),
        )
    } else {
        (
            "HIGH".to_string(),
            "p70".to_string(),
            percentile_value(&sorted, 70),
        )
    };

    if let Some(metrics) = &state.metrics {
        metrics.recommendations_total.inc();
    }

    let network_condition = result.network_condition.clone();

    Ok(Json(FeeRecommendResponse {
        recommended_fee: result.fee_in_stroops,
        congestion: network_condition,
        basis: result
            .alternatives
            .first()
            .map(|a| a.label.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        base_fee: 0,
        avg_fee: 0,
        timestamp: Utc::now().to_rfc3339(),
    }))
}

pub async fn recommend_history(
    State(state): State<RecommendationState>,
) -> Result<Json<RecommendHistoryResponse>, AppError> {
    let entries = match &state.repository {
        Some(repository) => repository
            .query_recent_recommendations(50)
            .await
            .map_err(|err| AppError::Unknown(format!("Failed to query recommendations: {}", err)))?
            .into_iter()
            .map(|rec| RecommendHistoryEntry {
                id: rec.id.unwrap_or_default(),
                requested_at: DateTime::parse_from_rfc3339(&rec.computed_at)
                    .map(|ts| ts.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                target_ledgers: rec.target_ledgers as u32,
                urgency: rec.percentile_basis,
                recommended_fee: rec.recommended_fee as u64,
                actual_confirmed: None,
            })
            .collect(),
        None => vec![],
    };

    Ok(Json(RecommendHistoryResponse { entries }))
    let response = RecommendHistoryResponse { entries: vec![] };
    Ok(Json(response))
}

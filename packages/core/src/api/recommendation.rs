use std::sync::Arc;

use axum::{
    extract::State,
    Json,
};

use crate::error::AppError;
use crate::metrics::AppMetrics;
use crate::middleware::validation::validate_recommend_request;
use crate::recommendation::engine::FeeRecommendationEngine;
use crate::recommendation::types::{RecommendHistoryResponse, RecommendRequest, RecommendResponse};

pub type RecommendationState = Arc<RecommendationApiState>;

pub struct RecommendationApiState {
    pub engine: FeeRecommendationEngine,
    pub metrics: Option<Arc<AppMetrics>>,
}

pub async fn recommend(
    State(state): State<RecommendationState>,
    Json(body): Json<RecommendRequest>,
) -> Result<Json<RecommendResponse>, AppError> {
    validate_recommend_request(&body).map_err(|(status, err_json)| {
        AppError::Parse(err_json["error"].as_str().unwrap_or("Validation error").to_string())
    })?;

    let result = state.engine.recommend(&body).await?;

    if let Some(metrics) = &state.metrics {
        metrics.recommendations_total.inc();
    }

    Ok(Json(result))
}

pub async fn get_recommend(
    State(state): State<RecommendationState>,
) -> Result<Json<RecommendResponse>, AppError> {
    let request = RecommendRequest {
        target_ledgers: Some(2),
        urgency: None,
        max_fee: None,
    };

    let result = state.engine.recommend(&request).await?;

    if let Some(metrics) = &state.metrics {
        metrics.recommendations_total.inc();
    }

    Ok(Json(result))
}

pub async fn recommend_history(
    State(_state): State<RecommendationState>,
) -> Result<Json<RecommendHistoryResponse>, AppError> {
    let response = RecommendHistoryResponse { entries: vec![] };
    Ok(Json(response))
}


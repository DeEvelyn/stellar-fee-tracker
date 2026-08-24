use std::sync::Arc;

use axum::{extract::State, Json};

use crate::error::AppError;
use crate::metrics::AppMetrics;
use crate::middleware::validation::validate_recommend_request;
use crate::recommendation::engine::FeeRecommendationEngine;
use crate::recommendation::types::{
    RecommendHistoryEntry, RecommendHistoryResponse, RecommendRequest, RecommendResponse,
};
use crate::repository::FeeRepository;

pub type RecommendationState = Arc<RecommendationApiState>;

pub struct RecommendationApiState {
    pub engine: FeeRecommendationEngine,
    pub metrics: Option<Arc<AppMetrics>>,
    pub repository: Arc<FeeRepository>,
}

pub async fn recommend(
    State(state): State<RecommendationState>,
    Json(body): Json<RecommendRequest>,
) -> Result<Json<RecommendResponse>, AppError> {
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
) -> Result<Json<RecommendResponse>, AppError> {
    let request = RecommendRequest {
        target_ledgers: None,
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
    State(state): State<RecommendationState>,
) -> Result<Json<RecommendHistoryResponse>, AppError> {
    let recs = state
        .repository
        .query_recent_recommendations(50)
        .await
        .map_err(|e| AppError::Unknown(format!("Database query failed: {}", e)))?;

    let entries: Vec<RecommendHistoryEntry> = recs
        .into_iter()
        .filter_map(|r| {
            let requested_at = chrono::DateTime::parse_from_rfc3339(&r.computed_at)
                .ok()?
                .with_timezone(&chrono::Utc);
            Some(RecommendHistoryEntry {
                id: r.id?,
                requested_at,
                target_ledgers: r.target_ledgers as u32,
                urgency: r.percentile_basis.clone(),
                recommended_fee: r.recommended_fee as u64,
                actual_confirmed: None,
            })
        })
        .collect();

    Ok(Json(RecommendHistoryResponse { entries }))
}

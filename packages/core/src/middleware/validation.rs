use axum::http::StatusCode;
use serde_json::json;

use crate::recommendation::types::{RecommendRequest, Urgency};

const MAX_FEE_UPPER_BOUND: u64 = 100_000_000;
const MAX_TARGET_LEDGERS: u32 = 100;

pub fn validate_recommend_request(req: &RecommendRequest) -> Result<(), (StatusCode, serde_json::Value)> {
    if let Some(ledgers) = req.target_ledgers {
        if ledgers == 0 || ledgers > MAX_TARGET_LEDGERS {
            return Err((
                StatusCode::BAD_REQUEST,
                json!({
                    "error": format!(
                        "target_ledgers must be between 1 and {}, got {}",
                        MAX_TARGET_LEDGERS, ledgers
                    )
                }),
            ));
        }
    }

    if let Some(ref urgency) = req.urgency {
        let valid = matches!(urgency, Urgency::Low | Urgency::Medium | Urgency::High | Urgency::Urgent);
        if !valid {
            return Err((
                StatusCode::BAD_REQUEST,
                json!({
                    "error": format!(
                        "urgency must be one of [low, medium, high, urgent], got {:?}",
                        urgency
                    )
                }),
            ));
        }
    }

    if let Some(ref max_fee) = req.max_fee {
        match max_fee.parse::<u64>() {
            Ok(v) if v > MAX_FEE_UPPER_BOUND => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    json!({
                        "error": format!(
                            "max_fee must not exceed {} stroops, got {}",
                            MAX_FEE_UPPER_BOUND, v
                        )
                    }),
                ));
            }
            Ok(_) => {}
            Err(_) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    json!({
                        "error": format!(
                            "max_fee must be a valid integer, got '{}'",
                            max_fee
                        )
                    }),
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_request_passes() {
        let req = RecommendRequest {
            target_ledgers: Some(1),
            urgency: Some(Urgency::High),
            max_fee: Some("10000".to_string()),
        };
        assert!(validate_recommend_request(&req).is_ok());
    }

    #[test]
    fn empty_request_passes() {
        let req = RecommendRequest {
            target_ledgers: None,
            urgency: None,
            max_fee: None,
        };
        assert!(validate_recommend_request(&req).is_ok());
    }

    #[test]
    fn target_ledgers_zero_rejected() {
        let req = RecommendRequest {
            target_ledgers: Some(0),
            urgency: None,
            max_fee: None,
        };
        assert!(validate_recommend_request(&req).is_err());
    }

    #[test]
    fn target_ledgers_too_large_rejected() {
        let req = RecommendRequest {
            target_ledgers: Some(101),
            urgency: None,
            max_fee: None,
        };
        assert!(validate_recommend_request(&req).is_err());
    }

    #[test]
    fn max_fee_too_large_rejected() {
        let req = RecommendRequest {
            target_ledgers: None,
            urgency: None,
            max_fee: Some("999999999".to_string()),
        };
        assert!(validate_recommend_request(&req).is_err());
    }

    #[test]
    fn max_fee_non_numeric_rejected() {
        let req = RecommendRequest {
            target_ledgers: None,
            urgency: None,
            max_fee: Some("not-a-number".to_string()),
        };
        assert!(validate_recommend_request(&req).is_err());
    }
}


use serde::{Deserialize, Serialize};

use super::trend::{analyze_trend, TrendDirection};

/// A single predicted fee point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPoint {
    /// Steps ahead from the last observation.
    pub step: usize,
    /// Predicted fee value.
    pub predicted_fee: f64,
    /// Lower bound of the 95 % confidence interval.
    pub lower_bound: f64,
    /// Upper bound of the 95 % confidence interval.
    pub upper_bound: f64,
}

/// Forecast result containing the trend used and predicted points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast {
    /// The direction detected in the input series.
    pub direction: TrendDirection,
    /// Predicted points for `horizon` steps ahead.
    pub predictions: Vec<ForecastPoint>,
}

/// Produce a simple extrapolation forecast for the given fee series.
///
/// Uses linear regression (from `trend::analyze_trend`) to project future fees
/// and widens the confidence band by ± 1.96 × std_dev × sqrt(step).
pub fn forecast(fees: &[f64], horizon: usize) -> Forecast {
    let trend = analyze_trend(fees);

    let n = fees.len() as f64;
    let intercept = trend.mean - trend.slope * (n - 1.0) / 2.0;

    let mut predictions = Vec::with_capacity(horizon);
    for step in 1..=horizon {
        let x = n + step as f64 - 1.0;
        let predicted = trend.slope * x + intercept;
        let spread = 1.96 * trend.std_dev * (step as f64).sqrt();
        predictions.push(ForecastPoint {
            step,
            predicted_fee: predicted,
            lower_bound: predicted - spread,
            upper_bound: predicted + spread,
        });
    }

    Forecast {
        direction: trend.direction,
        predictions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forecast_upward_series() {
        let fees: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 5.0).collect();
        let fc = forecast(&fees, 5);
        assert_eq!(fc.direction, TrendDirection::Upward);
        assert_eq!(fc.predictions.len(), 5);
        assert!(fc.predictions[0].predicted_fee > fees.last().copied().unwrap_or(0.0));
    }

    #[test]
    fn forecast_empty_series() {
        let fc = forecast(&[], 3);
        assert_eq!(fc.predictions.len(), 3);
        assert_eq!(fc.predictions[0].predicted_fee, 0.0);
    }
}

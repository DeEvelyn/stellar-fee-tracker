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

/// Confidence intervals for a single forecast point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// The point estimate.
    pub predicted: f64,
    /// Lower bound of the 80% confidence interval.
    pub lower_80: f64,
    /// Upper bound of the 80% confidence interval.
    pub upper_80: f64,
    /// Lower bound of the 95% confidence interval.
    pub lower_95: f64,
    /// Upper bound of the 95% confidence interval.
    pub upper_95: f64,
}

/// Fit a linear trend to `fees` and project `horizon` steps ahead.
///
/// Returns a `Vec<f64>` with `horizon` predicted fee values.
pub fn forecast_linear(fees: &[f64], horizon: usize) -> Vec<f64> {
    let trend = analyze_trend(fees);
    let n = fees.len() as f64;
    let intercept = trend.mean - trend.slope * (n - 1.0) / 2.0;
    (1..=horizon)
        .map(|step| trend.slope * (n + step as f64 - 1.0) + intercept)
        .collect()
}

/// Forecast using Holt's double exponential smoothing (trend-aware).
///
/// - `alpha`: level smoothing factor (0.0, 1.0)
/// - `beta`:  trend smoothing factor (0.0, 1.0)
///
/// Returns `horizon` predicted values.
pub fn forecast_holt(fees: &[f64], horizon: usize, alpha: f64, beta: f64) -> Vec<f64> {
    if fees.is_empty() {
        return vec![0.0; horizon];
    }
    if fees.len() == 1 {
        return vec![fees[0]; horizon];
    }
    // Initialise level and trend.
    let mut level = fees[0];
    let mut trend = fees[1] - fees[0];

    for i in 1..fees.len() {
        let prev_level = level;
        level = alpha * fees[i] + (1.0 - alpha) * (level + trend);
        trend = beta * (level - prev_level) + (1.0 - beta) * trend;
    }

    (1..=horizon)
        .map(|h| level + h as f64 * trend)
        .collect()
}

/// Compute 80% and 95% confidence intervals for a sequence of point forecasts
/// given the residual variance of the model.
///
/// Returns one [`ConfidenceInterval`] per forecast point.
pub fn confidence_intervals(forecast: &[f64], residual_variance: f64) -> Vec<ConfidenceInterval> {
    let std_dev = residual_variance.sqrt();
    forecast
        .iter()
        .enumerate()
        .map(|(i, &predicted)| {
            // Spread widens with horizon (square-root rule).
            let spread = std_dev * ((i + 1) as f64).sqrt();
            ConfidenceInterval {
                predicted,
                lower_80: predicted - 1.282 * spread,
                upper_80: predicted + 1.282 * spread,
                lower_95: predicted - 1.96 * spread,
                upper_95: predicted + 1.96 * spread,
            }
        })
        .collect()
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

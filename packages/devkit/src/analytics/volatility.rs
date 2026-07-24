use serde::{Deserialize, Serialize};

/// Volatility metrics for a fee series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityMetrics {
    /// Simple standard deviation of fee values.
    pub standard_deviation: f64,
    /// Coefficient of variation (std_dev / mean).
    pub coefficient_of_variation: f64,
    /// Average absolute period-over-period change.
    pub mean_abs_change: f64,
    /// Maximum observed fee.
    pub max: f64,
    /// Minimum observed fee.
    pub min: f64,
    /// Range (max - min).
    pub range: f64,
}

/// Compute volatility metrics for a slice of fee values.
pub fn compute_volatility(fees: &[f64]) -> VolatilityMetrics {
    let n = fees.len();
    if n == 0 {
        return VolatilityMetrics {
            standard_deviation: 0.0,
            coefficient_of_variation: 0.0,
            mean_abs_change: 0.0,
            max: 0.0,
            min: 0.0,
            range: 0.0,
        };
    }

    let mean = fees.iter().sum::<f64>() / n as f64;
    let variance = fees.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / n as f64;
    let standard_deviation = variance.sqrt();
    let coefficient_of_variation = if mean.abs() > f64::EPSILON {
        standard_deviation / mean
    } else {
        0.0
    };

    let mut abs_changes = Vec::with_capacity(n.saturating_sub(1));
    for window in fees.windows(2) {
        abs_changes.push((window[1] - window[0]).abs());
    }
    let mean_abs_change = if abs_changes.is_empty() {
        0.0
    } else {
        abs_changes.iter().sum::<f64>() / abs_changes.len() as f64
    };

    let max = fees.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = fees.iter().cloned().fold(f64::INFINITY, f64::min);
    let range = max - min;

    VolatilityMetrics {
        standard_deviation,
        coefficient_of_variation,
        mean_abs_change,
        max,
        min,
        range,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_series_zero_volatility() {
        let fees = vec![100.0; 10];
        let v = compute_volatility(&fees);
        assert!((v.standard_deviation - 0.0).abs() < f64::EPSILON);
        assert!((v.coefficient_of_variation - 0.0).abs() < f64::EPSILON);
        assert!((v.mean_abs_change - 0.0).abs() < f64::EPSILON);
        assert_eq!(v.max, 100.0);
        assert_eq!(v.min, 100.0);
    }

    #[test]
    fn increasing_series() {
        let fees: Vec<f64> = (0..5).map(|i| i as f64 * 10.0).collect();
        let v = compute_volatility(&fees);
        assert!(v.standard_deviation > 0.0);
        assert!((v.mean_abs_change - 10.0).abs() < f64::EPSILON);
        assert_eq!(v.max, 40.0);
        assert_eq!(v.min, 0.0);
    }

    #[test]
    fn empty_series() {
        let v = compute_volatility(&[]);
        assert_eq!(v.standard_deviation, 0.0);
        assert_eq!(v.max, 0.0);
        assert_eq!(v.min, 0.0);
    }
}

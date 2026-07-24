use serde::{Deserialize, Serialize};

/// Represents the direction of a fee trend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Fees are consistently increasing.
    Upward,
    /// Fees are consistently decreasing.
    Downward,
    /// Fees are oscillating around a stable mean.
    Sideways,
}

/// A trend analysis result for a fee series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Overall direction of the trend.
    pub direction: TrendDirection,
    /// Slope of the linear regression (fee change per unit time).
    pub slope: f64,
    /// R-squared goodness of fit [0.0, 1.0].
    pub r_squared: f64,
    /// Mean fee across the analysed window.
    pub mean: f64,
    /// Standard deviation of fees across the analysed window.
    pub std_dev: f64,
}

/// Compute a linear-regression trend over a slice of fee values.
pub fn analyze_trend(fees: &[f64]) -> TrendAnalysis {
    let n = fees.len() as f64;
    if n == 0.0 {
        return TrendAnalysis {
            direction: TrendDirection::Sideways,
            slope: 0.0,
            r_squared: 0.0,
            mean: 0.0,
            std_dev: 0.0,
        };
    }

    let mean = fees.iter().sum::<f64>() / n;
    let variance = fees.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();

    // Simple linear regression: y = slope * x + intercept
    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;

    let mean_x = (n - 1.0) / 2.0;
    for (i, &fee) in fees.iter().enumerate() {
        let x = i as f64;
        sum_xy += (x - mean_x) * (fee - mean);
        sum_x2 += (x - mean_x).powi(2);
    }

    let slope = if sum_x2 > f64::EPSILON {
        sum_xy / sum_x2
    } else {
        0.0
    };

    let intercept = mean - slope * mean_x;

    for (i, &fee) in fees.iter().enumerate() {
        let predicted = slope * i as f64 + intercept;
        ss_res += (fee - predicted).powi(2);
        ss_tot += (fee - mean).powi(2);
    }

    let r_squared = if ss_tot > f64::EPSILON {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };

    let direction = if slope > 1e-6 {
        TrendDirection::Upward
    } else if slope < -1e-6 {
        TrendDirection::Downward
    } else {
        TrendDirection::Sideways
    };

    TrendAnalysis {
        direction,
        slope,
        r_squared: r_squared.clamp(0.0, 1.0),
        mean,
        std_dev,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trend_upward() {
        let fees: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 10.0).collect();
        let result = analyze_trend(&fees);
        assert_eq!(result.direction, TrendDirection::Upward);
        assert!(result.slope > 0.0);
    }

    #[test]
    fn trend_downward() {
        let fees: Vec<f64> = (0..20).map(|i| 200.0 - i as f64 * 10.0).collect();
        let result = analyze_trend(&fees);
        assert_eq!(result.direction, TrendDirection::Downward);
        assert!(result.slope < 0.0);
    }

    #[test]
    fn trend_empty() {
        let result = analyze_trend(&[]);
        assert_eq!(result.direction, TrendDirection::Sideways);
        assert_eq!(result.slope, 0.0);
    }
}

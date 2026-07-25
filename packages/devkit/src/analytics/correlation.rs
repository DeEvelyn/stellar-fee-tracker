use serde::{Deserialize, Serialize};

/// Result of computing correlation between two fee series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationResult {
    /// Pearson correlation coefficient [-1.0, 1.0].
    pub pearson_r: f64,
    /// Coefficient of determination R^2.
    pub r_squared: f64,
    /// Number of paired observations used.
    pub sample_size: usize,
}

/// Compute Pearson correlation between two equally-sized slices.
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> CorrelationResult {
    let n = x.len().min(y.len());
    if n < 2 {
        return CorrelationResult {
            pearson_r: 0.0,
            r_squared: 0.0,
            sample_size: n,
        };
    }

    let mean_x: f64 = x.iter().take(n).sum::<f64>() / n as f64;
    let mean_y: f64 = y.iter().take(n).sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    let pearson_r = if denom > f64::EPSILON {
        (cov / denom).clamp(-1.0, 1.0)
    } else {
        0.0
    };

    CorrelationResult {
        pearson_r,
        r_squared: pearson_r * pearson_r,
        sample_size: n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_positive_correlation() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..10).map(|i| i as f64 * 2.0).collect();
        let r = pearson_correlation(&x, &y);
        assert!((r.pearson_r - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn perfect_negative_correlation() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..10).map(|i| 10.0 - i as f64).collect();
        let r = pearson_correlation(&x, &y);
        assert!((r.pearson_r - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn uncorrelated() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![3.0, 1.0, 4.0, 2.0];
        let r = pearson_correlation(&x, &y);
        assert!(r.pearson_r.abs() < 0.5);
    }
}

/// Compute the Pearson autocorrelation of a fee series at a given `lag`.
///
/// Returns the Pearson correlation coefficient between `fees` and `fees` shifted
/// by `lag` positions, in the range [-1.0, 1.0].
///
/// Returns 1.0 when `lag == 0` (a series is perfectly correlated with itself),
/// and 0.0 when `lag >= fees.len()`.
pub fn autocorrelation(fees: &[f64], lag: usize) -> f64 {
    let n = fees.len();
    if lag == 0 {
        return 1.0;
    }
    if lag >= n {
        return 0.0;
    }
    let x = &fees[..n - lag];
    let y = &fees[lag..];
    pearson_correlation(x, y).pearson_r
}

/// Measure the Pearson correlation between `fee_amount` and
/// `ledger_capacity_usage` across a paired window.
///
/// `fees` and `capacity` must be the same length; shorter slices are trimmed
/// to the minimum length. Returns a [`CorrelationResult`] using the standard
/// Pearson formula.
pub fn cross_correlation(fees: &[f64], capacity: &[f64]) -> CorrelationResult {
    pearson_correlation(fees, capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_positive_correlation() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..10).map(|i| i as f64 * 2.0).collect();
        let r = pearson_correlation(&x, &y);
        assert!((r.pearson_r - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn perfect_negative_correlation() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..10).map(|i| 10.0 - i as f64).collect();
        let r = pearson_correlation(&x, &y);
        assert!((r.pearson_r - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn uncorrelated() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![3.0, 1.0, 4.0, 2.0];
        let r = pearson_correlation(&x, &y);
        assert!(r.pearson_r.abs() < 0.5);
    }

    #[test]
    fn autocorrelation_lag_zero_is_one() {
        let fees: Vec<f64> = (0..20).map(|i| i as f64 * 5.0).collect();
        assert!((autocorrelation(&fees, 0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn autocorrelation_lag_too_large_is_zero() {
        let fees = vec![1.0, 2.0, 3.0];
        assert_eq!(autocorrelation(&fees, 10), 0.0);
    }
}

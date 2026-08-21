//! Fee regime change detector using the Kolmogorov–Smirnov (KS) statistic.
//!
//! Compares a short rolling window (1 h) against a long rolling window (24 h)
//! to identify when the underlying fee distribution has fundamentally shifted.
//! A KS statistic > 0.3 is treated as a regime change signal.

/// Returns `true` when the two-sample KS statistic between the 1-hour rolling
/// fee sample (`fees_1h`) and the 24-hour rolling fee sample (`fees_24h`)
/// exceeds the 0.3 threshold, indicating a regime change.
///
/// Returns `false` when either slice is empty.
///
/// # Algorithm
/// 1. Sort both samples independently.
/// 2. Build a merged evaluation grid of all unique fee values.
/// 3. Walk the grid computing the empirical CDF of each sample at every point.
/// 4. The KS statistic is the maximum absolute difference between the two CDFs.
pub fn detect_regime_change(fees_1h: &[f64], fees_24h: &[f64]) -> bool {
    if fees_1h.is_empty() || fees_24h.is_empty() {
        return false;
    }
    ks_statistic(fees_1h, fees_24h) > 0.3
}

/// Compute the two-sample KS statistic between two fee distributions.
pub fn ks_statistic(a: &[f64], b: &[f64]) -> f64 {
    let n_a = a.len() as f64;
    let n_b = b.len() as f64;

    let mut sorted_a = a.to_vec();
    let mut sorted_b = b.to_vec();
    sorted_a.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    sorted_b.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));

    // Build merged evaluation grid.
    let mut grid: Vec<f64> = sorted_a.iter().chain(sorted_b.iter()).cloned().collect();
    grid.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    grid.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);

    let mut max_diff = 0.0_f64;
    let mut i_a = 0usize;
    let mut i_b = 0usize;

    for &x in &grid {
        // Advance past all values <= x in each sorted sample.
        while i_a < sorted_a.len() && sorted_a[i_a] <= x {
            i_a += 1;
        }
        while i_b < sorted_b.len() && sorted_b[i_b] <= x {
            i_b += 1;
        }
        let cdf_a = i_a as f64 / n_a;
        let cdf_b = i_b as f64 / n_b;
        let diff = (cdf_a - cdf_b).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }

    max_diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_distributions_no_regime_change() {
        let fees: Vec<f64> = (0..100).map(|i| i as f64).collect();
        assert!(!detect_regime_change(&fees, &fees));
    }

    #[test]
    fn very_different_distributions_detected() {
        let fees_1h: Vec<f64> = (1000..1100).map(|i| i as f64).collect();
        let fees_24h: Vec<f64> = (0..100).map(|i| i as f64).collect();
        assert!(detect_regime_change(&fees_1h, &fees_24h));
    }

    #[test]
    fn empty_slices_return_false() {
        assert!(!detect_regime_change(&[], &[1.0, 2.0]));
        assert!(!detect_regime_change(&[1.0, 2.0], &[]));
    }

    #[test]
    fn ks_statistic_identical_is_zero() {
        let fees: Vec<f64> = (0..50).map(|i| i as f64).collect();
        assert!(ks_statistic(&fees, &fees) < f64::EPSILON);
    }
}

/// Summary statistics for a fee distribution.
#[derive(Debug, Clone)]
pub struct FeeDistributionSummary {
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub median: u64,
    pub std_dev: f64,
    /// Percentiles p10, p20, ..., p90, p99 (index 0 = p10, index 9 = p99).
    pub percentiles: [u64; 10],
}

/// Computes percentile statistics over fee samples.
pub struct Percentile;

impl Percentile {
    /// Returns the nearest-rank percentile of a sorted slice.
    /// `p` must be in 1..=100. Returns 0 for empty slices.
    pub fn nearest_rank(sorted: &[u64], p: usize) -> u64 {
        if sorted.is_empty() {
            return 0;
        }
        let idx = ((p as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
        sorted[idx.saturating_sub(1).min(sorted.len() - 1)]
    }

    /// Returns the linear-interpolation percentile of a sorted slice.
    /// `p` must be in 0..=100. Returns 0 for empty slices.
    pub fn linear_interpolation(sorted: &[u64], p: usize) -> u64 {
        if sorted.is_empty() {
            return 0;
        }
        if sorted.len() == 1 {
            return sorted[0];
        }
        let rank = (p as f64 / 100.0) * (sorted.len() - 1) as f64;
        let lo = rank.floor() as usize;
        let hi = rank.ceil() as usize;
        let frac = rank - lo as f64;
        (sorted[lo] as f64 + frac * (sorted[hi] as f64 - sorted[lo] as f64)).floor() as u64
    }

    /// Returns a full fee distribution summary for a sorted slice.
    /// Returns `None` for empty slices.
    pub fn fee_distribution_summary(sorted: &[u64]) -> Option<FeeDistributionSummary> {
        if sorted.is_empty() {
            return None;
        }
        let n = sorted.len();
        let min = sorted[0];
        let max = sorted[n - 1];
        let mean = sorted.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
        let median = Self::nearest_rank(sorted, 50);
        let variance = sorted
            .iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>()
            / n as f64;
        let std_dev = variance.sqrt();
        let ps = [10, 20, 30, 40, 50, 60, 70, 80, 90, 99];
        let percentiles = ps.map(|p| Self::nearest_rank(sorted, p));
        Some(FeeDistributionSummary {
            min,
            max,
            mean,
            median,
            std_dev,
            percentiles,
        })
    }
}

// ── Free functions (Issue #259, #260) ────────────────────────────────────────

/// Nearest-rank percentile of a pre-sorted slice.
/// Returns 0 for empty slices. `p` must be in 0..=100.
pub fn percentile_nearest(sorted: &[u64], p: u8) -> u64 {
    Percentile::nearest_rank(sorted, p as usize)
}

/// Linear-interpolation percentile of a pre-sorted slice.
/// Returns 0.0 for empty slices. `p` must be in 0..=100.
pub fn percentile_interpolated(sorted: &[u64], p: u8) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0] as f64;
    }
    let rank = (p as f64 / 100.0) * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    sorted[lo] as f64 + frac * (sorted[hi] as f64 - sorted[lo] as f64)
}

// ── P² streaming percentile estimator (Issue #261) ───────────────────────────

/// Streaming percentile estimator using the P² algorithm (Jain & Chlamtac, 1985).
///
/// Estimates a single percentile without storing all observations. Uses five
/// markers that track the distribution shape as values arrive.
///
/// # Example
///
/// ```
/// use stellar_devkit::analysis::percentile::PsquaredEstimator;
///
/// let mut est = PsquaredEstimator::new(0.50);
/// for v in 1..=1000 {
///     est.update(v);
/// }
/// let p50 = est.estimate();
/// assert!((400.0..=600.0).contains(&p50), "p50={p50} not near 500");
/// ```
#[derive(Debug, Clone)]
pub struct PsquaredEstimator {
    p: f64,
    count: usize,
    markers: [(f64, f64); 5],
    init_vals: [u64; 5],
    init_count: usize,
}

impl PsquaredEstimator {
    /// Create a new estimator for the given percentile.
    ///
    /// `p` must be in `(0.0, 1.0)`. Common values: `0.50` (p50), `0.90` (p90),
    /// `0.95` (p95), `0.99` (p99).
    ///
    /// # Panics
    ///
    /// Panics if `p` is not in `(0.0, 1.0)`.
    pub fn new(p: f64) -> Self {
        assert!(p > 0.0 && p < 1.0, "p must be in (0.0, 1.0), got {p}");
        Self {
            p,
            count: 0,
            markers: [(0.0, 0.0); 5],
            init_vals: [0; 5],
            init_count: 0,
        }
    }

    /// Feed a new observation into the estimator.
    pub fn update(&mut self, x: u64) {
        if self.count < 5 {
            self.init_vals[self.init_count] = x;
            self.init_count += 1;
            self.count += 1;
            if self.count == 5 {
                self.init_markers();
            }
            return;
        }

        self.count += 1;
        let x = x as f64;
        let n = self.count as f64;

        // Find cell k where the new value belongs
        let mut k = 0;
        for i in 0..4 {
            if x >= self.markers[i].1 {
                k = i + 1;
            }
        }

        // Update extreme markers if outside range
        if x < self.markers[0].1 {
            self.markers[0].1 = x;
        }
        if x > self.markers[4].1 {
            self.markers[4].1 = x;
        }

        // Increment marker positions from k+1 to 4 (paper: j = k+1..4)
        // When k=0 (x < h₀), we skip marker 0 so its n stays at 1.
        // When k=4 (x ≥ h₃), we only increment marker 4.
        let start = if k == 0 { 1 } else { k };
        for i in start..5 {
            self.markers[i].0 += 1.0;
        }

        // Update desired positions using the P² quantile mapping
        let n_prime = |i: usize| match i {
            0 => 1.0,
            1 => 1.0 + self.p * (n - 1.0) / 2.0,
            2 => 1.0 + self.p * (n - 1.0),
            3 => 1.0 + (1.0 + self.p) * (n - 1.0) / 2.0,
            4 => n,
            _ => unreachable!(),
        };

        // Adjust interior markers (i = 1,2,3)
        for i in 1..4 {
            let d = n_prime(i) - self.markers[i].0;
            let (n_im1, h_im1) = self.markers[i - 1];
            let (n_i, h_i) = self.markers[i];
            let (n_ip1, h_ip1) = self.markers[i + 1];

            let cond =
                (d >= 1.0 && (n_ip1 - n_i).abs() > 1.0) || (d <= -1.0 && (n_im1 - n_i).abs() > 1.0);
            if !cond {
                continue;
            }

            // Parabolic update for h_i  (canonical P² formula)
            let d_i = (n_i - n_im1 + d) * (h_ip1 - h_i) / (n_ip1 - n_i)
                + (n_ip1 - n_i - d) * (h_i - h_im1) / (n_i - n_im1);
            let h_prime = h_i + d / (n_ip1 - n_im1) * d_i;

            let new_h = if h_im1 < h_prime && h_prime < h_ip1 {
                h_prime
            } else {
                // Linear fallback: step proportionally within the containing cell
                if d >= 0.0 {
                    h_i + (h_ip1 - h_i) / (n_ip1 - n_i)
                } else {
                    h_i + (h_im1 - h_i) / (n_im1 - n_i)
                }
            };

            let new_n = n_i + if d >= 0.0 { 1.0 } else { -1.0 };
            self.markers[i] = (new_n, new_h);
        }
    }

    /// Return the current estimate for the configured percentile.
    pub fn estimate(&self) -> f64 {
        match self.count {
            0 => 0.0,
            1..=4 => {
                self.init_vals[..self.init_count].iter().sum::<u64>() as f64
                    / self.init_count as f64
            }
            _ => self.markers[2].1,
        }
    }

    /// Return the number of observations seen so far.
    pub fn count(&self) -> usize {
        self.count
    }

    fn init_markers(&mut self) {
        self.init_vals.sort_unstable();
        let p = self.p;
        self.markers = [
            (1.0, self.init_vals[0] as f64),
            (1.0 + 2.0 * p, self.init_vals[1] as f64),
            (1.0 + 4.0 * p, self.init_vals[2] as f64),
            (1.0 + 6.0 * p, self.init_vals[3] as f64),
            (5.0, self.init_vals[4] as f64),
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_basic() {
        let data = [10, 20, 30, 40, 50];
        assert_eq!(Percentile::nearest_rank(&data, 50), 30);
        assert_eq!(Percentile::nearest_rank(&data, 100), 50);
        assert_eq!(Percentile::nearest_rank(&data, 1), 10);
    }

    #[test]
    fn nearest_rank_empty() {
        assert_eq!(Percentile::nearest_rank(&[], 50), 0);
    }

    #[test]
    fn linear_interpolation_basic() {
        let data = [10, 20, 30, 40, 50];
        assert_eq!(Percentile::linear_interpolation(&data, 0), 10);
        assert_eq!(Percentile::linear_interpolation(&data, 100), 50);
        assert_eq!(Percentile::linear_interpolation(&data, 50), 30);
    }

    #[test]
    fn linear_interpolation_empty() {
        assert_eq!(Percentile::linear_interpolation(&[], 50), 0);
    }

    #[test]
    fn fee_distribution_summary_basic() {
        let data = [10u64, 20, 30, 40, 50];
        let s = Percentile::fee_distribution_summary(&data).unwrap();
        assert_eq!(s.min, 10);
        assert_eq!(s.max, 50);
        assert_eq!(s.mean, 30.0);
        assert_eq!(s.median, 30);
        assert_eq!(s.percentiles[9], 50); // p99
    }

    #[test]
    fn fee_distribution_summary_empty() {
        assert!(Percentile::fee_distribution_summary(&[]).is_none());
    }

    // ── Free function tests (Issue #259, #260) ───────────────────────────────

    #[test]
    fn percentile_nearest_free_function() {
        let data = [10, 20, 30, 40, 50];
        assert_eq!(percentile_nearest(&data, 50), 30);
        assert_eq!(percentile_nearest(&data, 100), 50);
        assert_eq!(percentile_nearest(&data, 1), 10);
    }

    #[test]
    fn percentile_nearest_empty() {
        assert_eq!(percentile_nearest(&[], 50), 0);
    }

    #[test]
    fn percentile_interpolated_free_function() {
        let data = [10, 20, 30, 40, 50];
        assert!((percentile_interpolated(&data, 50) - 30.0).abs() < 1e-9);
        assert!((percentile_interpolated(&data, 0) - 10.0).abs() < 1e-9);
        assert!((percentile_interpolated(&data, 100) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn percentile_interpolated_empty() {
        assert!((percentile_interpolated(&[], 50) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn percentile_interpolated_two_elements_midpoint() {
        let data = [10, 20];
        assert!((percentile_interpolated(&data, 50) - 15.0).abs() < 1e-9);
    }

    // ── P² estimator tests (Issue #261) ──────────────────────────────────────

    #[test]
    fn psquared_estimate_before_5_observations_returns_mean() {
        let mut est = PsquaredEstimator::new(0.95);
        est.update(10);
        est.update(20);
        assert!((est.estimate() - 15.0).abs() < 1e-9);
        assert_eq!(est.count(), 2);
    }

    #[test]
    fn psquared_empty_estimate_is_zero() {
        let est = PsquaredEstimator::new(0.95);
        assert!((est.estimate() - 0.0).abs() < 1e-9);
        assert_eq!(est.count(), 0);
    }

    #[test]
    fn psquared_converges_for_uniform_data() {
        let mut est = PsquaredEstimator::new(0.50);
        for v in 1..=1000 {
            est.update(v);
        }
        let p50 = est.estimate();
        assert!((400.0..=600.0).contains(&p50), "p50={p50} not near 500");
    }

    #[test]
    fn psquared_p95_on_simple_sequence() {
        let mut est = PsquaredEstimator::new(0.95);
        for v in 1..=200 {
            est.update(v);
        }
        let p95 = est.estimate();
        assert!(
            (175.0..=200.0).contains(&p95),
            "p95={p95} not in [175, 200]"
        );
    }

    #[test]
    fn psquared_p99_on_simple_sequence() {
        let mut est = PsquaredEstimator::new(0.99);
        for v in 1..=500 {
            est.update(v);
        }
        let p99 = est.estimate();
        assert!((480.0..=505.0).contains(&p99), "p99={p99} not near 500");
    }
}

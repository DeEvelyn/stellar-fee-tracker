/// Nearest-rank percentile of a pre-sorted slice.
/// Returns 0 for empty slices. `p` must be in 0..=100.
pub fn percentile_nearest(sorted: &[u64], p: u8) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((p as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[idx.saturating_sub(1).min(sorted.len() - 1)]
}

/// Compute percentile shift between two sorted fee slices.
/// Returns None if either slice is empty.
pub fn percentile_shift(short_sorted: &[u64], long_sorted: &[u64], p: u8) -> Option<f64> {
    if short_sorted.is_empty() || long_sorted.is_empty() {
        return None;
    }
    let short_p = percentile_nearest(short_sorted, p) as f64;
    let long_p = percentile_nearest(long_sorted, p) as f64;
    if long_p <= f64::EPSILON {
        return None;
    }
    Some(((short_p - long_p) / long_p) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_nearest_basic() {
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
    fn percentile_shift_basic() {
        let short = vec![100, 200, 300, 400, 500];
        let long = vec![50, 100, 150, 200, 250];
        let shift = percentile_shift(&short, &long, 50).unwrap();
        assert!(shift > 0.0, "short p50 should be higher than long p50");
    }

    #[test]
    fn percentile_shift_empty_returns_none() {
        assert!(percentile_shift(&[], &[100], 50).is_none());
        assert!(percentile_shift(&[100], &[], 50).is_none());
    }

    #[test]
    fn percentile_shift_identical_returns_zero() {
        let data = vec![100, 200, 300];
        let shift = percentile_shift(&data.clone(), &data, 50).unwrap();
        assert!((shift - 0.0).abs() < 1e-9);
    }
}

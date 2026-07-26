//! Fee comparator for evaluating a fee against a set of reference fees.
//!
//! Computes the percentage difference between an input fee and the mean of
//! the reference set, returning a [`ComparisonResult`] that classifies the
//! fee as below, within, or above the reference band.

/// The outcome of comparing an input fee against the reference set.
#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonResult {
    /// Fee is below the reference mean by the given percentage.
    Below(f64),
    /// Fee is within ±5 % of the reference mean.
    Within(f64),
    /// Fee is above the reference mean by the given percentage.
    Above(f64),
}

/// Compares input fees against a pre-configured set of reference fees.
#[derive(Debug, Clone)]
pub struct FeeComparator {
    reference_fees: Vec<u64>,
}

impl FeeComparator {
    /// Create a comparator backed by the given reference fees.
    ///
    /// # Panics
    /// Panics if `reference_fees` is empty.
    pub fn new(reference_fees: Vec<u64>) -> Self {
        assert!(!reference_fees.is_empty(), "reference_fees must not be empty");
        Self { reference_fees }
    }

    /// Compare `input_fee` against the mean of the reference fees.
    ///
    /// Returns [`ComparisonResult::Within`] if the percentage difference is
    /// within ±5 %, otherwise [`ComparisonResult::Below`] or
    /// [`ComparisonResult::Above`].
    pub fn compare(&self, input_fee: u64) -> ComparisonResult {
        let mean = self.reference_fees.iter().sum::<u64>() as f64
            / self.reference_fees.len() as f64;
        if mean == 0.0 {
            return ComparisonResult::Within(0.0);
        }
        let pct = ((input_fee as f64 - mean) / mean) * 100.0;
        if pct < -5.0 {
            ComparisonResult::Below(pct)
        } else if pct > 5.0 {
            ComparisonResult::Above(pct)
        } else {
            ComparisonResult::Within(pct)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comparator() -> FeeComparator {
        FeeComparator::new(vec![100, 200, 300])
    }

    #[test]
    fn below_reference() {
        let r = comparator().compare(50);
        assert!(matches!(r, ComparisonResult::Below(_)));
    }

    #[test]
    fn within_reference() {
        let r = comparator().compare(200);
        assert!(matches!(r, ComparisonResult::Within(_)));
    }

    #[test]
    fn above_reference() {
        let r = comparator().compare(500);
        assert!(matches!(r, ComparisonResult::Above(_)));
    }
}

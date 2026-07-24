#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PercentileTable {
    pub p10: u64,
    pub p20: u64,
    pub p30: u64,
    pub p40: u64,
    pub p50: u64,
    pub p60: u64,
    pub p70: u64,
    pub p80: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
}

impl PercentileTable {
    /// Build a `PercentileTable` from a slice of fee values.
    /// The input slice is sorted internally; it does not need to be pre-sorted.
    /// Returns `None` for empty slices.
    pub fn build(fees: &[u64]) -> Option<Self> {
        if fees.is_empty() {
            return None;
        }

        let mut sorted = fees.to_vec();
        sorted.sort_unstable();

        let percentile = |p: usize| -> u64 {
            if sorted.is_empty() {
                return 0;
            }
            let idx = ((p as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
            sorted[idx.saturating_sub(1).min(sorted.len() - 1)]
        };

        Some(Self {
            p10: percentile(10),
            p20: percentile(20),
            p30: percentile(30),
            p40: percentile(40),
            p50: percentile(50),
            p60: percentile(60),
            p70: percentile(70),
            p80: percentile(80),
            p90: percentile(90),
            p95: percentile(95),
            p99: percentile(99),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_basic() {
        let fees: Vec<u64> = (1..=100).collect();
        let table = PercentileTable::build(&fees).unwrap();
        assert_eq!(table.p10, 10);
        assert_eq!(table.p50, 50);
        assert_eq!(table.p90, 90);
        assert_eq!(table.p95, 95);
        assert_eq!(table.p99, 99);
    }

    #[test]
    fn build_single_element() {
        let table = PercentileTable::build(&[42]).unwrap();
        assert_eq!(table.p10, 42);
        assert_eq!(table.p50, 42);
        assert_eq!(table.p99, 42);
    }

    #[test]
    fn build_empty_returns_none() {
        assert!(PercentileTable::build(&[]).is_none());
    }

    #[test]
    fn build_unsorted_input() {
        let fees = vec![50, 10, 90, 30, 70, 20, 80, 40, 60];
        let table = PercentileTable::build(&fees).unwrap();
        assert_eq!(table.p10, 10);
        assert_eq!(table.p50, 50);
        assert_eq!(table.p90, 90);
        assert_eq!(table.p99, 90);
    }

    #[test]
    fn build_two_elements() {
        let table = PercentileTable::build(&[10, 20]).unwrap();
        assert_eq!(table.p10, 10);
        assert_eq!(table.p50, 10);
        assert_eq!(table.p99, 20);
    }
}

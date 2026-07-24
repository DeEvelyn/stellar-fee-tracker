//! Percentile table builder for fee data.

/// A complete percentile breakdown of a fee distribution.
#[derive(Debug, Clone, PartialEq)]
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
    /// Build a [`PercentileTable`] from a slice of fee values.
    /// Returns `None` when `fees` is empty.
    pub fn from_fees(fees: &[u64]) -> Option<Self> {
        if fees.is_empty() {
            return None;
        }
        let mut sorted = fees.to_vec();
        sorted.sort_unstable();
        Some(Self {
            p10: percentile(&sorted, 10),
            p20: percentile(&sorted, 20),
            p30: percentile(&sorted, 30),
            p40: percentile(&sorted, 40),
            p50: percentile(&sorted, 50),
            p60: percentile(&sorted, 60),
            p70: percentile(&sorted, 70),
            p80: percentile(&sorted, 80),
            p90: percentile(&sorted, 90),
            p95: percentile(&sorted, 95),
            p99: percentile(&sorted, 99),
        })
    }

    /// Render the table as a human-readable string.
    pub fn display(&self) -> String {
        format!(
            "Percentile Table\n\
             ================\n\
             p10:  {} stroops\n\
             p20:  {} stroops\n\
             p30:  {} stroops\n\
             p40:  {} stroops\n\
             p50:  {} stroops\n\
             p60:  {} stroops\n\
             p70:  {} stroops\n\
             p80:  {} stroops\n\
             p90:  {} stroops\n\
             p95:  {} stroops\n\
             p99:  {} stroops",
            self.p10, self.p20, self.p30, self.p40, self.p50,
            self.p60, self.p70, self.p80, self.p90, self.p95, self.p99,
        )
    }

    /// Render the table as a JSON object.
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"p10":{},"p20":{},"p30":{},"p40":{},"p50":{},"p60":{},"p70":{},"p80":{},"p90":{},"p95":{},"p99":{}}}"#,
            self.p10, self.p20, self.p30, self.p40, self.p50,
            self.p60, self.p70, self.p80, self.p90, self.p95, self.p99,
        )
    }
}

/// Nearest-rank percentile (0-indexed, clamped).
fn percentile(sorted: &[u64], p: u8) -> u64 {
    let n = sorted.len();
    let idx = ((p as f64 / 100.0 * n as f64).ceil() as usize).saturating_sub(1);
    sorted[idx.min(n - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fees() -> Vec<u64> {
        (1u64..=100).map(|i| i * 10).collect()
    }

    #[test]
    fn from_fees_returns_none_on_empty() {
        assert!(PercentileTable::from_fees(&[]).is_none());
    }

    #[test]
    fn p50_is_median() {
        let t = PercentileTable::from_fees(&sample_fees()).unwrap();
        assert_eq!(t.p50, 500);
    }

    #[test]
    fn single_value_all_equal() {
        let t = PercentileTable::from_fees(&[42]).unwrap();
        assert_eq!(t.p10, 42);
        assert_eq!(t.p99, 42);
    }

    #[test]
    fn display_contains_p50() {
        let t = PercentileTable::from_fees(&[100, 200, 300]).unwrap();
        assert!(t.display().contains("p50"));
    }

    #[test]
    fn to_json_contains_all_keys() {
        let t = PercentileTable::from_fees(&[100]).unwrap();
        let json = t.to_json();
        for key in ["p10","p20","p30","p40","p50","p60","p70","p80","p90","p95","p99"] {
            assert!(json.contains(key));
        }
    }
}

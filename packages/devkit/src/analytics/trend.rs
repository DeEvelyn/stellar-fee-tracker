/// Direction of a fee trend over a time window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Up,
    Down,
    Sideways,
}

impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrendDirection::Up => write!(f, "up"),
            TrendDirection::Down => write!(f, "down"),
            TrendDirection::Sideways => write!(f, "sideways"),
        }
    }
}

/// Determine whether fees are trending up, down, or sideways over a window.
///
/// Compares the mean of the first third vs the last third of the window:
/// - **Up**: `last_mean > first_mean * 1.05`
/// - **Down**: `last_mean < first_mean * 0.95`
/// - **Sideways**: within the 5% band
pub fn detect_trend(fees: &[f64]) -> TrendDirection {
    if fees.len() < 6 {
        return TrendDirection::Sideways;
    }

    let third = fees.len() / 3;
    let first_mean = mean(&fees[..third]);
    let last_mean = mean(&fees[fees.len() - third..]);

    if first_mean == 0.0 && last_mean == 0.0 {
        return TrendDirection::Sideways;
    }

    if last_mean > first_mean * 1.05 {
        TrendDirection::Up
    } else if last_mean < first_mean * 0.95 {
        TrendDirection::Down
    } else {
        TrendDirection::Sideways
    }
}

fn mean(slice: &[f64]) -> f64 {
    if slice.is_empty() {
        return 0.0;
    }
    slice.iter().sum::<f64>() / slice.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upward_trend() {
        let fees: Vec<f64> = (0..30).map(|i| 100.0 + i as f64 * 5.0).collect();
        assert_eq!(detect_trend(&fees), TrendDirection::Up);
    }

    #[test]
    fn downward_trend() {
        let fees: Vec<f64> = (0..30).map(|i| 300.0 - i as f64 * 5.0).collect();
        assert_eq!(detect_trend(&fees), TrendDirection::Down);
    }

    #[test]
    fn sideways_trend() {
        let fees: Vec<f64> = (0..30).map(|_| 100.0).collect();
        assert_eq!(detect_trend(&fees), TrendDirection::Sideways);
    }

    #[test]
    fn too_few_samples() {
        assert_eq!(detect_trend(&[1.0, 2.0, 3.0]), TrendDirection::Sideways);
        assert_eq!(detect_trend(&[]), TrendDirection::Sideways);
    }

    #[test]
    fn small_increase_is_sideways() {
        let fees: Vec<f64> = (0..30)
            .map(|i| 100.0 + (i as f64 * 0.1))
            .collect();
        assert_eq!(detect_trend(&fees), TrendDirection::Sideways);
    }

    #[test]
    fn large_increase_is_up() {
        let mut fees: Vec<f64> = (0..30).map(|_| 100.0).collect();
        for i in 20..30 {
            fees[i] = 200.0;
        }
        assert_eq!(detect_trend(&fees), TrendDirection::Up);
    }

    #[test]
    fn trend_display() {
        assert_eq!(TrendDirection::Up.to_string(), "up");
        assert_eq!(TrendDirection::Down.to_string(), "down");
        assert_eq!(TrendDirection::Sideways.to_string(), "sideways");
    }
}

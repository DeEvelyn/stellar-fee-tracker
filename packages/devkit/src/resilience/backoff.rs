use rand::Rng;

/// Strategy for computing delays between retry attempts.
#[derive(Debug, Clone, Copy)]
pub enum BackoffStrategy {
    /// `base_ms * 2^attempt`, capped at `max_ms`.
    Exponential {
        base_ms: u64,
        max_ms: u64,
        jitter_pct: f64,
    },
    /// `base_ms * attempt`, capped at `max_ms`.
    Linear { base_ms: u64, max_ms: u64 },
    /// Fixed delay regardless of attempt count.
    Fixed { delay_ms: u64 },
}

/// Compute the delay in milliseconds for a given attempt (0-indexed).
pub fn compute_delay(strategy: &BackoffStrategy, attempt: u32) -> u64 {
    match strategy {
        BackoffStrategy::Exponential {
            base_ms,
            max_ms,
            jitter_pct,
        } => {
            let exp = attempt as u32;
            let raw = base_ms.saturating_mul(1u64.checked_shl(exp).unwrap_or(u64::MAX));
            let capped = raw.min(*max_ms);
            if *jitter_pct > 0.0 {
                let jitter_range = (capped as f64 * jitter_pct / 100.0) as u64;
                if jitter_range > 0 {
                    let jitter = rand::thread_rng().gen_range(0..=jitter_range);
                    capped.saturating_add(jitter)
                } else {
                    capped
                }
            } else {
                capped
            }
        }
        BackoffStrategy::Linear { base_ms, max_ms } => {
            let raw = base_ms.saturating_mul(attempt as u64);
            raw.min(*max_ms)
        }
        BackoffStrategy::Fixed { delay_ms } => *delay_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_delay_attempts() {
        let strategy = BackoffStrategy::Exponential {
            base_ms: 100,
            max_ms: 5000,
            jitter_pct: 0.0,
        };
        assert_eq!(compute_delay(&strategy, 0), 100);
        assert_eq!(compute_delay(&strategy, 1), 200);
        assert_eq!(compute_delay(&strategy, 2), 400);
        assert_eq!(compute_delay(&strategy, 3), 800);
    }

    #[test]
    fn exponential_cap() {
        let strategy = BackoffStrategy::Exponential {
            base_ms: 1000,
            max_ms: 3000,
            jitter_pct: 0.0,
        };
        assert_eq!(compute_delay(&strategy, 0), 1000);
        assert_eq!(compute_delay(&strategy, 1), 2000);
        assert_eq!(compute_delay(&strategy, 2), 3000);
        assert_eq!(compute_delay(&strategy, 3), 3000);
    }

    #[test]
    fn linear_delay() {
        let strategy = BackoffStrategy::Linear {
            base_ms: 100,
            max_ms: 1000,
        };
        assert_eq!(compute_delay(&strategy, 0), 0);
        assert_eq!(compute_delay(&strategy, 1), 100);
        assert_eq!(compute_delay(&strategy, 2), 200);
        assert_eq!(compute_delay(&strategy, 3), 300);
    }

    #[test]
    fn linear_cap() {
        let strategy = BackoffStrategy::Linear {
            base_ms: 500,
            max_ms: 1200,
        };
        assert_eq!(compute_delay(&strategy, 0), 0);
        assert_eq!(compute_delay(&strategy, 1), 500);
        assert_eq!(compute_delay(&strategy, 2), 1000);
        assert_eq!(compute_delay(&strategy, 3), 1200);
    }

    #[test]
    fn fixed_delay() {
        let strategy = BackoffStrategy::Fixed { delay_ms: 250 };
        assert_eq!(compute_delay(&strategy, 0), 250);
        assert_eq!(compute_delay(&strategy, 5), 250);
    }
}

use std::time::Duration;

use rand::Rng;

/// Compute the next retry delay using exponential back-off.
///
/// `base_ms`  – base delay in milliseconds.
/// `max_ms`   – upper cap in milliseconds.
/// `jitter`   – when `true`, adds a random offset of up to 20 % of the computed delay.
pub fn exponential_backoff(attempt: u32, base_ms: u64, max_ms: u64, jitter: bool) -> Duration {
    let exp = attempt.min(63);
    let delay = base_ms.saturating_mul(1u64 << exp).min(max_ms);

    let delay = if jitter {
        let jitter_range = delay as f64 * 0.2;
        let jitter_value = rand::thread_rng().gen_range(0.0..=jitter_range);
        (delay as f64 + jitter_value) as u64
    } else {
        delay
    };

    Duration::from_millis(delay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_jitter_doubling() {
        let d = exponential_backoff(0, 100, 5000, false);
        assert_eq!(d, Duration::from_millis(100));

        let d = exponential_backoff(1, 100, 5000, false);
        assert_eq!(d, Duration::from_millis(200));

        let d = exponential_backoff(2, 100, 5000, false);
        assert_eq!(d, Duration::from_millis(400));
    }

    #[test]
    fn max_cap() {
        let d = exponential_backoff(20, 100, 5000, false);
        assert_eq!(d, Duration::from_millis(5000));
    }

    #[test]
    fn jitter_increases_delay() {
        let d = exponential_backoff(3, 100, 10000, true);
        assert!(d >= Duration::from_millis(800));
        assert!(d <= Duration::from_millis(960));
    }

    #[test]
    fn attempt_zero() {
        let d = exponential_backoff(0, 500, 10000, false);
        assert_eq!(d, Duration::from_millis(500));
    }
}

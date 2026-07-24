use std::time::Duration;

use rand::Rng;

/// Compute an exponential backoff delay for the given attempt.
///
/// Formula: `min(base_ms * 2^attempt, max_ms)` with optional random jitter up to 20%.
pub fn exponential_backoff(attempt: u32, base_ms: u64, max_ms: u64, jitter: bool) -> Duration {
    let shift = attempt.min(63);
    let delay = base_ms.saturating_mul(1u64 << shift).min(max_ms);
    let delay = apply_jitter(delay, jitter);
    Duration::from_millis(delay)
}

/// Compute a linear backoff delay for the given attempt.
///
/// Formula: `min(base_ms * attempt, max_ms)`. When `attempt == 0`, returns `base_ms`.
pub fn linear_backoff(attempt: u32, base_ms: u64, max_ms: u64) -> Duration {
    let delay = if attempt == 0 {
        base_ms
    } else {
        base_ms.saturating_mul(attempt as u64).min(max_ms)
    };
    Duration::from_millis(delay)
}

fn apply_jitter(delay_ms: u64, jitter: bool) -> u64 {
    if !jitter || delay_ms == 0 {
        return delay_ms;
    }
    let max_extra = (delay_ms as f64 * 0.20).ceil() as u64;
    if max_extra == 0 {
        return delay_ms;
    }
    let extra = rand::thread_rng().gen_range(0..=max_extra);
    delay_ms.saturating_add(extra)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_basic() {
        let d = exponential_backoff(0, 100, 10_000, false);
        assert_eq!(d.as_millis(), 100);
        let d = exponential_backoff(1, 100, 10_000, false);
        assert_eq!(d.as_millis(), 200);
        let d = exponential_backoff(2, 100, 10_000, false);
        assert_eq!(d.as_millis(), 400);
    }

    #[test]
    fn exponential_caps_at_max() {
        let d = exponential_backoff(20, 100, 5_000, false);
        assert_eq!(d.as_millis(), 5_000);
    }

    #[test]
    fn exponential_jitter_stays_within_20_percent() {
        for _ in 0..200 {
            let d = exponential_backoff(3, 100, 10_000, true);
            let ms = d.as_millis() as u64;
            assert!(ms >= 800, "below floor: {ms}");
            assert!(ms <= 960, "above ceiling: {ms}");
        }
    }

    #[test]
    fn linear_basic() {
        let d = linear_backoff(0, 100, 10_000);
        assert_eq!(d.as_millis(), 100);
        let d = linear_backoff(1, 100, 10_000);
        assert_eq!(d.as_millis(), 100);
        let d = linear_backoff(2, 100, 10_000);
        assert_eq!(d.as_millis(), 200);
        let d = linear_backoff(5, 100, 10_000);
        assert_eq!(d.as_millis(), 500);
    }

    #[test]
    fn linear_caps_at_max() {
        let d = linear_backoff(200, 100, 5_000);
        assert_eq!(d.as_millis(), 5_000);
    }
}

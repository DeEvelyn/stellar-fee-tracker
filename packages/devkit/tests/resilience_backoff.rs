use stellar_devkit::resilience::backoff::{BackoffStrategy, compute_delay};

#[test]
fn exponential_correct_delay_per_attempt() {
    let strategy = BackoffStrategy::Exponential {
        base_ms: 100,
        max_ms: 10_000,
        jitter_pct: 0.0,
    };
    assert_eq!(compute_delay(&strategy, 0), 100);
    assert_eq!(compute_delay(&strategy, 1), 200);
    assert_eq!(compute_delay(&strategy, 2), 400);
    assert_eq!(compute_delay(&strategy, 3), 800);
    assert_eq!(compute_delay(&strategy, 4), 1_600);
    assert_eq!(compute_delay(&strategy, 5), 3_200);
}

#[test]
fn exponential_caps_at_max_ms() {
    let strategy = BackoffStrategy::Exponential {
        base_ms: 500,
        max_ms: 2_000,
        jitter_pct: 0.0,
    };
    assert_eq!(compute_delay(&strategy, 0), 500);
    assert_eq!(compute_delay(&strategy, 1), 1_000);
    assert_eq!(compute_delay(&strategy, 2), 2_000);
    assert_eq!(compute_delay(&strategy, 3), 2_000);
    assert_eq!(compute_delay(&strategy, 10), 2_000);
}

#[test]
fn exponential_jitter_within_20_percent() {
    let strategy = BackoffStrategy::Exponential {
        base_ms: 100,
        max_ms: 10_000,
        jitter_pct: 20.0,
    };
    for attempt in 0..10 {
        let base = compute_delay(&BackoffStrategy::Exponential {
            base_ms: 100,
            max_ms: 10_000,
            jitter_pct: 0.0,
        }, attempt);
        let with_jitter = compute_delay(&strategy, attempt);
        let max_jitter = (base as f64 * 0.20) as u64;
        assert!(
            with_jitter >= base && with_jitter <= base + max_jitter,
            "attempt={attempt} base={base} jittered={with_jitter} max_allowed={}",
            base + max_jitter
        );
    }
}

use criterion::{criterion_group, criterion_main, Criterion};

use stellar_devkit::resilience::backoff::BackoffStrategy;
use stellar_devkit::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use stellar_devkit::resilience::retry::{retry, RetryConfig};

fn bench_circuit_breaker_state_check(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("circuit_breaker_allow_request_closed", |b| {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        b.iter(|| rt.block_on(cb.allow_request()));
    });

    c.bench_function("circuit_breaker_state_read", |b| {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        b.iter(|| rt.block_on(cb.state()));
    });
}

/// Benchmark the retry wrapper overhead against a no-op closure.
///
/// Measures the cost of one successful attempt with max_attempts=1 vs calling
/// the same async no-op directly. Target: <1µs overhead per invocation.
fn bench_retry_executor_overhead(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("retry_noop_1_attempt", |b| {
        let config = RetryConfig {
            max_attempts: 1,
            backoff: BackoffStrategy::Fixed { delay_ms: 0 },
        };
        b.iter(|| rt.block_on(retry(config.clone(), || async { Ok::<u64, &str>(42) })));
    });

    c.bench_function("raw_noop_closure", |b| {
        b.iter(|| rt.block_on(async { Ok::<u64, &str>(42) }));
    });
}

criterion_group!(
    benches,
    bench_circuit_breaker_state_check,
    bench_retry_executor_overhead
);
criterion_main!(benches);

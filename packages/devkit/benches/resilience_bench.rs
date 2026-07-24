use criterion::{criterion_group, criterion_main, Criterion};

use stellar_devkit::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

fn bench_circuit_breaker_state_check(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("circuit_breaker_allow_request_closed", |b| {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        b.iter(|| {
            rt.block_on(cb.allow_request())
        });
    });

    c.bench_function("circuit_breaker_state_read", |b| {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        b.iter(|| {
            rt.block_on(cb.state())
        });
    });
}

criterion_group!(benches, bench_circuit_breaker_state_check);
criterion_main!(benches);

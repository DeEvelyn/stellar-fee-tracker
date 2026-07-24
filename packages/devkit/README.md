# stellar-devkit

Developer toolkit for the Stellar Fee Tracker. Provides utilities for testing, mocking, and simulating Stellar network behaviour without hitting live infrastructure.

## Scope

`stellar-devkit` is a standalone testing and simulation package. It must not import from `stellar-core` or any live-network crate. All functionality is self-contained and intended for use in `[dev-dependencies]` only.

## Boundary Rules

- No imports from `packages/core`
- No live Horizon API calls
- No database connections
- All external I/O must be injectable or mockable

## Modules

| Module | Description |
|---|---|
| `harness` | Mock Horizon server and pre-built fee scenario fixtures |
| `harness::scenarios` | JSON scenario files and runtime loader |
| `simulation` | Fee models, network-load generators, congestion predictors |
| `analysis` | Percentile stats, spike classification, rolling window |
| `cli` | Replay, export, and benchmark CLI stubs |
| `types` | Shared types: `FeeRecord`, `Scenario`, `SimResult` |
| `error` | `DevkitError` unified error enum |

## Simulation

The `simulation` module provides fee modelling and network-load generation without any live-network dependencies.

### `FeeModelConfig` fields

| Field | Type | Description |
|---|---|---|
| `base_fee` | `u64` | Minimum fee (stroops) used as the simulation floor |
| `surge_multiplier` | `f64` | Fee multiplier applied when the network is congested |
| `congestion_threshold` | `f64` | Load ratio (0.0–1.0) above which surge pricing activates |

### Example usage

```rust
use stellar_devkit::simulation::{FeeModel, NetworkLoad};

let load = NetworkLoad::constant(0.85);          // 85 % utilisation
let result = FeeModel::run(&load, base_fee: 100, surge_multiplier: 2.0, congestion_threshold: 0.8);
println!("recommended fee: {} stroops", result.recommended_fee);
```

### Output format (`SimResult`)

| Field | Type | Description |
|---|---|---|
| `recommended_fee` | `u64` | Suggested fee for the simulated conditions |
| `congested` | `bool` | Whether surge pricing was triggered |
| `load_ratio` | `f64` | Network utilisation at simulation time |

## Running

```bash
# Run all devkit tests
cargo test -p stellar-devkit

# Run a specific test file
cargo test -p stellar-devkit --test harness_congested
```

## Mock Horizon Server

The harness exposes canned `GET /fee_stats` payloads through `HorizonMock` and the JSON fixtures in `src/harness/scenarios/`.

```bash
# Start with the baseline fixture
cargo test -p stellar-devkit --test harness_normal -- --nocapture

# Swap to a higher-pressure fixture
cargo test -p stellar-devkit --test harness_congested -- --nocapture
```

Scenario flags map directly to the fixture you load in your test setup:

- `normal` for a low-fee baseline
- `congested` for sustained high-fee demand
- `spike` for a sudden short-lived fee jump
- `recovery` for a return from congestion toward baseline

```rust
use std::path::Path;

use stellar_devkit::harness::{
    horizon_mock::HorizonMock,
    scenarios::load_from_file,
};

let payload = load_from_file(Path::new("src/harness/scenarios/spike.json"))?;
let mock = HorizonMock::new(payload);
assert!(mock.fee_stats_payload().contains("\"scenario\": \"spike\""));
```

## Resilience

The `resilience` module provides retry policies, backoff strategies, and related utilities for building fault-tolerant Stellar integrations.

### Retry Executor

Execute an async closure with automatic retries on failure. Configure `max_attempts`, a `BackoffStrategy`, and let the executor handle the rest.

```rust
use stellar_devkit::resilience::{retry, RetryConfig, BackoffStrategy};

let config = RetryConfig {
    max_attempts: 5,
    backoff: BackoffStrategy::Exponential {
        base_ms: 200,
        max_ms: 10_000,
        jitter_pct: 10.0,
    },
};

let result = retry(config, || async {
    fetch_fee_stats_from_horizon().await
}).await;
```

### Backoff Strategies

| Strategy | Formula | Example (base=100) |
|---|---|---|
| `Exponential` | `base_ms * 2^attempt`, capped at `max_ms` | 100, 200, 400, 800, … |
| `Linear` | `base_ms * attempt`, capped at `max_ms` | 0, 100, 200, 300, … |
| `Fixed` | `delay_ms` every attempt | 100, 100, 100, … |

```rust
use stellar_devkit::resilience::{compute_delay, BackoffStrategy};

let strategy = BackoffStrategy::Linear {
    base_ms: 150,
    max_ms: 5_000,
};
let delay = compute_delay(&strategy, 3); // 450 ms
```

### Circuit Breaker (Conceptual)

Wrap calls to an external service so that after N consecutive failures the circuit "opens" and fast-fails until a cooldown expires.

```rust
use stellar_devkit::resilience::{retry, RetryConfig, BackoffStrategy};

// Combine retry with a small max_attempts to approximate circuit-breaker
// behaviour; a full implementation can be layered on top.
let breaker_config = RetryConfig {
    max_attempts: 3,
    backoff: BackoffStrategy::Fixed { delay_ms: 0 },
};
```

### Timeout

Use `tokio::time::timeout` alongside the retry executor to enforce per-call deadlines.

```rust
use std::time::Duration;
use stellar_devkit::resilience::{retry, RetryConfig, BackoffStrategy};

let config = RetryConfig {
    max_attempts: 3,
    backoff: BackoffStrategy::Exponential {
        base_ms: 500,
        max_ms: 5_000,
        jitter_pct: 0.0,
    },
};

let result = tokio::time::timeout(Duration::from_secs(10), retry(config, || async {
    fetch_fee_stats_from_horizon().await
})).await;
```

### Bulkhead (Conceptual)

Limit concurrent requests to a specific resource by combining a `tokio::sync::Semaphore` with the retry executor.

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;
use stellar_devkit::resilience::{retry, RetryConfig, BackoffStrategy};

let semaphore = Arc::new(Semaphore::new(10)); // max 10 concurrent
let config = RetryConfig {
    max_attempts: 3,
    backoff: BackoffStrategy::Fixed { delay_ms: 100 },
};

let permit = semaphore.clone().acquire_owned().await.unwrap();
let result = retry(config, || async {
    fetch_fee_stats_from_horizon().await
}).await;
drop(permit);
```

### Fallback

Return a default value when all retries are exhausted.

```rust
use stellar_devkit::resilience::{retry, RetryConfig, BackoffStrategy};

let config = RetryConfig {
    max_attempts: 3,
    backoff: BackoffStrategy::Fixed { delay_ms: 50 },
};

let fee = retry(config, || async {
    fetch_fee_stats_from_horizon().await
}).await.unwrap_or(100); // fallback fee of 100 stroops
```

## Adding to Your Crate

```toml
[dev-dependencies]
stellar-devkit = { path = "../devkit" }
```

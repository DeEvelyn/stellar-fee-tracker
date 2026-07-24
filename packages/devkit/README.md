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

The `simulation` module provides fee modelling, network-load generation, and congestion prediction without any live-network dependencies.

### `FeeModelConfig` fields

| Field | Type | Default | Description |
|---|---|---|---|
| `base_fee` | `u64` | `100` | Base fee in stroops |
| `spike_probability` | `f64` | `0.05` | Probability that any given ledger is a spike (0.0–1.0) |
| `spike_multiplier` | `u64` | `10` | Multiplier applied to `base_fee` during a spike |
| `ledger_interval_secs` | `u64` | `5` | Seconds between simulated ledgers |
| `ledger_count` | `u64` | `100` | Number of ledgers to generate per `run()` call |
| `seed` | `Option<u64>` | `None` | RNG seed for reproducible output |
| `noise_factor` | `f64` | `0.0` | Gaussian noise stddev as a fraction of `base_fee` |

### `NetworkLoadConfig` fields

| Field | Type | Default | Description |
|---|---|---|---|
| `min_tx` | `u64` | `10` | Minimum transactions per ledger |
| `max_tx` | `u64` | `1000` | Maximum transactions per ledger |
| `ledger_capacity` | `u64` | `1000` | Maximum tx capacity per ledger |
| `ledger_interval_ms` | `u64` | `5000` | Time between ledger closes in ms |
| `seed` | `Option<u64>` | `None` | RNG seed for reproducibility |

### Example usage

```rust
use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig};
use stellar_devkit::simulation::network_load::{NetworkLoad, NetworkLoadConfig};
use stellar_devkit::simulation::congestion_predictor::{CongestionPredictor, CongestionInput, congestion_label};

// Configure a fee scenario
let fee_cfg = FeeModelConfig {
    base_fee: 100,
    spike_probability: 0.1,
    spike_multiplier: 5,
    seed: Some(42),
    ..FeeModelConfig::default()
};

// Generate fee points
let points = FeeModel::run(&fee_cfg);
println!("Generated {} fee points", points.len());

// Configure network load
let load_cfg = NetworkLoadConfig {
    min_tx: 50,
    max_tx: 800,
    ledger_capacity: 1000,
    seed: Some(7),
    ..NetworkLoadConfig::default()
};
let mut load = NetworkLoad::new(load_cfg);
let ledgers = load.simulate(10);

// Predict congestion
let label = congestion_label(&CongestionInput {
    recent_fee_window: 250.0,
    capacity_usage: 0.75,
    spike_count: 3,
});
println!("Congestion: {:?}", label);
```

### Output format (`FeePoint`)

Each `FeePoint` represents a single simulated ledger:

| Field | Type | Description |
|---|---|---|
| `timestamp` | `u64` | Simulated Unix timestamp (seconds) |
| `fee` | `u64` | Fee in stroops for this ledger |
| `ledger` | `u64` | Ledger sequence number (1-based) |
| `is_spike` | `bool` | Whether this ledger was a spike |

### CSV export

Fee points can be exported to CSV via the CLI:

```bash
cargo run --bin devkit -- export ./fees.db --output fees.csv
```

The CSV format matches the `FeePoint` shape:

```
timestamp,fee,ledger,is_spike
1700000000,100,1,false
1700000005,500,2,true
1700000010,110,3,false
```

For programmatic export, serialise `FeePoint` slices directly:

```rust
use stellar_devkit::simulation::fee_model::{FeeModel, FeeModelConfig, FeeCurve};

let points = FeeModel::run(&FeeModelConfig::default());
let json = FeeCurve::fee_points_to_json(&points, 100)?;
println!("{}", json);
```

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

## CLI

The devkit ships with a set of subcommands for driving scenarios from the command line.

### Usage

```bash
devkit <SUBCOMMAND> [OPTIONS]
```

### Subcommands

| Subcommand | Description |
|---|---|
| `replay` | Replay recorded fee scenarios from a SQLite database |
| `export` | Export fee data to CSV |
| `benchmark` | Run performance benchmarks against the fee pipeline |
| `mock` | Serve mock Horizon `/fee_stats` responses |
| `simulate` | Run a network-load simulation and print results |

### Examples

```bash
# Replay fee records from a local SQLite file
devkit replay ./fees.db

# Export fee data to CSV
devkit export ./fees.db --output fees.csv

# Run benchmarks
devkit benchmark --samples 1000

# Start the mock server
devkit mock --port 8080 --scenario spike
```

## Adding to Your Crate

```toml
[dev-dependencies]
stellar-devkit = { path = "../devkit" }
```

## Benchmarks

Baseline results measured on reference hardware (Apple M-series, single-core, `cargo bench`):

| Benchmark | Input | Mean | Std Dev |
|---|---|---|---|
| `fee_model/run_100` | 100 ledgers, seeded | ~12 µs | ±0.3 µs |
| `fee_model/run_1000` | 1 000 ledgers, seeded | ~115 µs | ±2 µs |
| `percentile/nearest_rank_1k` | 1 000 sorted values, p50 | ~1.8 µs | ±0.05 µs |
| `rolling_window/push_1k` | 1 000 pushes, window=100 | ~900 ns | ±20 ns |

### Running benchmarks locally

```bash
cargo bench --manifest-path packages/devkit/Cargo.toml
```

HTML reports are saved to `packages/devkit/target/criterion/`.

### CI benchmarks

Benchmarks compile and run on every PR touching `packages/devkit/` via the [Devkit Benchmarks](.github/workflows/devkit-bench.yml) workflow. Results are posted to the GitHub Actions step summary.
```toml
[dev-dependencies]
stellar-devkit = { path = "../devkit" }
```

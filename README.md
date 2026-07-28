# Stellar Fee Tracker

Stellar Fee Tracker is a real-time observability service for Stellar network transaction fees. It polls the Horizon API on a configurable interval, stores fee history in SQLite, and exposes a REST API that lets developers and users query current fees, historical trends, congestion indicators, and smart fee recommendations — all without hitting Horizon directly.

## Architecture Overview

The service is built around three layers:

- **Polling loop** — A background scheduler (`packages/core/src/scheduler.rs`) calls the Horizon `/fee_stats` endpoint at a configurable interval. Each result is validated, deduplicated, and persisted to SQLite.
- **Insights engine** — An in-memory analytics engine (`packages/core/src/insights/`) computes rolling averages, extremes, congestion trends, and spike detection over the stored data.
- **API layer** — An Axum HTTP server (`packages/core/src/api/`) exposes all insights and raw history over a versioned REST API. Responses include ETag and `Cache-Control` headers for efficient client-side caching.

```
Horizon API  →  Scheduler  →  SQLite store
                                   ↓
                           Insights engine
                                   ↓
                           Axum REST API  →  Clients
```

## Prerequisites

| Dependency | Minimum version | Notes |
|---|---|---|
| Rust toolchain | 1.75 | Install via [rustup](https://rustup.rs/) |
| SQLite | 3.x | Usually pre-installed on Linux/macOS |
| (Optional) Docker | 20.x | For container deployment |

## Quickstart

```bash
# 1. Clone the repository
git clone https://github.com/StellarCommons/stellar-fee-tracker.git
cd stellar-fee-tracker

# 2. Copy the example environment file and edit as needed
cp .env.example .env

# 3. (Optional) Set STELLAR_NETWORK to mainnet if needed — default is testnet
#    Edit .env with your preferred settings

# 4. Build and run
cargo run --package stellar-fee-tracker

# 5. Verify the service is running
curl http://localhost:8080/health
```

The service creates `stellar_fees.db` in the working directory on first run.

## Configuration Reference

All configuration is via environment variables. Copy `.env.example` to `.env` and adjust.

| Variable | Type | Default | Description |
|---|---|---|---|
| `STELLAR_NETWORK` | `string` | — | **Required.** `testnet` or `mainnet` |
| `HORIZON_URL` | `string` | Network default | Override the Horizon base URL |
| `POLL_INTERVAL_SECONDS` | `u64` | `10` | Seconds between Horizon fee polls |
| `API_PORT` | `u16` | `8080` | Port the REST API listens on |
| `DATABASE_URL` | `string` | `sqlite://stellar_fees.db` | SQLite database URL |
| `CACHE_TTL_SECONDS` | `u64` | `5` | Response cache TTL for `/fees/current` |
| `STORAGE_RETENTION_DAYS` | `u64` | `7` | Days of fee history to retain |
| `RETRY_ATTEMPTS` | `u32` | `3` | Max Horizon request retries |
| `BASE_RETRY_DELAY_MS` | `u64` | `1000` | Base delay between retries (ms) |
| `RATE_LIMIT_PER_MINUTE` | `u32` | `60` | Requests per minute per IP |
| `ALLOWED_ORIGINS` | `string` | `http://localhost:3000` | CORS allowed origins |
| `API_KEY` | `string` | _(unset)_ | API key for protected routes; leave empty to disable |
| `WEBHOOK_URL` | `string` | _(unset)_ | Webhook URL for fee spike alerts; leave empty to disable |
| `ALERT_THRESHOLD` | `string` | `Major` | Alert severity: `Minor`, `Moderate`, `Major`, `Critical` |
| `RUST_LOG` | `string` | `info` | Log level: `error`, `warn`, `info`, `debug`, `trace` |

## API Overview

| Method | Path | Description |
|---|---|---|
| `GET` | `/health` | Service liveness check |
| `GET` | `/fees/current` | Current fee stats from the most recent Horizon poll |
| `GET` | `/fees/history` | Paginated fee history from SQLite |
| `GET` | `/fees/trend` | Trend direction and rate-of-change over configurable windows |
| `GET` | `/insights` | Full insights snapshot (averages, extremes, congestion) |
| `GET` | `/insights/averages` | Rolling average fees for multiple time windows |
| `GET` | `/insights/extremes` | All-time and windowed min/max fees |
| `GET` | `/insights/congestion` | Congestion score and trend for the current period |
| `GET` | `/insights/health` | Insights engine status and last-update timestamp |
| `GET` | `/alerts/config` | Get the current alert configuration |
| `POST` | `/alerts/config` | Create or update alert thresholds |
| `GET` | `/alerts/history` | Paginated history of triggered alerts |
| `GET` | `/metrics` | Prometheus-format metrics |

Full endpoint documentation with request/response examples is in [`docs/API.md`](docs/API.md).

## Running Tests

```bash
# Run all tests across the workspace
cargo test --all

# Run only the core package tests
cargo test --package stellar-fee-tracker

# Run only the devkit package tests
cargo test --package stellar-devkit

# Run tests with output
cargo test --all -- --nocapture
```

## Docker

### Build the image

```bash
docker build -t stellar-fee-tracker .
```

### Run the container

```bash
docker run \
  -e STELLAR_NETWORK=testnet \
  -e API_PORT=8080 \
  -p 8080:8080 \
  stellar-fee-tracker
```

### Run with Docker Compose

```bash
docker compose up
```

The `docker-compose.yml` in the repository root configures the service with sensible defaults and a persistent SQLite volume.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

Key points:
- Fork the repo and create a feature branch from `main`
- Write tests for any new functionality
- Ensure `cargo test --all` passes before submitting a PR
- PR descriptions must include `Closes #<issue-number>` for any issues being resolved

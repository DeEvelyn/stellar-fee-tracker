# Stellar Fee Tracker — API Reference

Base URL: `http://localhost:8080` (configurable via `API_PORT`)

All endpoints return JSON unless otherwise noted. Successful responses are always `200 OK` with a JSON body. Error responses use standard HTTP status codes and the shape `{"error": "<message>"}`.

Responses include the following caching headers where applicable:

- `Cache-Control: public, max-age=<n>, s-maxage=<m>` — indicates cacheability
- `ETag` — opaque fingerprint of the response body
- `Last-Modified` — RFC 7231 timestamp of the last data update

Clients should send `If-None-Match: <etag>` on subsequent requests; if the data has not changed the server returns `304 Not Modified` with no body.

---

## `GET /health`

Service liveness check. Returns `ok` as plain text.

**Request**

```bash
curl http://localhost:8080/health
```

**Response** — `200 OK`

```
ok
```

**Headers**

```
Cache-Control: no-store
```

---

## `GET /fees/current`

Returns the most recent fee stats fetched from Horizon.

**Request**

```bash
curl http://localhost:8080/fees/current
```

**Response** — `200 OK`

```json
{
  "base_fee": "100",
  "min_fee": "100",
  "max_fee": "10000",
  "avg_fee": "250",
  "percentiles": {
    "p10": "100",
    "p20": "100",
    "p30": "100",
    "p40": "150",
    "p50": "200",
    "p60": "250",
    "p70": "300",
    "p80": "500",
    "p90": "1000",
    "p95": "3000",
    "p99": "8000"
  }
}
```

**Error responses**

| Status | Body | Cause |
|---|---|---|
| `503 Service Unavailable` | `{"error": "Horizon unavailable"}` | Horizon could not be reached |
| `500 Internal Server Error` | `{"error": "<reason>"}` | Unexpected server error |

---

## `GET /fees/history`

Returns paginated fee history from SQLite.

**Query parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `limit` | `u32` | No | `100` | Number of records to return (max `1000`) |
| `offset` | `u32` | No | `0` | Number of records to skip |
| `from` | `i64` | No | — | Start of time range (Unix timestamp, seconds) |
| `to` | `i64` | No | — | End of time range (Unix timestamp, seconds) |

**Request**

```bash
# Last 50 records
curl "http://localhost:8080/fees/history?limit=50"

# Records from the last hour
curl "http://localhost:8080/fees/history?from=1700000000&to=1700003600"
```

**Response** — `200 OK`

```json
{
  "records": [
    {
      "id": 1,
      "timestamp": 1700000000,
      "base_fee": 100,
      "min_fee": 100,
      "max_fee": 5000,
      "avg_fee": 300,
      "ledger_sequence": 48000001
    }
  ],
  "total": 1,
  "limit": 100,
  "offset": 0
}
```

**Error responses**

| Status | Body | Cause |
|---|---|---|
| `400 Bad Request` | `{"error": "invalid limit"}` | `limit` exceeds maximum or is not a number |
| `500 Internal Server Error` | `{"error": "<reason>"}` | Database query failed |

---

## `GET /fees/trend`

Returns the trend direction and rate-of-change over configurable time windows.

**Query parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `window_minutes` | `u32` | No | `60` | Window size in minutes |

**Request**

```bash
curl "http://localhost:8080/fees/trend?window_minutes=30"
```

**Response** — `200 OK`

```json
{
  "direction": "Upward",
  "strength": "Moderate",
  "rate_of_change": 12.4,
  "window_minutes": 30,
  "data_points": 180,
  "computed_at": "2024-01-15T10:30:00Z"
}
```

Fields:

| Field | Type | Description |
|---|---|---|
| `direction` | `string` | `Upward`, `Downward`, or `Stable` |
| `strength` | `string` | `Weak`, `Moderate`, or `Strong` |
| `rate_of_change` | `f64` | Stroops per minute (positive = rising, negative = falling) |
| `window_minutes` | `u32` | Window used for the calculation |
| `data_points` | `u32` | Number of records used |
| `computed_at` | `string` | ISO 8601 timestamp of the computation |

---

## `GET /insights`

Returns a full insights snapshot including rolling averages, extremes, and congestion data.

**Request**

```bash
curl http://localhost:8080/insights
```

**Response** — `200 OK`

```json
{
  "rolling_averages": {
    "avg_1h": 250.5,
    "avg_6h": 210.3,
    "avg_24h": 195.8
  },
  "extremes": {
    "all_time_min": 100,
    "all_time_max": 50000,
    "window_min": 100,
    "window_max": 8000
  },
  "congestion": {
    "score": 0.42,
    "label": "Moderate",
    "trend": "Increasing"
  },
  "last_updated": "2024-01-15T10:30:00Z"
}
```

---

## `GET /insights/averages`

Returns rolling average fees computed over multiple time windows.

**Request**

```bash
curl http://localhost:8080/insights/averages
```

**Response** — `200 OK`

```json
{
  "avg_1h": 250.5,
  "avg_6h": 210.3,
  "avg_24h": 195.8,
  "avg_7d": 180.2
}
```

---

## `GET /insights/extremes`

Returns all-time and windowed minimum/maximum fee values.

**Request**

```bash
curl http://localhost:8080/insights/extremes
```

**Response** — `200 OK`

```json
{
  "all_time_min": 100,
  "all_time_max": 50000,
  "window_min": 100,
  "window_max": 8000,
  "window_hours": 24
}
```

---

## `GET /insights/congestion`

Returns the current congestion score, label, and trend direction.

**Request**

```bash
curl http://localhost:8080/insights/congestion
```

**Response** — `200 OK`

```json
{
  "score": 0.42,
  "label": "Moderate",
  "trend": "Increasing",
  "spike_count_1h": 3,
  "capacity_usage": 0.65
}
```

Fields:

| Field | Type | Description |
|---|---|---|
| `score` | `f64` | Congestion score from 0.0 (none) to 1.0 (maximum) |
| `label` | `string` | `Low`, `Moderate`, `High`, or `Critical` |
| `trend` | `string` | `Increasing`, `Decreasing`, or `Stable` |
| `spike_count_1h` | `u32` | Number of fee spikes detected in the last hour |
| `capacity_usage` | `f64` | Estimated network capacity utilisation (0.0–1.0) |

---

## `GET /insights/health`

Returns the health status of the insights engine.

**Request**

```bash
curl http://localhost:8080/insights/health
```

**Response** — `200 OK`

```json
{
  "status": "healthy",
  "last_update": "2024-01-15T10:30:00Z",
  "config": {
    "polling_interval_seconds": 10,
    "time_windows": 4,
    "spike_threshold": 3.0
  }
}
```

---

Alert configs and events carry an `alert_type` of `spike`, `recovery`,
`good_window`, or `stale_data` (Issue #556). `spike` fires when a fee
spike crosses the configured `threshold`; `recovery` fires when an
active spike condition clears, correlated back to the spike it resolves
via `correlation_id`; `good_window` fires when congestion is declining
(a good time to submit); `stale_data` fires when the poll pipeline's
data freshness exceeds the configured staleness threshold. All four
share the same webhook delivery mechanism (SSRF-guarded HTTPS-only
URLs, retry-once on non-2xx). The webhook payload's `event` field
identifies which kind fired (`fee_spike_detected`,
`fee_spike_recovered`, `good_submission_window`, `data_pipeline_stale`);
fields that don't apply to a given type (e.g. `peak_fee` for a
`good_window` event) are omitted from the JSON body entirely rather than
sent as `null`.

## `POST /alerts/config`

Registers a new alert webhook target. Each call creates a new,
independent config row — it does not replace or merge with existing
ones.

**Request body**

```json
{
  "webhook_url": "https://hooks.slack.com/services/xxx",
  "threshold": "Major",
  "alert_type": "spike"
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `webhook_url` | `string` | Yes | HTTPS URL with a public hostname (SSRF-guarded: loopback/private/link-local hosts are rejected) |
| `threshold` | `string` | No | Alert severity: `Minor`, `Moderate`, `Major`, `Critical` (default `Major`). Only meaningful for `spike`/`recovery` alert types |
| `alert_type` | `string` | No | `spike`, `recovery`, `good_window`, or `stale_data` (default `spike`) |

**Request**

```bash
curl -X POST http://localhost:8080/alerts/config \
  -H "Content-Type: application/json" \
  -d '{"webhook_url": "https://hooks.slack.com/services/xxx", "threshold": "Moderate", "alert_type": "stale_data"}'
```

**Response** — `201 Created`

```json
{ "id": 1 }
```

**Error responses**

| Status | Body | Cause |
|---|---|---|
| `400 Bad Request` | `{"error": "Invalid threshold '<value>'. Must be one of: ..."}` | Unrecognised threshold value |
| `400 Bad Request` | `{"error": "Invalid alert_type '<value>'. Must be one of: ..."}` | Unrecognised alert_type value |
| `400 Bad Request` | `{"error": "Invalid webhook_url: must be an HTTPS URL with a public hostname"}` | URL is not HTTPS, or resolves to a private/loopback/link-local host |

---

## `GET /alerts/config`

Lists every registered alert webhook config, including disabled
(soft-deleted) ones.

**Request**

```bash
curl http://localhost:8080/alerts/config
```

**Response** — `200 OK`

```json
[
  {
    "id": 1,
    "webhook_url": "https://hooks.slack.com/services/xxx",
    "threshold": "Major",
    "alert_type": "spike",
    "enabled": true,
    "created_at": "2024-01-15T08:00:00Z"
  }
]
```

---

## `PATCH /alerts/config/:id`

Partially updates a config. Any field omitted from the request body
keeps its current value.

**Request body**

```json
{
  "threshold": "Critical",
  "enabled": false,
  "alert_type": "good_window"
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `threshold` | `string` | No | New severity threshold |
| `enabled` | `bool` | No | New enabled state |
| `alert_type` | `string` | No | New alert type: `spike`, `recovery`, `good_window`, or `stale_data` |

**Request**

```bash
curl -X PATCH http://localhost:8080/alerts/config/1 \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'
```

**Response** — `204 No Content`

**Error responses**

| Status | Body | Cause |
|---|---|---|
| `400 Bad Request` | `{"error": "Invalid threshold '<value>'. Must be one of: ..."}` | Unrecognised threshold value |
| `400 Bad Request` | `{"error": "Invalid alert_type '<value>'. Must be one of: ..."}` | Unrecognised alert_type value |
| `404 Not Found` | `{"error": "Alert config not found"}` | No config with that id |

---

## `DELETE /alerts/config/:id`

Soft-deletes a config by setting `enabled` to `false`. The row still
appears in `GET /alerts/config`.

**Request**

```bash
curl -X DELETE http://localhost:8080/alerts/config/1
```

**Response** — `204 No Content`

**Error responses**

| Status | Body | Cause |
|---|---|---|
| `404 Not Found` | `{"error": "Alert config not found"}` | No config with that id |

---

## `GET /alerts/history`

Returns a paginated list of triggered alert events.

**Query parameters**

| Name | Type | Required | Default | Description |
|---|---|---|---|---|
| `limit` | `i64` | No | `20` | Number of events to return (clamped to `100`) |
| `severity` | `string` | No | — | Filter by severity: `Minor`, `Moderate`, `Major`, `Critical` |
| `delivered` | `bool` | No | — | Filter by webhook delivery success |
| `alert_type` | `string` | No | — | Filter by `spike`, `recovery`, `good_window`, or `stale_data` |

**Request**

```bash
curl "http://localhost:8080/alerts/history?limit=10&alert_type=stale_data"
```

**Response** — `200 OK`

```json
{
  "total": 1,
  "items": [
    {
      "id": 42,
      "config_id": 1,
      "alert_type": "stale_data",
      "severity": "Major",
      "peak_fee": 0,
      "baseline_fee": 0.0,
      "spike_ratio": 0.0,
      "webhook_url": "https://hooks.slack.com/services/xxx",
      "delivered": true,
      "triggered_at": "2024-01-15T09:15:00Z",
      "correlation_id": null
    }
  ]
}
```

`peak_fee`, `baseline_fee`, and `spike_ratio` only carry meaningful
values for `spike` events; they default to `0`/`0.0` for the other
three alert types. `correlation_id` is set only on `recovery` events,
identifying the spike they resolve.

**Error responses**

| Status | Body | Cause |
|---|---|---|
| `400 Bad Request` | `{"error": "Invalid severity '<value>'. Must be one of: ..."}` | Unrecognised severity value |
| `400 Bad Request` | `{"error": "Invalid alert_type '<value>'. Must be one of: ..."}` | Unrecognised alert_type value |

---

## `GET /metrics`

Returns Prometheus-format metrics for scraping.

**Request**

```bash
curl http://localhost:8080/metrics
```

**Response** — `200 OK`, `Content-Type: text/plain; version=0.0.4`

```
# HELP stellar_fee_polls_total Total number of Horizon fee polls attempted
# TYPE stellar_fee_polls_total counter
stellar_fee_polls_total 1234

# HELP stellar_fee_poll_errors_total Total number of failed Horizon fee polls
# TYPE stellar_fee_poll_errors_total counter
stellar_fee_poll_errors_total 2

# HELP stellar_current_base_fee Current base fee in stroops from the last successful poll
# TYPE stellar_current_base_fee gauge
stellar_current_base_fee 100

# HELP stellar_api_requests_total Total number of REST API requests by path and status
# TYPE stellar_api_requests_total counter
stellar_api_requests_total{path="/fees/current",status="200"} 890
stellar_api_requests_total{path="/insights",status="200"} 341
```

---

## Error Response Format

All error responses use this consistent shape:

```json
{
  "error": "human-readable error message"
}
```

Common HTTP status codes:

| Status | Meaning |
|---|---|
| `400` | Bad request — malformed query parameters or request body |
| `404` | Not found — resource does not exist |
| `422` | Unprocessable entity — valid JSON but semantically invalid |
| `429` | Too many requests — rate limit exceeded |
| `500` | Internal server error — unexpected failure |
| `503` | Service unavailable — Horizon unreachable |

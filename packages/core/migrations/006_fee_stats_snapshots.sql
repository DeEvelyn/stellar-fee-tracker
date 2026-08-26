-- Migration 004: Ledger-keyed fee_stats snapshots (Issue #550).
--
-- Stores one aggregate snapshot per Horizon ledger, sourced from the
-- `/fee_stats` endpoint instead of sampled /transactions data.
-- `ledger` is the primary key so persistence is idempotent: re-polling
-- a ledger UPSERTs (ON CONFLICT (ledger) DO UPDATE) rather than
-- duplicating rows.

CREATE TABLE IF NOT EXISTS fee_stats_snapshots (
    ledger                INTEGER PRIMARY KEY,
    base_fee              INTEGER NOT NULL,
    min_fee_charged       INTEGER NOT NULL,
    max_fee_charged       INTEGER NOT NULL,
    mode_fee_charged      INTEGER NOT NULL,
    mean_fee_charged      REAL    NOT NULL,
    median_fee_charged    INTEGER NOT NULL,
    p10_fee_charged       INTEGER NOT NULL,
    p95_fee_charged       INTEGER NOT NULL,
    p99_fee_charged       INTEGER NOT NULL,
    max_fee               INTEGER NOT NULL,
    ledger_capacity_usage REAL,
    captured_at           TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_fee_stats_snapshots_captured_at
    ON fee_stats_snapshots (captured_at);

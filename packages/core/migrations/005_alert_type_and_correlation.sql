-- Migration 005: Alert type dimension + spike/recovery correlation
-- Extends alert_configs and alert_events to support multiple alert kinds
-- (spike | recovery | good_window | stale_data) beyond the original
-- spike-only threshold model. Existing rows default to 'spike' for
-- backward compatibility.

ALTER TABLE alert_configs
    ADD COLUMN alert_type TEXT NOT NULL DEFAULT 'spike';

ALTER TABLE alert_events
    ADD COLUMN alert_type TEXT NOT NULL DEFAULT 'spike';

-- Links a recovery event back to the spike event/identity it resolves.
-- NULL for spike, good_window, and stale_data events (nothing to correlate).
ALTER TABLE alert_events
    ADD COLUMN correlation_id TEXT;

CREATE INDEX IF NOT EXISTS idx_alert_events_correlation_id
    ON alert_events (correlation_id);

CREATE INDEX IF NOT EXISTS idx_alert_events_alert_type
    ON alert_events (alert_type);

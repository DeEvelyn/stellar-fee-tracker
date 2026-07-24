CREATE TABLE IF NOT EXISTS recommendations (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  recommended_fee   INTEGER NOT NULL,
  confidence        REAL    NOT NULL,
  target_ledgers    INTEGER NOT NULL,
  network_condition TEXT    NOT NULL,
  percentile_basis  TEXT    NOT NULL,
  input_confidence  REAL    NOT NULL,
  input_ledgers     INTEGER NOT NULL,
  sample_count      INTEGER NOT NULL,
  computed_at       TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recommendations_computed_at
    ON recommendations (computed_at);

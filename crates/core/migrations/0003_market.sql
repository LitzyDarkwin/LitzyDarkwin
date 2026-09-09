-- 0003_market.sql — schema v3: market cache, refresh log, provider health.
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS market_items (
  item_id INTEGER PRIMARY KEY,
  region TEXT NOT NULL DEFAULT 'eu',
  market_value INTEGER NOT NULL DEFAULT 0,
  min_buyout INTEGER NOT NULL DEFAULT 0,
  historical INTEGER NOT NULL DEFAULT 0,
  sale_avg INTEGER NOT NULL DEFAULT 0,
  sale_rate REAL NOT NULL DEFAULT 0.0,
  quantity INTEGER NOT NULL DEFAULT 0,
  market_updated_at INTEGER NOT NULL DEFAULT 0,
  region_updated_at INTEGER NOT NULL DEFAULT 0,
  fetched_at INTEGER NOT NULL DEFAULT 0,
  provider TEXT NOT NULL DEFAULT 'tsm'
);

CREATE INDEX IF NOT EXISTS idx_market_region ON market_items(region);
CREATE INDEX IF NOT EXISTS idx_market_fetched ON market_items(fetched_at);

CREATE TABLE IF NOT EXISTS market_refresh (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider TEXT NOT NULL,
  region TEXT NOT NULL,
  dataset TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  finished_at INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'started',
  rows_total INTEGER NOT NULL DEFAULT 0,
  rows_upserted INTEGER NOT NULL DEFAULT 0,
  rows_skipped INTEGER NOT NULL DEFAULT 0,
  error TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_refresh_provider ON market_refresh(provider, region, dataset, started_at);

CREATE TABLE IF NOT EXISTS provider_state (
  provider TEXT PRIMARY KEY,
  dataset TEXT NOT NULL DEFAULT '',
  last_success_at INTEGER NULL,
  last_attempt_at INTEGER NOT NULL DEFAULT 0,
  last_error TEXT NULL,
  failures INTEGER NOT NULL DEFAULT 0,
  next_retry_at INTEGER NOT NULL DEFAULT 0
);

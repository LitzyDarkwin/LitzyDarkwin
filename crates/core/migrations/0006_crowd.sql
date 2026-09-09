-- 0006_crowd.sql — schema v6: crowd aggregate cache (local-first, opt-in).
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS crowd_observations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  zone_id TEXT NOT NULL REFERENCES zones(id) ON DELETE CASCADE,
  resource_id TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
  bucket_x INTEGER NOT NULL,
  bucket_y INTEGER NOT NULL,
  day_bucket INTEGER NOT NULL,
  success INTEGER NOT NULL DEFAULT 0,
  trials INTEGER NOT NULL DEFAULT 0,
  contributors INTEGER NOT NULL DEFAULT 0,
  weight REAL NOT NULL DEFAULT 0.0,
  updated_at INTEGER NOT NULL DEFAULT 0,
  UNIQUE (zone_id, resource_id, bucket_x, bucket_y, day_bucket)
);

CREATE INDEX IF NOT EXISTS idx_crowd_zone_day ON crowd_observations(zone_id, day_bucket);
CREATE INDEX IF NOT EXISTS idx_crowd_resource ON crowd_observations(resource_id, updated_at);

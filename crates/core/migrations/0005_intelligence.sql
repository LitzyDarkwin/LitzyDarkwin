-- 0005_intelligence.sql — schema v5: personal learning tables.
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS personal_metrics (
  key TEXT PRIMARY KEY,
  value REAL NOT NULL,
  samples INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS personal_zone_stats (
  zone_id TEXT PRIMARY KEY REFERENCES zones(id) ON DELETE CASCADE,
  sessions INTEGER NOT NULL DEFAULT 0,
  total_seconds INTEGER NOT NULL DEFAULT 0,
  avg_gph REAL NOT NULL DEFAULT 0.0,
  efficiency REAL NOT NULL DEFAULT 1.0,
  hit_rate REAL NOT NULL DEFAULT 0.0,
  updated_at INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS personal_resource_stats (
  resource_id TEXT PRIMARY KEY REFERENCES resources(id) ON DELETE CASCADE,
  gathers INTEGER NOT NULL DEFAULT 0,
  total_qty REAL NOT NULL DEFAULT 0.0,
  avg_yield REAL NOT NULL DEFAULT 0.0,
  avg_gold_per_gather REAL NOT NULL DEFAULT 0.0,
  updated_at INTEGER NOT NULL DEFAULT 0
);

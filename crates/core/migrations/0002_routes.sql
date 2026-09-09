-- 0002_routes.sql — schema v2: saved routes.
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS saved_routes (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  zone_id TEXT NULL REFERENCES zones(id) ON DELETE SET NULL,
  resource_ids_json TEXT NOT NULL DEFAULT '[]',
  waypoints_json TEXT NOT NULL DEFAULT '[]',
  excluded_json TEXT NOT NULL DEFAULT '[]',
  config_json TEXT NOT NULL DEFAULT '{}',
  stats_json TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_routes_zone ON saved_routes(zone_id);
CREATE INDEX IF NOT EXISTS idx_routes_updated ON saved_routes(updated_at);

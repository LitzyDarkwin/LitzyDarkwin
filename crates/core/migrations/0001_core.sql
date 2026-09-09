-- 0001_core.sql — schema v1: atlas hierarchy, resources, observations, settings.
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS expansions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  short TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS continents (
  id TEXT PRIMARY KEY,
  expansion_id TEXT NOT NULL REFERENCES expansions(id) ON DELETE CASCADE,
  name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS zones (
  id TEXT PRIMARY KEY,
  continent_id TEXT NOT NULL REFERENCES continents(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  map_id INTEGER NULL,
  yards_per_unit REAL NOT NULL DEFAULT 10.0,
  bounds_json TEXT NOT NULL DEFAULT '{}',
  meta_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS farm_categories (
  id TEXT PRIMARY KEY,
  key TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS resources (
  id TEXT PRIMARY KEY,
  category_id TEXT NOT NULL REFERENCES farm_categories(id) ON DELETE RESTRICT,
  key TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  item_id INTEGER NULL,
  quality INTEGER NOT NULL DEFAULT 1,
  meta_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS resource_observations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  zone_id TEXT NOT NULL REFERENCES zones(id) ON DELETE CASCADE,
  resource_id TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
  map_id INTEGER NULL,
  x REAL NOT NULL,
  y REAL NOT NULL,
  source TEXT NOT NULL DEFAULT 'seed',
  yield_qty REAL NULL,
  observed_at INTEGER NOT NULL DEFAULT 0,
  bucket TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_obs_zone_xy ON resource_observations(zone_id, x, y);
CREATE INDEX IF NOT EXISTS idx_obs_resource_time ON resource_observations(resource_id, observed_at);
CREATE INDEX IF NOT EXISTS idx_obs_source ON resource_observations(source);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL DEFAULT 0
);

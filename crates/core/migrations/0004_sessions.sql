-- 0004_sessions.sql — schema v4: farm sessions and session loot.
-- (Created during Phase 5 milestone; consumed by the Phase 6 session system.)
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS farm_sessions (
  id TEXT PRIMARY KEY,
  external_id TEXT NULL,
  character_name TEXT NOT NULL DEFAULT '',
  realm TEXT NOT NULL DEFAULT '',
  zone_id TEXT NULL REFERENCES zones(id) ON DELETE SET NULL,
  route_id TEXT NULL REFERENCES saved_routes(id) ON DELETE SET NULL,
  status TEXT NOT NULL DEFAULT 'idle',
  started_at INTEGER NOT NULL DEFAULT 0,
  ended_at INTEGER NULL,
  active_seconds INTEGER NOT NULL DEFAULT 0,
  expected_gph INTEGER NULL,
  expected_value INTEGER NULL,
  notes TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_sessions_status ON farm_sessions(status, started_at);
CREATE INDEX IF NOT EXISTS idx_sessions_zone ON farm_sessions(zone_id, started_at);
CREATE INDEX IF NOT EXISTS idx_sessions_external ON farm_sessions(external_id);

CREATE TABLE IF NOT EXISTS farm_session_items (
  session_id TEXT NOT NULL REFERENCES farm_sessions(id) ON DELETE CASCADE,
  item_id INTEGER NOT NULL,
  resource_id TEXT NULL REFERENCES resources(id) ON DELETE SET NULL,
  qty REAL NOT NULL DEFAULT 0.0,
  unit_value INTEGER NULL,
  PRIMARY KEY (session_id, item_id)
);

CREATE INDEX IF NOT EXISTS idx_session_items_resource ON farm_session_items(resource_id);

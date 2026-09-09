# DATA MODEL

Canonical entities (SQLite; migrations in `crates/core/migrations/`, embedded via
`include_str!`). Money = integer copper. Time = Unix seconds UTC.

## Schema versions

| v | migration              | adds                                                        |
|---|------------------------|-------------------------------------------------------------|
| 1 | 0001_core.sql          | expansions, continents, zones, farm_categories, resources,  |
|   |                        | resource_observations, app_settings                         |
| 2 | 0002_routes.sql        | saved_routes                                                |
| 3 | 0003_market.sql        | market_items, market_refresh, provider_state                |
| 4 | 0004_sessions.sql      | farm_sessions, farm_session_items                           |
| 5 | 0005_intelligence.sql  | personal_metrics, personal_zone_stats, personal_resource_stats |
| 6 | 0006_crowd.sql         | crowd_observations (local aggregate cache)                  |
| 7 | 0007_market_tsm.sql    | market_items ⇄ TSM columns; provider_state PK fix           |

`PRAGMA user_version` = schema version. Runner backs up DB before destructive
migrations (Phase 9 backup behavior; no destructive migrations shipped yet).

## Core tables (abbreviated; see migration SQL for exact DDL)

- `expansions(id TEXT PK, name, short, sort_order)`
- `continents(id TEXT PK, expansion_id FK, name)`
- `zones(id TEXT PK, continent_id FK, name, map_id INT, yards_per_unit REAL, bounds_json)`
- `farm_categories(id TEXT PK, key UNIQUE, name)` — mining, herbalism, fishing,
  skinning, treasure, mob, other (extensible without redesign).
- `resources(id TEXT PK, category_id FK, key UNIQUE, name, item_id INT NULL,
  quality INT DEFAULT 1, meta_json)`
- `resource_observations(id INTEGER PK, zone_id FK, resource_id FK, map_id, x, y,
  source TEXT, yield_qty REAL NULL, observed_at INT, bucket TEXT NULL)` + spatial index
  `(zone_id, x, y)` and `(resource_id, observed_at)`.
- `app_settings(key TEXT PK, value_json NOT NULL, updated_at INT)`
- `saved_routes(id TEXT PK, name, zone_id FK, resource_ids_json, waypoints_json
  [{x,y,cluster_id,value}], config_json, stats_json, created_at, updated_at)`
- `market_items(item_id INT PK, region TEXT, market_value INT, min_buyout INT,
  recent INT, historical INT, avg_sale_price INT, sale_rate REAL,
  sold_per_day REAL, item_name TEXT, market_updated_at INT, region_updated_at INT,
  fetched_at INT, provider TEXT)` (v7 TSM alignment)
- `market_refresh(id INTEGER PK, provider, region, dataset, started_at, finished_at,
  status TEXT, rows_total INT, rows_upserted INT, rows_skipped INT, error TEXT NULL)`
- `provider_state(provider TEXT, dataset TEXT, last_success_at INT NULL,
  last_attempt_at INT, last_error TEXT NULL, failures INT, next_retry_at INT;
  PK(provider, dataset))`
- `farm_sessions(id TEXT PK, external_id TEXT NULL, character_name, realm,
  zone_id NULL, route_id NULL, status TEXT, started_at INT, ended_at INT NULL,
  active_seconds INT, expected_gph INT NULL, notes TEXT)`
- `farm_session_items(session_id FK, item_id INT, resource_id TEXT NULL, qty REAL,
  unit_value INT NULL, PRIMARY KEY(session_id, item_id))`
- `personal_metrics(key TEXT PK, value REAL, samples INT, updated_at INT)`
- `personal_zone_stats(zone_id TEXT PK, sessions INT, avg_gph REAL,
  efficiency REAL, hit_rate REAL, updated_at INT)`
- `personal_resource_stats(resource_id TEXT PK, gathers INT, avg_yield REAL,
  avg_gold_per_gather REAL, updated_at INT)`
- `crowd_observations(id INTEGER PK, zone_id, resource_id, bucket_x INT, bucket_y INT,
  day_bucket INT, success INT, trials INT, contributors INT, weight REAL, updated_at INT)`

## Freshness semantics

| state         | meaning                                        |
|---------------|------------------------------------------------|
| Fresh         | within `fresh_secs` for that source            |
| Aging         | within `stale_secs`, reduced confidence        |
| Stale         | older than `stale_secs`, usable with penalty   |
| Unavailable   | source failed / never synced                   |
| Unknown       | no timestamp recorded                          |

Defaults: market fresh 12h / stale 72h; addon sync fresh 10min / stale 24h;
crowd fresh 24h / stale 14d. Stored in config, surfaced in UI.

## Well-known `app_settings` keys (JSON values)

| key | type | default | writer |
|-----|------|---------|--------|
| `wow_dir` | string\|null | null | Settings UI |
| `region` | string | `eu` | Settings UI |
| `market_provider` | string | `tsm` | Settings UI |
| `market_refresh_hours` | int | 12 | Settings UI |
| `crowd.share` / `crowd.use` | bool | false | Settings UI |
| `crowd.endpoint` | string | `""` | Settings UI |
| `route.config` / `route.movement_profile` | object | `{}` | Routes UI (P4) |
| `ui.last_route` | string\|null | null | UI |
| `sync.last_at` | int\|null | null | bridge |
| `sync.last_result` | string\|null | null | bridge |
| `sync.last_export_at` | int\|null | null | bridge |

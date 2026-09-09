# PROVIDERS — external/addon contracts

All external data is untrusted: validate, normalize, attribute, timestamp.

## Market — TSM public data (primary)
- Base: `https://public-data.tradeskillmaster.com/{gameType}/{region}/…`
- Retail EU datasets:
  - `retail/eu/commodities.csv` (columns incl. `itemID,minBuyout,marketValue,historical,…`)
  - `retail/eu/region/items.csv` (columns incl. `itemID,marketValue,historical,saleAvg,…`)
- No API key, no rate limits (public static files). Refresh cadence observed ~daily;
  treat as volatile: record `fetched_at` + provider-reported timestamps, never assume.
- Contract in core: `MarketProvider { id(), datasets(), parse(dataset, bytes) }`;
  fetching via `HttpFetcher` (shell). Malformed rows are skipped + counted, never fatal.
- Fallback chain: fresh cache → stale cache (penalized) → Unknown (explicit risk).

## Atlas seed data — bundled provider
- `crates/core/src/seed/*.json`: expansions/continents/zones/resources + representative
  node priors for initial mining/herbalism zones. Source attribution: `seed`.
- Replaceable: future providers (community DB, user CSV import) implement the same
  `AtlasProvider` trait and normalize into canonical entities.

## Addon bridge — GatherOptimizer SavedVariables
- File: `<WoW>/_retail_/WTF/Account/<ACCT>/SavedVariables/GatherOptimizer.lua`
- Lua table `GatherOptimizerDB` with `schemaVersion`, `sessions[]`, `observations[]`,
  `export { version, exportedAt }`. Parsed by core's Lua-subset parser (`lua.rs`).
- Version mismatch → structured `BridgeError::SchemaMismatch { found, supported }`
  with user guidance, never silent mis-parse.

## Optional in-game integrations (feature-detected, never required)
- **TomTom**: `TomTom:AddWaypoint(...)`, `TomTom:SetCrazyArrow(...)` if global exists.
- **GatherMate2**: show/hide node icons via its public API only when `GatherMate2`
  loads; our data never depends on its internals.
- **Routes**: foreground route drawing via its public API when present; else our
  waypoint list still works standalone.
- All calls wrapped in `pcall`; absence only disables that adapter (`status` shown).

## Crowd (future remote)
- HTTP contract: `POST /v1/observations` (anonymous bucketed payload, client hash
  rotated daily), `GET /v1/aggregate?region&zone&since`. Disabled by default;
  local mock aggregator implements the same trait for tests/offline.

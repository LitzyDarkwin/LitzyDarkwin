# ARCHITECTURE

## System

```text
World of Warcraft (Retail)
  │  GatherOptimizer addon (Lua, event-driven, optional deps only)
  ▼
Versioned SavedVariables  ── safe sync boundary (file mtime + export timestamp)
  │  desktop reads SV file (WoW must /reload or logout to flush — shown honestly)
  │  desktop → addon via in-game import string (/go import), never fake IPC
  ▼
Tauri 2 shell (apps/desktop/src-tauri) — thin commands, fs/dialog, HTTP fetch
  ▼
wowgoldfarm-core (crates/core) — ALL business logic, sync, no GUI deps
  ▼
SQLite (rusqlite, bundled) — durable local state, file-based migrations
  ▼
React UI (apps/desktop/src) — presentation only, typed API wrappers
```

## Crate layout

- `crates/core`: `error, models, storage, migrations, lua, bridge, atlas, providers,
  routes, market, sessions, intelligence, crowd, settings, diagnostics, seed`.
  Pure Rust + rusqlite(bundled)/serde/chrono/thiserror/csv. Testable anywhere.
- `apps/desktop/src-tauri`: Tauri commands (thin), `reqwest` fetcher implementing
  core's `HttpFetcher`, app-data paths, backup. GUI deps isolated here.

## Data flows

- **Addon sync**: `detect_wow → read SV lua → lua.rs subset parser → validate version →
  upsert observations/sessions → sync state + freshness`.
- **Market**: `provider fetch (shell) → core parse/validate → market_items cache →
  valuation (market/conservative/effective + confidence) → routes/sessions`.
  Failed refresh NEVER destroys cache. Cooldowns + backoff + provider health.
- **Routes**: `eligible observations → normalize → cluster → value → prune →
  candidates → 2-opt/local search → simulate (P10/med/P90) → rank → persist`.
- **Sessions**: `Idle→Prepared→Active→Paused→Ended→Imported→Analyzed`;
  `RealizedGPH = RealizedEffectiveValue / ActiveSessionHours`.
- **Intelligence**: `generic prior + personal observations → confidence-weighted blend →
  personalized estimate` (min-sample thresholds + decay, no overfit).
- **Crowd** (opt-in): anonymous bucketed signals → validate/dedupe → crowd prior →
  blended under personal observations.

## Key contracts

- Freshness: `Fresh | Aging | Stale | Unavailable | Unknown` (see DATA_MODEL).
- Confidence: 0.0–1.0 from measured factors (samples, recency, agreement, volatility).
- Money: integer copper internally; gold formatting at presentation edge.
- Time: Unix seconds (UTC) everywhere; addon uses `GetServerTime()`.
- Determinism: seeded RNG; stored configs/weights; fixtures with expected outputs.

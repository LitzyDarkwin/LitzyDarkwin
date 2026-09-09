//! wowgoldfarm-core — domain logic for the wowgoldfarm desktop app.
//!
//! Module map (see docs/ARCHITECTURE.md):
//! - `error` — structured error model with user-facing messages.
//! - `models` — canonical domain types (freshness, confidence, money, ids).
//! - `storage` — SQLite connection handling and pragmas.
//! - `migrations` — sequential migration runner (embeds `migrations/*.sql`).
//! - `lua` — safe SavedVariables Lua-subset parser.
//! - `bridge` — addon synchronization orchestration.
//! - `atlas`, `providers` — farming atlas + provider adapters.
//! - `routes` — economic route optimization + simulation.
//! - `market` — market cache, providers, valuation.
//! - `sessions` — farm session lifecycle + analytics.
//! - `intelligence` — personal adaptive learning + recommendations.
//! - `crowd` — optional opt-in crowd aggregation.
//! - `settings` — typed application settings.
//! - `diagnostics` — health + diagnostic reports.
//! - `seed` — bundled atlas seed data loader.

pub mod atlas;
pub mod bridge;
pub mod crowd;
pub mod diagnostics;
pub mod error;
pub mod intelligence;
pub mod lua;
pub mod market;
pub mod migrations;
pub mod models;
pub mod providers;
pub mod routes;
pub mod seed;
pub mod sessions;
pub mod settings;
pub mod storage;

/// Current addon SavedVariables schema supported by the bridge.
pub const ADDON_SCHEMA_VERSION: u32 = 1;
/// Current SQLite schema version (mirrors number of migrations).
pub const SCHEMA_VERSION: u32 = 6;

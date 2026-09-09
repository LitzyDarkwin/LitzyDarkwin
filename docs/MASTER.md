# MASTER — wowgoldfarm product requirements (stable)

> Answer: "What should I farm, where, which route, and what realistic gold/hour for me right now?"

## Product

World of Warcraft Retail farming-intelligence platform:

1. **GatherOptimizer** WoW addon (Lua) — session tracking, observations, navigation hints.
2. **Windows 11 desktop app** (Tauri 2 + React + Rust + SQLite).
3. **Farming Atlas** — expansions → continents → zones → categories → resources → observations.
4. **Economic route engine** — maximize expected value per active hour (not shortest path).
5. **Market engine** — TSM public CSV provider (EU first), conservative valuation, spike protection.
6. **Live Farm dashboard** — freshest legitimately synced data, expected vs realized GPH.
7. **Personal adaptive intelligence** — explainable, confidence-weighted learning from own history.
8. **Optional crowd intelligence** — opt-in, anonymous, never trusted blindly.
9. **Production packaging** — installer, addon zip, diagnostics, backups, release docs.

## Non-negotiable rules

- Retail + Windows 11 + EU first; extensible to other regions.
- Lua addon / Tauri 2 shell / React+TS+Vite frontend / Rust core / SQLite.
- Analytical assistant only. NEVER: automate movement/input/gathering/looting/casting/
  mounting/interaction; read/inject/hook game memory; bypass protected APIs, anti-cheat,
  or Blizzard restrictions; fake realtime communication.
- Where realtime is unavailable: honest asynchronous sync with visible freshness.
- React = presentation. Rust = business logic. SQLite = durable state. Lua = in-game only.
- No `invoke()` scattered in components — typed API wrappers in `apps/desktop/src/lib/`.
- External dependencies (GatherMate2, Routes, TomTom, TSM, future providers) behind adapters.
- Core domain entities never depend on provider formats.

## Quality bar

- Deterministic algorithms where seeded; no mystery constants (config + weights explicit).
- Conservative economic estimates over inflated GPH. No false precision (P10/median/P90).
- Every valuation, route, and recommendation explainable.
- Freshness (`Fresh|Aging|Stale|Unavailable|Unknown`) and confidence (measured factors) first-class.
- Tests required: Rust (core), frontend (vitest), Lua (mocked-WoW harness), schema (SQL runner).
- Phase N is done only when its checks pass: `cargo fmt/check/test`, typecheck, vitest, build.

## Out of scope (never build)

Auction-house automation, botting, gameplay automation, trading bots, AI chat, account/guild
platforms, raid/DPS/talent tools, WoW encyclopedia, cloud account systems.

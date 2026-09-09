# DECISIONS (architectural decision records)

## D01 — Split `wowgoldfarm-core` from the Tauri shell
Pure-Rust domain crate with no GUI deps; Tauri crate holds thin commands + HTTP.
Rationale: domain logic unit-testable on any OS/CI without webkit/gtk; keeps the
"Rust owns business logic" boundary enforceable. Date: 2026-09-09.

## D02 — File-based SQL migrations embedded with `include_str!`
Each `crates/core/migrations/NNNN_name.sql` is both human-readable and embedded.
Rationale: single source of truth; Python sqlite3 can execute the same files to
validate schema in environments without a Rust toolchain. Sequential, never mutated.

## D03 — Honest async sync, no fake realtime
Addon persists versioned SavedVariables; desktop reads the SV file on user action;
desktop→addon route transfer uses an in-game import string (`/go import <code>`).
Rationale: WoW flushes SV only on logout/reload; any "realtime link" claim would be
false. Freshness (file mtime + embedded export timestamp) is always displayed.

## D04 — Sync core, async shell edge
Core is synchronous (rusqlite, pure math); Tauri commands wrap slow work
(market fetch, heavy route gen) in `tauri::async_runtime::spawn` + channels.
Rationale: simpler core, no tokio dependency in domain; UI never blocks.

## D05 — HTTP via `reqwest` (default-tls) in the shell behind `HttpFetcher` trait
Core defines the trait; tests use mock fetchers. Rationale: TLS via Windows
schannel needs no extra system deps; core stays network-agnostic and testable.

## D06 — Money as integer copper; floats only for quantities/rates
Rationale: exact money math, no rounding drift in valuations/GPH.

## D07 — Frontend: zustand + hand-rolled SVG charts, no react-query/recharts
Rationale: full control over caching/freshness semantics; fewer deps; restrained
analytical visuals per spec. Pinned: vite 6.4.3, TS 5.9.3, vitest 3.2.7,
@tauri-apps/api 2.11.1, react 19, react-router 7.

## D08 — Browser-safe mock provider for the UI
`lib/tauri.ts` detects the Tauri runtime; without it (vite preview, tests) the UI
runs against a typed mock backend with an explicit banner. Rationale: UI developable
and reviewable without the desktop shell; no `localhost` calls from browser code.

## D09 — Lua 5.1-compatible addon + mocked-WoW test harness
Addon targets WoW's Lua 5.1 dialect; `tests/lua/` runs all modules against a mock
WoW API with stock Lua 5.1. Rationale: validates addon logic outside the game client.

## D10 — Independent numeric cross-checks
`scripts/verify-algorithms.py` reimplements key math from ALGORITHMS.md and checks
`fixtures/expected/*.json` (also asserted by Rust tests). Rationale: catches
transcription errors; documents expected values where Rust can't run locally.

## D11 — Import-string route transfer as primary addon-bound channel
Compact `GO1.<map>.<n>.<coords>.<check>` payload (decimal triplets, mod-997
checksum) parsed by `/go import`. Chosen over base64 packing for debuggability
and trivial reimplementation on both sides.
Rationale: addons cannot read arbitrary files; SV-file patching while WoW runs is
unsafe. Optional offline SV merge (WoW closed, with backup) is secondary.

## D12 — Crowd is local-first optional
Interfaces + schemas + local aggregation ship; remote endpoint is config, default
disabled. Rationale: no gratuitous cloud; MVP fully functional offline.

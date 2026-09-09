# TESTING

## Rust (`crates/core`) — runs in CI + on dev machines with Rust
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test -p wowgoldfarm-core`.
- Unit tests per module + integration tests in `crates/core/tests/`:
  migrations round-trip, bridge import, atlas seed load, route pipeline determinism,
  2-opt improvement, simulation bands, market parse/valuation/spike/clamp,
  cooldown/backoff, session GPH math, intelligence blending/decay/min-samples,
  crowd aggregation/outlier rejection, diagnostics report shape.
- Fixtures under `fixtures/`; expected numeric outputs under `fixtures/expected/`
  (shared with the Python cross-check).

## Frontend (`apps/desktop`) — runs here (Node 22)
- `npm run typecheck` (tsc -b), `npm run test` (vitest run), `npm run build`.
- Tests: API wrappers (mocked `invoke`), `invokeSafe` fallback, formatting utils,
  stores, and page states (loading/empty/error/stale) via Testing Library.
- No network in tests; no `localhost` calls from browser code.

## Lua addon — runs here (stock Lua 5.1 built from source)
- `tests/lua/run_tests.lua` + `tests/lua/mock_wow.lua`: loads every addon module
  against a mocked WoW API, exercises init/session/loot/import/adapters/UI.
- `luac -p` syntax gate on all `addon/**/*.lua` (Lua 5.1 dialect only).

## Schema — runs here (Python 3 + stdlib sqlite3)
- `scripts/validate-schema.py`: executes `crates/core/migrations/*.sql` in order on
  a temp DB, asserts `user_version`, tables, indexes, FK enforcement, and seed load.

## Algorithm cross-check — runs here (Python 3, no deps)
- `scripts/verify-algorithms.py`: independent implementation of core math from
  ALGORITHMS.md; asserts `fixtures/expected/*.json`.

## Integration paths validated
- Addon → sync → parser → SQLite → domain → frontend (fixtures + bridge tests).
- Market → valuation → route → simulation → recommendation (pipeline tests).
- Session → history → learning → future route score (intelligence tests).

## What runs where
| suite    | sandbox (this env) | CI (ubuntu) | CI (windows) |
|----------|--------------------|-------------|--------------|
| frontend | ✅                 | ✅          | ✅ (tauri build) |
| lua      | ✅                 | ✅          | —            |
| schema   | ✅                 | ✅          | —            |
| py x-check | ✅               | ✅          | —            |
| cargo    | ❌ (no toolchain; hosts blocked) | ✅ | ✅ |
| tauri build | ❌             | —           | ✅           |

Rust/Tauri validation is delegated to CI with pinned toolchains; every Rust test
assertion is additionally cross-checked against Python-computed fixtures.

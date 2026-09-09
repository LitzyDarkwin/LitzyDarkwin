# Task 02 — Phase 2 desktop foundation

## Exit criteria
- [x] core `settings` (typed get/patch, validation) + unit tests
- [x] core `lua` (SV subset parser: tables/strings/numbers/bools/comments, caps) + tests
- [x] core `atlas` ensure-helpers (categories/zones/resources) + tests
- [x] core `bridge` (WoW validate, SV sync, session upsert, observation snapshot) + tests
- [x] core `diagnostics` (degrading report) + tests
- [x] integration test `bridge_import` (fake WoW → sync → SQLite → idempotent resync)
- [x] shell commands: detect/set/sync/state/settings/diagnostics
- [x] React Settings page (WoW+sync, market, privacy, diagnostics) + mocks + tests
- [x] frontend: typecheck + 27 vitest + vite build PASS
- [x] rust syntax gate: 21/21 files PASS (full cargo suite runs in CI)

# Task 03 — Phase 3 atlas

## Exit criteria
- [x] deterministic seed generator (`scripts/gen-seed.py`, seed=20260909) + committed `atlas.json`
- [x] `seed::load` (idempotent upserts, snapshot observations, placeholder merges) + tests
- [x] atlas query service (tree, search, zone detail, grid observations) + tests
- [x] shell commands + seed-on-startup in `init_state`
- [x] React Atlas (browser, search, filters, SVG density map, route handoff) + mocks + tests
- [x] `fixtures/expected/geo.json` cross-check green
- [x] frontend: typecheck + 41 vitest + vite build PASS
- [x] rust syntax gate PASS (full cargo suite runs in CI)

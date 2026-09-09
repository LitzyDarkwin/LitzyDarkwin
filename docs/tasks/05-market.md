# Task 05 — Phase 5 market engine

## Exit criteria
- [x] migration 0007: market_items ⇄ real TSM columns, provider_state PK fix
- [x] `market.rs`: dependency-free CSV, strict headers, ISO-8601, transactional ingest, §5 valuation, queries, price bridge + 13 tests
- [x] 5 shell commands (`refresh_market` … `get_market_prices`, reqwest blocking)
- [x] fixtures: `tsm_*_sample.csv` + `expected/valuation.json` (70 x-checks green)
- [x] React Market page (status, refresh, search, valuation detail) + Routes fill-from-market + mocks
- [x] frontend: typecheck + 62 vitest + vite build PASS
- [x] ALGORITHMS.md §5 + PROVIDERS.md + DATA_MODEL.md match the implementation
- [x] rust syntax gate + schema validator PASS (full cargo suite runs in CI)

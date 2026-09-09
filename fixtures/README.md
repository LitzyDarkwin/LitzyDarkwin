# fixtures/

Synthetic, deterministic test data shared by Rust tests, the Lua harness, and
the Python cross-checks. Nothing here is real player data or real market data.

- `tsm_commodities_sample.csv` — Phase 5: trimmed TSM commodities sample.
- `tsm_region_items_sample.csv` — Phase 5: trimmed TSM region-items sample.
- `savedvariables_sample.lua` — Phase 2/6: sample `GatherOptimizerDB` export.
- `observations_sample.json` — Phase 4: small deterministic observation set.
- `expected/*.json` — Phase 4+: independently computed expected outputs
  (see `scripts/verify-algorithms.py`).

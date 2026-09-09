# Task 07 — Phase 7 personal adaptive intelligence

## Exit criteria
- [x] schema v8: `farm_session_items.gathers` (0008) + bridge counts per-item gathers
- [x] bridge: `rebuild_personal` after every sync (same transaction)
- [x] `intelligence.rs`: §7 decay/weight/blend, full-rebuild learning, profile reads,
      yield personalization, zone efficiency, personalized pipeline + 7 tests
- [x] routes: `generate_from_observations` seam + `PersonalAdjustment` on candidates
- [x] 3 shell commands (`get_personal_profile`, `rebuild_personal`,
      `generate_personalized_routes`)
- [x] fixtures: `expected/blend.json` (115 x-checks green)
- [x] React Intelligence page (metrics, zones, resources, rebuild) + Routes
      personalize toggle with explain lines + mocks
- [x] frontend: typecheck + 79 vitest + vite build PASS
- [x] ALGORITHMS.md §7 implementation notes
- [x] rust syntax gate PASS (full cargo suite runs in CI)
- [x] fixed latent FK/NOT NULL test-seeder failures in routes/sessions tests

## Notes
- Learning is uniform-sum SQL + read-time decay (no stored drift); efficiency
  only ever compares linked sessions; unpriced loot excluded, never zeroed.
- `hit_rate`, travel speed, gather time stay priors — the addon persists no
  attempts or loop counts yet.

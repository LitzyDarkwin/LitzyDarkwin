# PROGRESS

- [x] Phase 0 — Architecture & repository foundation (checkpoint 2026-09-09)
- [x] Phase 1 — GatherOptimizer WoW addon (checkpoint 2026-09-09)
- [x] Phase 2 — Desktop application foundation (checkpoint 2026-09-09)
- [ ] Phase 3 — Farming Atlas
- [ ] Phase 4 — Economic route engine
- [ ] Phase 5 — Market engine
- [ ] Phase 6 — Live farm session system
- [ ] Phase 7 — Adaptive personal intelligence
- [ ] Phase 8 — Optional crowd intelligence
- [ ] Phase 9 — Polish, performance, release

## Current phase
Phase 0 — see CURRENT_TASK.md.

## Known defects / risks
- R1: Sandbox has no Rust toolchain (registry hosts blocked) → Rust/Tauri suites
  run in CI; mitigated by Python schema + algorithm cross-checks (see TESTING.md).
- R2: WoW TOC interface version is volatile → pinned `120100` (verified 2026-09-09
  against Warcraft Wiki TOC examples); re-verify before release packaging.
- R3: TSM CSV column sets may drift → parser is header-driven + skip-and-count;
  provider health recorded per refresh.

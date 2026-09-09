# wowgoldfarm

World of Warcraft Retail **farming intelligence** platform: what to farm, where,
which route, and what realistic gold/hour to expect — for you, right now.

> Analytical assistant only. It never automates gameplay, input, movement, or
> gathering, and never touches the WoW process. See "Safety boundaries" below.

## Components

| Part | Stack | Path |
|------|-------|------|
| GatherOptimizer addon | Lua (5.1) | `addon/GatherOptimizer/` |
| Desktop app | Tauri 2 + React + TS + Rust + SQLite | `apps/desktop/` |
| Domain core | Rust (`wowgoldfarm-core`) | `crates/core/` |
| Docs (persistent project memory) | Markdown | `docs/` |

## Quick start

```sh
# Desktop frontend (browser preview with mock data, or Tauri webview)
cd apps/desktop && npm ci && npm run dev      # http://localhost:1420

# Checks that run without a Rust toolchain
python3 scripts/validate-schema.py            # SQLite migrations
python3 scripts/verify-algorithms.py          # algorithm cross-check (Phase 4+)
scripts/check-lua.sh                          # addon syntax (Phase 1+)

# Full checks (need Rust stable; run in CI too)
cargo fmt --check && cargo test -p wowgoldfarm-core
cd apps/desktop && npm run typecheck && npm run test && npm run build
```

Windows installer and addon packaging: `scripts/` (Phase 9).

## Safety boundaries

- No automation of movement, input, gathering, looting, casting, mounting, NPC/object
  interaction. No memory reading, injection, hooks, or protected-API bypass.
- Addon uses public WoW APIs/events only; optional integrations (GatherMate2, Routes,
  TomTom) are feature-detected and never required.
- Desktop↔addon sync is honest async file sync (SavedVariables) plus an in-game
  import string for routes — freshness is always displayed, never faked.

## Documentation map

`docs/MASTER.md` (requirements) · `ARCHITECTURE.md` · `ALGORITHMS.md` ·
`DATA_MODEL.md` · `DECISIONS.md` · `PROGRESS.md` · `CURRENT_TASK.md` ·
`BUILD_NOTES.md` · `TESTING.md` · `PROVIDERS.md` · `SECURITY.md` · `tasks/`

## License

MIT (project code). World of Warcraft is a trademark of Blizzard Entertainment;
this is an independent tool with no affiliation.

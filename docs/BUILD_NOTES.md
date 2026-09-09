# BUILD NOTES

## Toolchain (pinned)
- Rust: stable ≥1.77 (CI: `stable`), `cargo fmt`, `clippy`.
- Node 22, npm (lockfile committed). Frontend pins: vite 6.4.3, TS 5.9.3,
  vitest 3.2.7, @tauri-apps/api 2.11.1, @tauri-apps/cli 2.11.1, react 19.2.8,
  react-router-dom 7.18.3, zustand 5.0.15.
- Lua 5.1 (addon dialect; CI installs `lua5.1`; sandbox builds from
  `https://github.com/lua/lua` tag `v5.1.5`).
- Python 3.11+ stdlib only (schema + algorithm checks).

## Commands
```sh
# frontend (apps/desktop)
npm ci
npm run dev            # vite on :1420 (mock backend when no Tauri runtime)
npm run typecheck
npm run test           # vitest run
npm run build          # → dist/ (also used by tauri)

# core (needs Rust toolchain; runs in CI)
cargo fmt --check
cargo clippy -p wowgoldfarm-core -- -D warnings
cargo test -p wowgoldfarm-core

# desktop shell (Windows)
npm run tauri:build    # vite build + tauri build

# addon checks (no WoW client needed)
scripts/check-lua.sh
lua5.1 tests/lua/run_tests.lua

# schema + algorithms (no Rust needed)
python3 scripts/validate-schema.py
python3 scripts/verify-algorithms.py
```

## Environment assumptions
- Windows 11 target: WebView2 (Tauri 2 prerequisite), MSVC toolchain for builds.
- WoW Retail installed with `_retail_/Interface/AddOns/` writable by the user.
- Addon SV flush requires logout/reload — the UI states this explicitly.
- App data dir: `%APPDATA%/com.wowgoldfarm.app/` (SQLite + logs + market cache).
- Network: HTTPS to `public-data.tradeskillmaster.com` for market refresh;
  everything else works offline.

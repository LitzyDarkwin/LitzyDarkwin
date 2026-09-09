# SECURITY

## Boundaries
- **No game interaction beyond the addon sandbox**: no memory read/injection/hooks,
  no input simulation, no protected-API bypass. Addon uses public events/APIs only.
- **Tauri shell**: no raw FS/network plugins exposed to the frontend. All file and
  network access happens in Rust commands with path validation (WoW dir + app-data
  dir allowlist) and HTTPS-only market URLs (host allowlist:
  `public-data.tradeskillmaster.com`, configurable crowd host when enabled).
- **No secrets in frontend bundles**: there are no API keys by design (TSM public
  data). If a future provider needs keys, they live in OS-secure storage via Rust,
  never in JS.

## Input handling (all untrusted)
- Market CSV: bounded size (default 64 MiB/dataset), row-count caps, per-field
  validation, skip-and-count malformed rows, never `panic` on parse.
- Addon SV lua: size cap (default 16 MiB), subset-parser with depth/length limits,
  schema-version gate.
- Import strings, deep-pasted data: length caps + checksum validation.
- SQLite: parameterized queries only; `PRAGMA journal_mode=WAL`, `foreign_keys=ON`.

## Privacy
- Local-first: farming history stays in local SQLite. Crowd sharing is opt-in,
  default OFF, with coarse buckets (no precise trails, no chat, no identifiers;
  rotating daily client hash, no account/character linkage).
- Diagnostics export excludes paths/characters beyond first-use necessity and never
  includes credentials (there are none to include).

## Supply chain / release
- Pinned frontend deps (`package-lock.json` committed); Rust deps versioned in
  `Cargo.lock` (committed). CI builds on `windows-latest` for the installer.
- CodeQL-friendly: no `unwrap`/`expect`/`panic!` on production paths (see audit);
  `cargo clippy -D warnings` in CI.

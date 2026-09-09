#!/usr/bin/env bash
# Syntax-check all addon Lua with a Lua 5.1 compiler (WoW's addon dialect).
# Prefers luac5.1/luac on PATH, else $LUA51_DIR/luac (sandbox-built interpreter).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

mapfile -t FILES < <(find "$ROOT/addon" -name '*.lua' 2>/dev/null || true)
if [ "${#FILES[@]}" -eq 0 ]; then
  echo "check-lua: no Lua sources yet (pre-Phase-1) — nothing to check"
  exit 0
fi

LUAC=""
for c in luac5.1 luac; do
  if command -v "$c" >/dev/null 2>&1; then LUAC="$c"; break; fi
done
if [ -z "$LUAC" ] && [ -n "${LUA51_DIR:-}" ] && [ -x "$LUA51_DIR/luac" ]; then
  LUAC="$LUA51_DIR/luac"
fi
if [ -z "$LUAC" ]; then
  echo "check-lua: FAIL — no Lua 5.1 compiler found (need luac5.1, luac, or \$LUA51_DIR/luac)" >&2
  exit 1
fi

echo "check-lua: using $LUAC"
FAIL=0
for f in "${FILES[@]}"; do
  if "$LUAC" -p "$f"; then
    echo "ok  $f"
  else
    echo "FAIL  $f" >&2
    FAIL=1
  fi
done
if [ "$FAIL" -ne 0 ]; then echo "check-lua: FAILURES" >&2; exit 1; fi
echo "check-lua: PASS (${#FILES[@]} files)"

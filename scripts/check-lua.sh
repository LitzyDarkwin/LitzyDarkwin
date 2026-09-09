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

# Fallback: stock lua 5.1 via loadfile() (also validates 5.1 dialect).
LUA=""
for c in lua5.1 lua; do
  if command -v "$c" >/dev/null 2>&1; then LUA="$c"; break; fi
done
if [ -z "$LUA" ] && [ -x "$ROOT/.tools/lua51-src/lua" ]; then
  LUA="$ROOT/.tools/lua51-src/lua"
fi

if [ -z "$LUAC" ] && [ -z "$LUA" ]; then
  echo "check-lua: FAIL — no Lua 5.1 compiler/interpreter found" >&2
  exit 1
fi

check_with_luac() { "$LUAC" -p "$1"; }
check_with_lua() {
  LUA_CHECK_FILE="$1" "$LUA" -e 'local f=os.getenv("LUA_CHECK_FILE"); local fn,err=loadfile(f); if not fn then io.stderr:write(f..": "..(err or "?").."\n") os.exit(1) end'
}

if [ -n "$LUAC" ]; then echo "check-lua: using $LUAC"; else echo "check-lua: using $LUA (loadfile mode)"; fi
FAIL=0
for f in "${FILES[@]}"; do
  if [ -n "$LUAC" ]; then check_with_luac "$f"; else check_with_lua "$f"; fi
  if [ $? -eq 0 ]; then
    echo "ok  $f"
  else
    echo "FAIL  $f" >&2
    FAIL=1
  fi
done
if [ "$FAIL" -ne 0 ]; then echo "check-lua: FAILURES" >&2; exit 1; fi
echo "check-lua: PASS (${#FILES[@]} files, Lua 5.1 dialect)"

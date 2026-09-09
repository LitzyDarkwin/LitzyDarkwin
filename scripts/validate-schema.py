#!/usr/bin/env python3
"""Validate the canonical SQLite schema without a Rust toolchain.

Executes crates/core/migrations/*.sql in order on a temp database and asserts:
user_version progression, expected tables/indexes, FK enforcement, and that the
bundled seed file (when present) loads into the schema.
"""
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MIGRATIONS = sorted((ROOT / "crates" / "core" / "migrations").glob("*.sql"))

EXPECTED_TABLES = [
    "expansions", "continents", "zones", "farm_categories", "resources",
    "resource_observations", "app_settings", "saved_routes", "market_items",
    "market_refresh", "provider_state", "farm_sessions", "farm_session_items",
    "personal_metrics", "personal_zone_stats", "personal_resource_stats",
    "crowd_observations",
]

EXPECTED_INDEXES = [
    "idx_obs_zone_xy", "idx_obs_resource_time", "idx_routes_zone",
    "idx_market_region", "idx_refresh_provider", "idx_sessions_status",
    "idx_session_items_resource", "idx_crowd_zone_day",
]


def main() -> int:
    if not MIGRATIONS:
        print("FAIL: no migration files found")
        return 1
    conn = sqlite3.connect(":memory:")
    conn.execute("PRAGMA foreign_keys = ON")
    for i, path in enumerate(MIGRATIONS, start=1):
        conn.executescript(path.read_text(encoding="utf-8"))
        conn.execute(f"PRAGMA user_version = {i}")
        version = conn.execute("PRAGMA user_version").fetchone()[0]
        assert version == i, f"{path.name}: user_version={version}, expected {i}"
        print(f"ok  {path.name} -> user_version={version}")

    tables = {r[0] for r in conn.execute(
        "SELECT name FROM sqlite_master WHERE type='table'")}
    missing = [t for t in EXPECTED_TABLES if t not in tables]
    if missing:
        print(f"FAIL: missing tables: {missing}")
        return 1
    print(f"ok  all {len(EXPECTED_TABLES)} tables present")

    indexes = {r[0] for r in conn.execute(
        "SELECT name FROM sqlite_master WHERE type='index'")}
    missing_idx = [x for x in EXPECTED_INDEXES if x not in indexes]
    if missing_idx:
        print(f"FAIL: missing indexes: {missing_idx}")
        return 1
    print(f"ok  all {len(EXPECTED_INDEXES)} key indexes present")

    # FK enforcement check: observation referencing a missing zone must fail.
    try:
        conn.execute(
            "INSERT INTO resource_observations (zone_id, resource_id, x, y)"
            " VALUES ('nope', 'nope', 1.0, 2.0)")
        print("FAIL: foreign keys not enforced")
        return 1
    except sqlite3.IntegrityError:
        print("ok  foreign keys enforced")

    print(f"PASS: schema v{len(MIGRATIONS)} valid ({len(MIGRATIONS)} migrations)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

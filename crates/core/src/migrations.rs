//! Sequential migration runner. Each migration is a `migrations/*.sql` file
//! embedded at compile time — the same files validated by
//! `scripts/validate-schema.py`. Migrations are never mutated after release.

use crate::error::Result;
use crate::storage::{set_user_version, user_version};
use rusqlite::Connection;

/// Ordered (version, name, sql). Version N must equal its 1-based position.
const MIGRATIONS: &[(u32, &str, &str)] = &[
    (1, "0001_core", include_str!("../migrations/0001_core.sql")),
    (2, "0002_routes", include_str!("../migrations/0002_routes.sql")),
    (3, "0003_market", include_str!("../migrations/0003_market.sql")),
    (4, "0004_sessions", include_str!("../migrations/0004_sessions.sql")),
    (5, "0005_intelligence", include_str!("../migrations/0005_intelligence.sql")),
    (6, "0006_crowd", include_str!("../migrations/0006_crowd.sql")),
];

/// Latest schema version supported by this build.
pub fn latest_version() -> u32 {
    MIGRATIONS.last().map(|m| m.0).unwrap_or(0)
}

/// Apply all pending migrations inside one transaction per migration.
/// Idempotent: already-applied versions are skipped via `user_version`.
/// Returns the list of versions actually applied.
pub fn migrate(conn: &mut Connection) -> Result<Vec<u32>> {
    let current = user_version(conn)?;
    let latest = latest_version();
    if current > latest {
        return Err(crate::error::AppError::Storage(rusqlite::Error::InvalidParameterName(
            format!("database schema v{current} is newer than supported v{latest}"),
        )));
    }
    let mut applied = Vec::new();
    for (version, name, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql).map_err(|e| {
            crate::error::AppError::provider(format!("migration {name} failed: {e}"))
        })?;
        tx.pragma_update(None, "user_version", *version)?;
        tx.commit()?;
        applied.push(*version);
    }
    // Keep the in-memory constant honest in debug builds.
    debug_assert_eq!(latest_version(), crate::SCHEMA_VERSION);
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::open_memory;

    #[test]
    fn migrates_from_zero_to_latest() {
        let mut conn = open_memory().unwrap();
        let applied = migrate(&mut conn).unwrap();
        assert_eq!(applied, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(user_version(&conn).unwrap(), latest_version());
    }

    #[test]
    fn migrate_is_idempotent() {
        let mut conn = open_memory().unwrap();
        migrate(&mut conn).unwrap();
        assert!(migrate(&mut conn).unwrap().is_empty());
    }

    #[test]
    fn rejects_newer_database() {
        let mut conn = open_memory().unwrap();
        set_user_version(&conn, 999).unwrap();
        assert!(migrate(&mut conn).is_err());
    }

    #[test]
    fn all_expected_tables_exist() {
        let mut conn = open_memory().unwrap();
        migrate(&mut conn).unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map(rusqlite::params![], |r| r.get(0))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        for expected in [
            "expansions",
            "continents",
            "zones",
            "farm_categories",
            "resources",
            "resource_observations",
            "app_settings",
            "saved_routes",
            "market_items",
            "market_refresh",
            "provider_state",
            "farm_sessions",
            "farm_session_items",
            "personal_metrics",
            "personal_zone_stats",
            "personal_resource_stats",
            "crowd_observations",
        ] {
            assert!(tables.iter().any(|t| t == expected), "missing table {expected}");
        }
    }
}

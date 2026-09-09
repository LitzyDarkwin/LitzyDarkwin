//! Health snapshots and diagnostic reports.
//!
//! `collect()` never fails on missing/partial data: every probe degrades to a
//! warning so the report itself is always available when the DB opens.

use crate::bridge;
use crate::error::Result;
use crate::migrations;
use crate::models::{policies, Freshness};
use crate::settings;
use crate::storage;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Point-in-time health report. Safe to display and export (no secrets exist
/// by design; paths are local-only).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub app_version: String,
    pub schema_version: u32,
    pub migration_state: String,
    pub db_path: String,
    pub db_size_bytes: u64,
    pub wow_dir: Option<String>,
    pub wow_detected: bool,
    pub addon_detected: bool,
    pub addon_version: Option<String>,
    pub last_sync_at: Option<i64>,
    pub sync_freshness: Freshness,
    pub market_provider: String,
    pub market_freshness: Freshness,
    pub last_market_error: Option<String>,
    pub session_count: i64,
    pub observation_count: i64,
    pub route_count: i64,
    pub warnings: Vec<String>,
}

fn count(conn: &Connection, table: &str) -> i64 {
    // Table names are internal constants, never user input.
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), rusqlite::params![], |r| {
        r.get(0)
    })
    .unwrap_or(0)
}

pub fn collect(
    conn: &Connection,
    app_version: &str,
    db_path: &Path,
    now: i64,
) -> Result<DiagnosticReport> {
    let mut warnings: Vec<String> = Vec::new();
    let schema_version = storage::user_version(conn).unwrap_or(0);
    let latest = migrations::latest_version();
    let migration_state = if schema_version == latest {
        format!("v{schema_version} (latest)")
    } else if schema_version < latest {
        warnings.push(format!("database schema v{schema_version} is behind v{latest}"));
        format!("v{schema_version} (behind latest v{latest})")
    } else {
        warnings.push(format!("database schema v{schema_version} is newer than supported v{latest}"));
        format!("v{schema_version} (NEWER than supported v{latest})")
    };

    let cfg = settings::get_settings(conn).unwrap_or_default();
    let db_size_bytes = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);

    let mut wow_detected = false;
    let mut addon_detected = false;
    let mut addon_version = None;
    match cfg.wow_dir.as_deref() {
        None => warnings.push("WoW directory is not configured".to_string()),
        Some(dir) => match bridge::validate_wow_dir(Path::new(dir)) {
            Ok(d) => {
                wow_detected = true;
                addon_detected = d.addon_installed;
                addon_version = d.addon_version;
                if !d.addon_installed {
                    warnings.push("GatherOptimizer addon not found in Interface/AddOns".to_string());
                }
                if d.accounts.is_empty() {
                    warnings.push("no addon SavedVariables files found (log into WoW, then log out)".to_string());
                }
            }
            Err(e) => warnings.push(format!("WoW directory invalid: {}", e.user_message())),
        },
    }

    let sync = bridge::get_sync_state(conn, now).unwrap_or(bridge::SyncStateInfo {
        last_sync_at: None,
        last_export_at: None,
        last_result: None,
        freshness: Freshness::Unknown,
    });
    if sync.last_sync_at.is_none() {
        warnings.push("never synced with the addon".to_string());
    }

    let market_fetched: Option<i64> = conn
        .query_row("SELECT MAX(fetched_at) FROM market_items", rusqlite::params![], |r| r.get(0))
        .unwrap_or(None);
    let market_freshness = policies::MARKET.classify(market_fetched, now);
    let last_market_error: Option<String> = conn
        .query_row(
            "SELECT error FROM market_refresh WHERE status = 'failed' ORDER BY started_at DESC LIMIT 1",
            rusqlite::params![],
            |r| r.get(0),
        )
        .unwrap_or(None);
    if market_fetched.is_none() {
        warnings.push("no market data cached yet".to_string());
    }

    Ok(DiagnosticReport {
        app_version: app_version.to_string(),
        schema_version,
        migration_state,
        db_path: db_path.to_string_lossy().into_owned(),
        db_size_bytes,
        wow_dir: cfg.wow_dir,
        wow_detected,
        addon_detected,
        addon_version,
        last_sync_at: sync.last_sync_at,
        sync_freshness: sync.freshness,
        market_provider: cfg.market_provider,
        market_freshness,
        last_market_error,
        session_count: count(conn, "farm_sessions"),
        observation_count: count(conn, "resource_observations"),
        route_count: count(conn, "saved_routes"),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage;

    #[test]
    fn collect_degrades_gracefully_on_empty_db() {
        let mut conn = storage::open_memory().unwrap();
        migrations::migrate(&mut conn).unwrap();
        let report = collect(&conn, "0.1.0", Path::new("test.db"), 1_700_000_000).unwrap();
        assert_eq!(report.schema_version, migrations::latest_version());
        assert!(!report.wow_detected);
        assert_eq!(report.sync_freshness, Freshness::Unknown);
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn counts_reflect_stored_rows() {
        let mut conn = storage::open_memory().unwrap();
        migrations::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO farm_sessions (id, started_at) VALUES ('s1', 1)",
            rusqlite::params![],
        )
        .unwrap();
        let report = collect(&conn, "0.1.0", Path::new("test.db"), 1_700_000_000).unwrap();
        assert_eq!(report.session_count, 1);
        assert_eq!(report.observation_count, 0);
    }
}

//! Addon synchronization orchestration.
//!
//! Honest async boundary: the desktop reads the addon's SavedVariables file on
//! user action (WoW flushes it only on logout/reload). This module detects the
//! WoW install, parses SV files, validates versions, and upserts sessions and
//! observations into SQLite.
//!
//! Sync semantics:
//! - Sessions are incremental, deduplicated by addon session id (upsert).
//! - Observations are a snapshot: the addon ring buffer IS the full dataset,
//!   so `source='addon'` rows are replaced wholesale per sync, in one txn.

use crate::atlas;
use crate::error::{AppError, Result};
use crate::lua::parse_saved_variables;
use crate::models::{policies, source, Freshness};
use crate::settings::{self, keys};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const SV_VAR_NAME: &str = "GatherOptimizerDB";
pub const SV_FILE_NAME: &str = "GatherOptimizer.lua";
/// Refuse to read SV files larger than this (corruption/DoS guard).
pub const MAX_SV_BYTES: u64 = 16 * 1024 * 1024;
/// Placeholder item id for name-only loot rows (no `|Hitem|` in the message).
pub const UNKNOWN_ITEM_ID: i64 = 0;

/// One account's SavedVariables file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSv {
    pub account: String,
    pub path: String,
    pub size_bytes: u64,
    /// File mtime as Unix seconds (0 when unreadable).
    pub mtime: i64,
}

/// Result of validating a WoW installation directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WowDetection {
    pub wow_dir: String,
    pub retail_dir: String,
    pub addons_dir: String,
    pub addon_installed: bool,
    pub addon_version: Option<String>,
    pub addon_interface: Option<String>,
    pub accounts: Vec<AccountSv>,
}

/// Per-file sync accounting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFileReport {
    pub account: String,
    pub path: String,
    pub export_timestamp: Option<i64>,
    pub sessions_seen: usize,
    pub sessions_imported: usize,
    pub sessions_updated: usize,
    pub sessions_skipped: usize,
    pub observations_seen: usize,
    pub observations_imported: usize,
    pub observations_skipped: usize,
    pub warnings: Vec<String>,
}

/// Whole-sync report persisted to settings (`sync.*`) and returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub synced_at: i64,
    pub files: Vec<SyncFileReport>,
    pub sessions_imported: usize,
    pub sessions_updated: usize,
    pub sessions_skipped: usize,
    pub observations_imported: usize,
    pub warnings: Vec<String>,
}

/// Last-sync state for freshness display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStateInfo {
    pub last_sync_at: Option<i64>,
    pub last_export_at: Option<i64>,
    pub last_result: Option<String>,
    pub freshness: Freshness,
}

/// Default install locations to probe (Windows). Non-Windows builds return an
/// empty list: the path is configured manually there (dev scenario).
pub fn candidate_wow_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Ok(base) = std::env::var(var) {
                out.push(PathBuf::from(base).join("World of Warcraft"));
            }
        }
        out.push(PathBuf::from(r"C:\World of Warcraft"));
    }
    out
}

fn mtime_unix(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn parse_toc_field(toc_path: &Path, field: &str) -> Option<String> {
    let text = std::fs::read_to_string(toc_path).ok()?;
    let prefix = format!("## {field}:");
    for line in text.lines() {
        let line = line.trim_start_matches('\u{feff}').trim();
        if line.starts_with(&prefix) {
            let value = line[prefix.len()..].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Validate `wow_dir`: layout, addon presence/version, account SV files.
pub fn validate_wow_dir(wow_dir: &Path) -> Result<WowDetection> {
    if !wow_dir.is_dir() {
        return Err(AppError::bridge(format!(
            "`{}` is not a directory",
            wow_dir.to_string_lossy()
        )));
    }
    let retail = wow_dir.join("_retail_");
    if !retail.is_dir() {
        return Err(AppError::bridge(format!(
            "no `_retail_` folder under `{}` — is this the WoW install?",
            wow_dir.to_string_lossy()
        )));
    }
    let addons = retail.join("Interface").join("AddOns");
    let toc = addons.join("GatherOptimizer").join("GatherOptimizer.toc");
    let addon_installed = toc.is_file();
    let addon_version = addon_installed.then(|| parse_toc_field(&toc, "Version")).flatten();
    let addon_interface = addon_installed.then(|| parse_toc_field(&toc, "Interface")).flatten();

    let mut accounts = Vec::new();
    let accounts_dir = retail.join("WTF").join("Account");
    if let Ok(entries) = std::fs::read_dir(&accounts_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let sv = entry.path().join("SavedVariables").join(SV_FILE_NAME);
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) && sv.is_file() {
                let size = std::fs::metadata(&sv).map(|m| m.len()).unwrap_or(0);
                accounts.push(AccountSv {
                    account: name,
                    path: sv.to_string_lossy().into_owned(),
                    size_bytes: size,
                    mtime: mtime_unix(&sv),
                });
            }
        }
    }
    accounts.sort_by(|a, b| a.account.cmp(&b.account));
    Ok(WowDetection {
        wow_dir: wow_dir.to_string_lossy().into_owned(),
        retail_dir: retail.to_string_lossy().into_owned(),
        addons_dir: addons.to_string_lossy().into_owned(),
        addon_installed,
        addon_version,
        addon_interface,
        accounts,
    })
}

// JSON field helpers (SV tables decode to serde_json::Value) -------------------

fn str_field(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(|s| s.to_string())
}

fn i64_field(v: &serde_json::Value, key: &str) -> Option<i64> {
    v.get(key)?.as_i64()
}

fn f64_field(v: &serde_json::Value, key: &str) -> Option<f64> {
    let f = v.get(key)?;
    f.as_f64().or_else(|| f.as_i64().map(|i| i as f64))
}

/// Accept both JSON arrays and objects with all-numeric keys (mixed Lua tables).
fn as_sequence(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    match v {
        serde_json::Value::Array(items) => items.iter().collect(),
        serde_json::Value::Object(map) => {
            let mut keys: Vec<usize> = Vec::new();
            for k in map.keys() {
                match k.parse::<usize>() {
                    Ok(n) if n >= 1 => keys.push(n),
                    _ => return Vec::new(),
                }
            }
            keys.sort_unstable();
            keys.into_iter().filter_map(|n| map.get(&n.to_string())).collect()
        }
        _ => Vec::new(),
    }
}

fn upsert_session(
    conn: &Connection,
    s: &serde_json::Value,
    report: &mut SyncFileReport,
    now: i64,
) -> Result<()> {
    let Some(external_id) = str_field(s, "id") else {
        report.sessions_skipped += 1;
        report.warnings.push("session without id skipped".to_string());
        return Ok(());
    };
    let zone_id = atlas::ensure_zone_for_map(
        conn,
        i64_field(s, "mapID"),
        str_field(s, "zone").as_deref(),
    )?;
    let character = str_field(s, "character").unwrap_or_default();
    let realm = str_field(s, "realm").unwrap_or_default();
    let started = i64_field(s, "startedAt").unwrap_or(0);
    let ended = i64_field(s, "endedAt");
    let active = i64_field(s, "activeSeconds").unwrap_or(0);
    // Rows land as `imported`; analyze_session() below snapshots prices and
    // flips them to `analyzed` in the same transaction.
    let status = "imported";
    let notes = str_field(s, "note").unwrap_or_default();

    let existing: Option<(Option<i64>, i64)> = conn
        .query_row(
            "SELECT ended_at, active_seconds FROM farm_sessions WHERE id = ?1",
            rusqlite::params![external_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;

    // Aggregate gathers by item id (qty + event count for yield learning).
    let mut items: std::collections::BTreeMap<i64, (f64, String, i64)> = Default::default();
    for g in as_sequence(s.get("gathers").unwrap_or(&serde_json::Value::Null)) {
        let item = i64_field(g, "itemID").unwrap_or(UNKNOWN_ITEM_ID);
        let qty = f64_field(g, "qty").unwrap_or(1.0).max(0.0);
        let resource = atlas::ensure_resource_for_item(
            conn,
            if item == UNKNOWN_ITEM_ID { None } else { Some(item) },
            str_field(g, "name").as_deref(),
            str_field(g, "cat").as_deref(),
        )?;
        let entry = items.entry(item).or_insert((0.0, resource, 0));
        entry.0 += qty;
        entry.2 += 1;
    }

    match existing {
        None => {
            conn.execute(
                "INSERT INTO farm_sessions (id, external_id, character_name, realm, zone_id,
                 status, started_at, ended_at, active_seconds, notes, created_at, updated_at)
                 VALUES (?1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                rusqlite::params![
                    external_id, character, realm, zone_id, status, started, ended, active,
                    notes, started
                ],
            )?;
            report.sessions_imported += 1;
        }
        Some((prev_ended, prev_active)) if prev_ended != ended || prev_active != active => {
            conn.execute(
                "UPDATE farm_sessions SET character_name = ?2, realm = ?3, zone_id = ?4,
                 ended_at = ?5, active_seconds = ?6, notes = ?7, updated_at = ?8 WHERE id = ?1",
                rusqlite::params![
                    external_id, character, realm, zone_id, ended, active, notes, started
                ],
            )?;
            conn.execute("DELETE FROM farm_session_items WHERE session_id = ?1", rusqlite::params![external_id])?;
            report.sessions_updated += 1;
        }
        Some(_) => {
            report.sessions_skipped += 1;
            return Ok(());
        }
    }
    for (item_id, (qty, resource_id, gathers)) in &items {
        conn.execute(
            "INSERT INTO farm_session_items (session_id, item_id, resource_id, qty, gathers)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![external_id, item_id, resource_id, qty, gathers],
        )?;
    }
    // Snapshot market prices for the imported loot (first snapshot wins).
    let refresh_hours = settings::get_raw(conn, keys::MARKET_REFRESH_HOURS)
        .ok()
        .flatten()
        .and_then(|v| v.as_i64())
        .unwrap_or(12) as f64;
    let cfg = crate::market::ValuationConfig {
        refresh_hours: refresh_hours.clamp(1.0, 168.0),
        ..Default::default()
    };
    crate::sessions::analyze_session(conn, &external_id, &cfg, now)?;
    Ok(())
}

fn import_observations(
    conn: &Connection,
    root: &serde_json::Value,
    report: &mut SyncFileReport,
) -> Result<()> {
    let empty = serde_json::Value::Null;
    let list = as_sequence(root.get("observations").unwrap_or(&empty));
    report.observations_seen = list.len();
    for o in list {
        let (Some(x), Some(y)) = (f64_field(o, "x"), f64_field(o, "y")) else {
            report.observations_skipped += 1;
            continue;
        };
        if !(0.0..=100.0).contains(&x) || !(0.0..=100.0).contains(&y) {
            report.observations_skipped += 1;
            continue;
        }
        let zone_id = atlas::ensure_zone_for_map(
            conn,
            i64_field(o, "mapID"),
            str_field(o, "zone").as_deref(),
        )?;
        let item = i64_field(o, "itemID");
        let resource_id = atlas::ensure_resource_for_item(
            conn,
            item,
            str_field(o, "name").as_deref(),
            str_field(o, "cat").as_deref(),
        )?;
        conn.execute(
            "INSERT INTO resource_observations
             (zone_id, resource_id, map_id, x, y, source, yield_qty, observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                zone_id,
                resource_id,
                i64_field(o, "mapID"),
                x,
                y,
                source::ADDON,
                f64_field(o, "qty"),
                i64_field(o, "t").unwrap_or(0),
            ],
        )?;
        report.observations_imported += 1;
    }
    Ok(())
}

fn sync_one_file(conn: &Connection, file: &AccountSv, now: i64) -> Result<SyncFileReport> {
    let mut report = SyncFileReport {
        account: file.account.clone(),
        path: file.path.clone(),
        export_timestamp: None,
        sessions_seen: 0,
        sessions_imported: 0,
        sessions_updated: 0,
        sessions_skipped: 0,
        observations_seen: 0,
        observations_imported: 0,
        observations_skipped: 0,
        warnings: Vec::new(),
    };
    if file.size_bytes > MAX_SV_BYTES {
        report.warnings.push(format!(
            "file larger than {} MiB — skipped (not parsed)",
            MAX_SV_BYTES / 1024 / 1024
        ));
        return Ok(report);
    }
    let text = std::fs::read_to_string(&file.path).map_err(|e| {
        AppError::bridge(format!("cannot read {}: {e}", file.path))
    })?;
    if text.len() as u64 > MAX_SV_BYTES {
        report.warnings.push("file larger than cap after read — skipped".to_string());
        return Ok(report);
    }
    let root = parse_saved_variables(&text, SV_VAR_NAME)?;
    let schema = root.get("schemaVersion").and_then(serde_json::Value::as_u64).unwrap_or(0);
    if schema as u32 != crate::ADDON_SCHEMA_VERSION {
        report.warnings.push(format!(
            "unsupported addon schema v{schema} (want v{}) — file skipped; update the addon or app",
            crate::ADDON_SCHEMA_VERSION
        ));
        return Ok(report);
    }
    report.export_timestamp = root.get("export").and_then(|e| i64_field(e, "exportedAt"));

    let empty = serde_json::Value::Null;
    let sessions = as_sequence(root.get("sessions").unwrap_or(&empty));
    report.sessions_seen = sessions.len();
    for s in sessions {
        upsert_session(conn, s, &mut report, now)?;
    }
    import_observations(conn, &root, &mut report)?;

    Ok(report)
}

/// Run a full sync across all detected account files. One transaction.
pub fn sync_from_addon(
    conn: &mut Connection,
    detection: &WowDetection,
    now: i64,
) -> Result<SyncReport> {
    if detection.accounts.is_empty() {
        return Err(AppError::bridge(
            "no SavedVariables files found — log into WoW with the addon enabled, then log out or /reload",
        ));
    }
    let tx = conn.transaction()?;
    // Snapshot replace for addon observations (see module docs).
    tx.execute("DELETE FROM resource_observations WHERE source = ?1", rusqlite::params![source::ADDON])?;

    let mut report = SyncReport {
        synced_at: now,
        files: Vec::new(),
        sessions_imported: 0,
        sessions_updated: 0,
        sessions_skipped: 0,
        observations_imported: 0,
        warnings: Vec::new(),
    };
    let mut newest_export: Option<i64> = None;
    for file in &detection.accounts {
        let file_report = sync_one_file(&tx, file, now)?;
        if let Some(ts) = file_report.export_timestamp {
            newest_export = Some(newest_export.map_or(ts, |prev: i64| prev.max(ts)));
        }
        report.sessions_imported += file_report.sessions_imported;
        report.sessions_updated += file_report.sessions_updated;
        report.sessions_skipped += file_report.sessions_skipped;
        report.observations_imported += file_report.observations_imported;
        report.warnings.extend(file_report.warnings.clone());
        report.files.push(file_report);
    }

    let result = if report.warnings.is_empty() { "ok" } else { "warnings" };
    settings::set_raw(&tx, keys::SYNC_LAST_AT, &serde_json::Value::from(now), now)?;
    settings::set_raw(
        &tx,
        keys::SYNC_LAST_RESULT,
        &serde_json::Value::String(result.to_string()),
        now,
    )?;
    settings::set_raw(
        &tx,
        keys::SYNC_LAST_EXPORT_AT,
        &newest_export.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null),
        now,
    )?;
    // Learned stats follow the fresh sessions (same transaction, full rebuild).
    crate::intelligence::rebuild_personal(&tx, now)?;
    tx.commit()?;
    Ok(report)
}

/// Read last-sync state for freshness display.
pub fn get_sync_state(conn: &Connection, now: i64) -> Result<SyncStateInfo> {
    let last_sync_at = settings::get_raw(conn, keys::SYNC_LAST_AT)?.and_then(|v| v.as_i64());
    let last_export_at =
        settings::get_raw(conn, keys::SYNC_LAST_EXPORT_AT)?.and_then(|v| v.as_i64());
    let last_result =
        settings::get_raw(conn, keys::SYNC_LAST_RESULT)?.and_then(|v| v.as_str().map(str::to_string));
    let freshness = policies::ADDON_SYNC.classify(last_sync_at, now);
    Ok(SyncStateInfo { last_sync_at, last_export_at, last_result, freshness })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{migrations, storage};

    #[test]
    fn as_sequence_accepts_arrays_and_numeric_objects() {
        let arr: serde_json::Value = serde_json::from_str("[1,2]").unwrap();
        assert_eq!(as_sequence(&arr).len(), 2);
        let obj: serde_json::Value = serde_json::from_str(r#"{"1":"a","2":"b"}"#).unwrap();
        assert_eq!(as_sequence(&obj).len(), 2);
        let bad: serde_json::Value = serde_json::from_str(r#"{"x":1}"#).unwrap();
        assert!(as_sequence(&bad).is_empty());
    }

    #[test]
    fn validate_rejects_non_wow_dirs() {
        let dir = std::env::temp_dir().join("wgf-test-not-wow");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(validate_wow_dir(&dir.join("missing")).is_err());
        assert!(validate_wow_dir(&dir).is_err()); // no _retail_
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sync_requires_account_files() {
        let mut conn = storage::open_memory().unwrap();
        migrations::migrate(&mut conn).unwrap();
        let detection = WowDetection {
            wow_dir: String::new(),
            retail_dir: String::new(),
            addons_dir: String::new(),
            addon_installed: false,
            addon_version: None,
            addon_interface: None,
            accounts: Vec::new(),
        };
        assert!(sync_from_addon(&mut conn, &detection, 1).is_err());
    }
}

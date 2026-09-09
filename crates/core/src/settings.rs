//! Typed application settings over the `app_settings` table.
//!
//! Values are stored as JSON; reads apply defaults for missing keys, and writes
//! validate before persisting. Unknown keys are preserved untouched.

use crate::error::{AppError, Result};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Well-known setting keys stored in `app_settings`.
pub mod keys {
    pub const WOW_DIR: &str = "wow_dir";
    pub const REGION: &str = "region";
    pub const MARKET_PROVIDER: &str = "market_provider";
    pub const MARKET_REFRESH_HOURS: &str = "market_refresh_hours";
    pub const CROWD_SHARE: &str = "crowd.share";
    pub const CROWD_USE: &str = "crowd.use";
    pub const CROWD_ENDPOINT: &str = "crowd.endpoint";
    pub const ROUTE_CONFIG: &str = "route.config";
    pub const MOVEMENT_PROFILE: &str = "route.movement_profile";
    pub const UI_LAST_ROUTE: &str = "ui.last_route";
    pub const SYNC_LAST_AT: &str = "sync.last_at";
    pub const SYNC_LAST_RESULT: &str = "sync.last_result";
    pub const SYNC_LAST_EXPORT_AT: &str = "sync.last_export_at";
}

/// Supported market regions (EU first; others reserved for later).
pub const KNOWN_REGIONS: &[&str] = &["eu", "us", "kr", "tw", "cn"];

/// Full typed settings snapshot with defaults applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub wow_dir: Option<String>,
    pub region: String,
    pub market_provider: String,
    pub market_refresh_hours: i64,
    pub crowd_share: bool,
    pub crowd_use: bool,
    pub crowd_endpoint: String,
    #[serde(default)]
    pub route_config: serde_json::Value,
    #[serde(default)]
    pub movement_profile: serde_json::Value,
    pub ui_last_route: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            wow_dir: None,
            region: "eu".to_string(),
            market_provider: "tsm".to_string(),
            market_refresh_hours: 12,
            crowd_share: false,
            crowd_use: false,
            crowd_endpoint: String::new(),
            route_config: serde_json::Value::Object(Default::default()),
            movement_profile: serde_json::Value::Object(Default::default()),
            ui_last_route: None,
        }
    }
}

/// Partial update: every field optional; `None` leaves the stored value alone.
/// For nullable fields (`wow_dir`, `ui_last_route`), pass `Some(None)` to clear.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub wow_dir: Option<Option<String>>,
    pub region: Option<String>,
    pub market_provider: Option<String>,
    pub market_refresh_hours: Option<i64>,
    pub crowd_share: Option<bool>,
    pub crowd_use: Option<bool>,
    pub crowd_endpoint: Option<String>,
    pub route_config: Option<serde_json::Value>,
    pub movement_profile: Option<serde_json::Value>,
    pub ui_last_route: Option<Option<String>>,
}

/// Read a raw JSON setting. Returns `None` when the key was never stored.
pub fn get_raw(conn: &Connection, key: &str) -> Result<Option<serde_json::Value>> {
    let text: Option<String> = conn
        .query_row("SELECT value_json FROM app_settings WHERE key = ?1", rusqlite::params![key], |r| {
            r.get(0)
        })
        .optional()?;
    match text {
        None => Ok(None),
        Some(t) => Ok(Some(serde_json::from_str(&t)?)),
    }
}

/// Write a raw JSON setting (validated by callers for known keys).
pub fn set_raw(conn: &Connection, key: &str, value: &serde_json::Value, now: i64) -> Result<()> {
    let text = serde_json::to_string(value)?;
    conn.execute(
        "INSERT INTO app_settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json,
                                        updated_at = excluded.updated_at",
        rusqlite::params![key, text, now],
    )?;
    Ok(())
}

fn get_string(conn: &Connection, key: &str, fallback: &str) -> Result<String> {
    match get_raw(conn, key)? {
        Some(serde_json::Value::String(s)) => Ok(s),
        Some(_) => Err(AppError::config(format!("setting `{key}` has wrong type"))),
        None => Ok(fallback.to_string()),
    }
}

fn get_opt_string(conn: &Connection, key: &str) -> Result<Option<String>> {
    match get_raw(conn, key)? {
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(_) => Err(AppError::config(format!("setting `{key}` has wrong type"))),
    }
}

fn get_bool(conn: &Connection, key: &str, fallback: bool) -> Result<bool> {
    match get_raw(conn, key)? {
        Some(serde_json::Value::Bool(b)) => Ok(b),
        Some(_) => Err(AppError::config(format!("setting `{key}` has wrong type"))),
        None => Ok(fallback),
    }
}

fn get_int(conn: &Connection, key: &str, fallback: i64) -> Result<i64> {
    match get_raw(conn, key)? {
        Some(v) => v.as_i64().ok_or_else(|| AppError::config(format!("setting `{key}` has wrong type"))),
        None => Ok(fallback),
    }
}

fn get_json(conn: &Connection, key: &str) -> Result<serde_json::Value> {
    Ok(get_raw(conn, key)?.unwrap_or(serde_json::Value::Object(Default::default())))
}

/// Load the full settings snapshot with defaults for missing keys.
pub fn get_settings(conn: &Connection) -> Result<Settings> {
    let defaults = Settings::default();
    Ok(Settings {
        wow_dir: get_opt_string(conn, keys::WOW_DIR)?,
        region: get_string(conn, keys::REGION, &defaults.region)?,
        market_provider: get_string(conn, keys::MARKET_PROVIDER, &defaults.market_provider)?,
        market_refresh_hours: get_int(
            conn,
            keys::MARKET_REFRESH_HOURS,
            defaults.market_refresh_hours,
        )?,
        crowd_share: get_bool(conn, keys::CROWD_SHARE, defaults.crowd_share)?,
        crowd_use: get_bool(conn, keys::CROWD_USE, defaults.crowd_use)?,
        crowd_endpoint: get_string(conn, keys::CROWD_ENDPOINT, &defaults.crowd_endpoint)?,
        route_config: get_json(conn, keys::ROUTE_CONFIG)?,
        movement_profile: get_json(conn, keys::MOVEMENT_PROFILE)?,
        ui_last_route: get_opt_string(conn, keys::UI_LAST_ROUTE)?,
    })
}

fn validate_region(region: &str) -> Result<String> {
    let normalized = region.trim().to_lowercase();
    if !KNOWN_REGIONS.contains(&normalized.as_str()) {
        return Err(AppError::config(format!(
            "unknown region `{region}` (known: {})",
            KNOWN_REGIONS.join(", ")
        )));
    }
    Ok(normalized)
}

fn validate_refresh_hours(hours: i64) -> Result<i64> {
    if !(1..=168).contains(&hours) {
        return Err(AppError::config("market refresh interval must be 1–168 hours"));
    }
    Ok(hours)
}

fn validate_endpoint(endpoint: &str) -> Result<String> {
    let trimmed = endpoint.trim().to_string();
    if !trimmed.is_empty()
        && !trimmed.starts_with("https://")
        && !trimmed.starts_with("http://localhost")
        && !trimmed.starts_with("http://127.0.0.1")
    {
        return Err(AppError::config("crowd endpoint must be https (or localhost for tests)"));
    }
    Ok(trimmed)
}

/// Apply a validated patch and return the new snapshot.
pub fn update_settings(conn: &Connection, patch: &SettingsPatch, now: i64) -> Result<Settings> {
    if let Some(wow_dir) = &patch.wow_dir {
        let v = wow_dir.clone().map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
        set_raw(conn, keys::WOW_DIR, &v, now)?;
    }
    if let Some(region) = &patch.region {
        let v = validate_region(region)?;
        set_raw(conn, keys::REGION, &serde_json::Value::String(v), now)?;
    }
    if let Some(provider) = &patch.market_provider {
        let p = provider.trim().to_lowercase();
        if p != "tsm" {
            return Err(AppError::config(format!("unknown market provider `{provider}`")));
        }
        set_raw(conn, keys::MARKET_PROVIDER, &serde_json::Value::String(p), now)?;
    }
    if let Some(hours) = patch.market_refresh_hours {
        let h = validate_refresh_hours(hours)?;
        set_raw(conn, keys::MARKET_REFRESH_HOURS, &serde_json::Value::from(h), now)?;
    }
    if let Some(share) = patch.crowd_share {
        set_raw(conn, keys::CROWD_SHARE, &serde_json::Value::Bool(share), now)?;
    }
    if let Some(use_crowd) = patch.crowd_use {
        set_raw(conn, keys::CROWD_USE, &serde_json::Value::Bool(use_crowd), now)?;
    }
    if let Some(endpoint) = &patch.crowd_endpoint {
        let e = validate_endpoint(endpoint)?;
        set_raw(conn, keys::CROWD_ENDPOINT, &serde_json::Value::String(e), now)?;
    }
    if let Some(cfg) = &patch.route_config {
        set_raw(conn, keys::ROUTE_CONFIG, cfg, now)?;
    }
    if let Some(profile) = &patch.movement_profile {
        set_raw(conn, keys::MOVEMENT_PROFILE, profile, now)?;
    }
    if let Some(last) = &patch.ui_last_route {
        let v = last.clone().map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
        set_raw(conn, keys::UI_LAST_ROUTE, &v, now)?;
    }
    get_settings(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{migrations, storage};

    fn db() -> Connection {
        let mut conn = storage::open_memory().unwrap();
        migrations::migrate(&mut conn).unwrap();
        conn
    }

    #[test]
    fn defaults_apply_on_empty_db() {
        let conn = db();
        let s = get_settings(&conn).unwrap();
        assert_eq!(s.region, "eu");
        assert_eq!(s.market_refresh_hours, 12);
        assert!(!s.crowd_share && !s.crowd_use);
        assert!(s.wow_dir.is_none());
    }

    #[test]
    fn patch_roundtrip_and_validation() {
        let conn = db();
        let s = update_settings(
            &conn,
            &SettingsPatch {
                region: Some("EU".to_string()),
                market_refresh_hours: Some(24),
                crowd_share: Some(true),
                wow_dir: Some(Some("C:\\Games\\WoW".to_string())),
                ..Default::default()
            },
            1000,
        )
        .unwrap();
        assert_eq!(s.region, "eu");
        assert_eq!(s.market_refresh_hours, 24);
        assert!(s.crowd_share);
        assert_eq!(s.wow_dir.as_deref(), Some("C:\\Games\\WoW"));

        assert!(update_settings(&conn, &SettingsPatch { region: Some("xx".into()), ..Default::default() }, 1001).is_err());
        assert!(update_settings(&conn, &SettingsPatch { market_refresh_hours: Some(0), ..Default::default() }, 1001).is_err());
        assert!(update_settings(&conn, &SettingsPatch { crowd_endpoint: Some("ftp://x".into()), ..Default::default() }, 1001).is_err());

        // Clearing a nullable field works.
        let s = update_settings(&conn, &SettingsPatch { wow_dir: Some(None), ..Default::default() }, 1002).unwrap();
        assert!(s.wow_dir.is_none());
    }

    #[test]
    fn wrong_stored_type_is_an_error() {
        let conn = db();
        set_raw(&conn, keys::MARKET_REFRESH_HOURS, &serde_json::Value::String("x".into()), 1).unwrap();
        assert!(get_settings(&conn).is_err());
    }
}

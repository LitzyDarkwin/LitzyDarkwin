//! wowgoldfarm desktop shell — thin Tauri commands over `wowgoldfarm-core`.
//!
//! Rules: no business logic here beyond argument validation, path resolution,
//! HTTP fetching, and error translation. Domain commands are added per phase.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use tauri::{Manager, State};
use wowgoldfarm_core::{atlas, bridge, diagnostics, migrations, models, seed, settings, storage};

/// Shared application state: one SQLite connection behind a mutex (rusqlite
/// connections are `Send`, and all DB access is short + synchronous).
pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub app_dir: PathBuf,
    pub db_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub version: String,
    pub schema_version: u32,
    pub db_path: String,
    pub sync_state: String,
}

fn lock_db<'a>(
    state: &'a State<'_, AppState>,
) -> Result<MutexGuard<'a, rusqlite::Connection>, String> {
    state.db.lock().map_err(|e| format!("Database is busy: {e}"))
}

fn user_err(e: wowgoldfarm_core::error::AppError) -> String {
    e.user_message()
}

/// Minimal status probe: version, schema, sync state.
#[tauri::command]
fn get_status(state: State<'_, AppState>) -> Result<StatusPayload, String> {
    let db = lock_db(&state)?;
    let schema_version = storage::user_version(&db).map_err(user_err)?;
    let sync_state = bridge::get_sync_state(&db, models::now_unix())
        .map(|s| s.last_result.unwrap_or_else(|| "not_synced".to_string()))
        .unwrap_or_else(|_| "unknown".to_string());
    Ok(StatusPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version,
        db_path: state.db_path.to_string_lossy().to_string(),
        sync_state,
    })
}

/// Probe default install locations for valid WoW installs.
#[tauri::command]
fn detect_wow() -> Result<Vec<bridge::WowDetection>, String> {
    let mut found = Vec::new();
    for candidate in bridge::candidate_wow_dirs() {
        if let Ok(detection) = bridge::validate_wow_dir(&candidate) {
            found.push(detection);
        }
    }
    Ok(found)
}

/// Validate a user-supplied WoW directory and persist it to settings.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn set_wow_dir(state: State<'_, AppState>, path: String) -> Result<bridge::WowDetection, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("WoW directory path is empty.".to_string());
    }
    if trimmed.len() > 512 {
        return Err("WoW directory path is too long.".to_string());
    }
    let detection = bridge::validate_wow_dir(Path::new(trimmed)).map_err(user_err)?;
    let db = lock_db(&state)?;
    let patch = settings::SettingsPatch {
        wow_dir: Some(Some(detection.wow_dir.clone())),
        ..Default::default()
    };
    settings::update_settings(&db, &patch, models::now_unix()).map_err(user_err)?;
    Ok(detection)
}

/// Read the configured WoW dir, validate it, and sync all account SV files.
#[tauri::command]
fn sync_addon(state: State<'_, AppState>) -> Result<bridge::SyncReport, String> {
    let (wow_dir, now) = {
        let db = lock_db(&state)?;
        let cfg = settings::get_settings(&db).map_err(user_err)?;
        (cfg.wow_dir, models::now_unix())
    };
    let wow_dir = wow_dir.ok_or_else(|| {
        "WoW directory is not configured. Detect or set it in Settings first.".to_string()
    })?;
    let detection = bridge::validate_wow_dir(Path::new(&wow_dir)).map_err(user_err)?;
    let mut db = lock_db(&state)?;
    bridge::sync_from_addon(&mut db, &detection, now).map_err(user_err)
}

#[tauri::command]
fn get_sync_state(state: State<'_, AppState>) -> Result<bridge::SyncStateInfo, String> {
    let db = lock_db(&state)?;
    bridge::get_sync_state(&db, models::now_unix()).map_err(user_err)
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<settings::Settings, String> {
    let db = lock_db(&state)?;
    settings::get_settings(&db).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn update_settings(
    state: State<'_, AppState>,
    patch: settings::SettingsPatch,
) -> Result<settings::Settings, String> {
    // Re-validate an incoming WoW dir before anything is persisted.
    if let Some(Some(dir)) = patch.wow_dir.as_ref() {
        bridge::validate_wow_dir(Path::new(dir)).map_err(user_err)?;
    }
    let db = lock_db(&state)?;
    settings::update_settings(&db, &patch, models::now_unix()).map_err(user_err)
}

#[tauri::command]
fn get_diagnostics(state: State<'_, AppState>) -> Result<diagnostics::DiagnosticReport, String> {
    let db = lock_db(&state)?;
    diagnostics::collect(&db, env!("CARGO_PKG_VERSION"), &state.db_path, models::now_unix())
        .map_err(user_err)
}

#[tauri::command]
fn get_atlas_tree(state: State<'_, AppState>) -> Result<atlas::AtlasTree, String> {
    let db = lock_db(&state)?;
    atlas::tree(&db).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn search_resources(
    state: State<'_, AppState>,
    query: String,
    category: Option<String>,
    limit: usize,
) -> Result<Vec<atlas::ResourceHit>, String> {
    let db = lock_db(&state)?;
    atlas::search_resources(&db, &query, category.as_deref(), limit).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn get_zone_detail(state: State<'_, AppState>, zone_id: String) -> Result<atlas::ZoneDetail, String> {
    let db = lock_db(&state)?;
    atlas::zone_detail(&db, &zone_id).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn get_zone_observations(
    state: State<'_, AppState>,
    zone_id: String,
    resource_ids: Vec<String>,
    grid: usize,
) -> Result<atlas::ClusteredObservations, String> {
    let db = lock_db(&state)?;
    atlas::zone_observations(&db, &zone_id, &resource_ids, grid).map_err(user_err)
}

fn init_logging(app_dir: &std::path::Path) {
    // Dev: stderr. Prod file log is added in Phase 9 diagnostics work.
    let _ = app_dir;
    let _ = env_logger::try_init();
}

/// Resolve the app-data directory, open + migrate the database.
fn init_state(app: &tauri::App) -> Result<AppState, wowgoldfarm_core::error::AppError> {
    let app_dir: PathBuf = app
        .path()
        .app_data_dir()
        .map_err(|e| wowgoldfarm_core::error::AppError::config(e.to_string()))?;
    std::fs::create_dir_all(&app_dir)?;
    let db_path = app_dir.join("wowgoldfarm.db");
    let mut db = storage::open_db(&db_path)?;
    let applied = migrations::migrate(&mut db)?;
    if !applied.is_empty() {
        log::info!("applied migrations: {applied:?}");
    }
    // Bundled atlas seed (idempotent, upgrade-safe). A corrupt seed must not
    // brick startup, but it is always logged loudly.
    match seed::load(&mut db) {
        Ok(stats) => log::info!(
            "atlas seed: {} zones, {} resources, {} observations ({} merges)",
            stats.zones,
            stats.resources,
            stats.observations,
            stats.merged_zones + stats.merged_resources
        ),
        Err(e) => log::error!("atlas seed failed: {}", e.user_message()),
    }
    Ok(AppState { db: Mutex::new(db), app_dir, db_path })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir =
                app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir());
            init_logging(&data_dir);
            let state = init_state(app)
                .map_err(|e| -> Box<dyn std::error::Error> { e.user_message().into() })?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            detect_wow,
            set_wow_dir,
            sync_addon,
            get_sync_state,
            get_settings,
            update_settings,
            get_diagnostics,
            get_atlas_tree,
            search_resources,
            get_zone_detail,
            get_zone_observations
        ])
        .run(tauri::generate_context!())
        // Process entry point: nothing recoverable to do if the runtime fails.
        .expect("failed to run wowgoldfarm");
}

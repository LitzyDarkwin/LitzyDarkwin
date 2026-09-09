//! wowgoldfarm desktop shell — thin Tauri commands over `wowgoldfarm-core`.
//!
//! Rules: no business logic here beyond argument validation, path resolution,
//! HTTP fetching, and error translation. Domain commands are added per phase.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use wowgoldfarm_core::{migrations, storage};

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
    pub sync_state: &'static str,
}

/// Minimal Phase-0 command proving shell↔core wiring. Full command surface
/// arrives per phase (bridge in P2, atlas P3, routes P4, market P5, …).
#[tauri::command]
fn get_status(state: State<'_, AppState>) -> Result<StatusPayload, String> {
    let db = state.db.lock().map_err(|e| format!("Database busy: {e}"))?;
    let schema_version = storage::user_version(&db).map_err(|e| e.user_message())?;
    Ok(StatusPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version,
        db_path: state.db_path.to_string_lossy().to_string(),
        sync_state: "not_synced",
    })
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
        .invoke_handler(tauri::generate_handler![get_status])
        .run(tauri::generate_context!())
        // Process entry point: nothing recoverable to do if the runtime fails.
        .expect("failed to run wowgoldfarm");
}

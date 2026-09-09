//! wowgoldfarm desktop shell — thin Tauri commands over `wowgoldfarm-core`.
//!
//! Rules: no business logic here beyond argument validation, path resolution,
//! HTTP fetching, and error translation. Domain commands are added per phase.

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use tauri::{Manager, State};
use wowgoldfarm_core::{
    atlas, bridge, diagnostics, market, migrations, models, routes, seed, settings, storage,
};

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

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn generate_routes(
    state: State<'_, AppState>,
    zone_id: String,
    resource_ids: Vec<String>,
    config: routes::RouteConfigDto,
    prices: HashMap<String, i64>,
    excluded: Vec<routes::ExcludedArea>,
) -> Result<Vec<routes::CandidateDto>, String> {
    let db = lock_db(&state)?;
    let cfg = config.into_config();
    routes::generate(&db, &zone_id, &resource_ids, &cfg, &prices, &excluded).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn save_route(
    state: State<'_, AppState>,
    input: routes::SaveRouteInput,
) -> Result<routes::SavedRoute, String> {
    let db = lock_db(&state)?;
    routes::save_route(&db, &input, models::now_unix()).map_err(user_err)
}

#[tauri::command]
fn list_routes(
    state: State<'_, AppState>,
    zone_id: Option<String>,
) -> Result<Vec<routes::RouteSummary>, String> {
    let db = lock_db(&state)?;
    routes::list_routes(&db, zone_id.as_deref()).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn get_route(state: State<'_, AppState>, id: String) -> Result<routes::SavedRoute, String> {
    let db = lock_db(&state)?;
    routes::get_route(&db, &id).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn delete_route(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let db = lock_db(&state)?;
    routes::delete_route(&db, &id).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn compare_routes(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<Vec<routes::SavedRoute>, String> {
    let db = lock_db(&state)?;
    routes::compare_routes(&db, &ids).map_err(user_err)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouteExport {
    pub code: String,
    pub map_id: i64,
    pub waypoint_count: usize,
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn export_route_string(
    state: State<'_, AppState>,
    id: String,
) -> Result<RouteExport, String> {
    let db = lock_db(&state)?;
    let route = routes::get_route(&db, &id).map_err(user_err)?;
    let zone = route.zone_id.clone().ok_or_else(|| {
        "Route has no zone — cannot build an in-game import code.".to_string()
    })?;
    let map_id = routes::zone_map_id(&db, &zone)
        .map_err(user_err)?
        .ok_or_else(|| format!("Zone `{zone}` has no map id — cannot build an import code."))?;
    let pts: Vec<(f64, f64)> = route.waypoints.iter().map(|w| (w.x, w.y)).collect();
    let code = routes::go1_encode(map_id, &pts).map_err(user_err)?;
    Ok(RouteExport { code, map_id, waypoint_count: pts.len() })
}

/// Effective region slug: explicit arg wins, else settings. Strict shape —
/// the slug is interpolated into a URL.
fn resolve_region(db: &rusqlite::Connection, region: Option<String>) -> Result<String, String> {
    let slug = match region {
        Some(r) if !r.trim().is_empty() => r,
        _ => settings::get_settings(db).map_err(user_err)?.region,
    };
    let slug = slug.trim().to_lowercase();
    if slug.len() < 2
        || slug.len() > 16
        || !slug.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
    {
        return Err(format!("Invalid region slug `{slug}`."));
    }
    Ok(slug)
}

fn valuation_cfg(db: &rusqlite::Connection) -> Result<market::ValuationConfig, String> {
    let hours = settings::get_settings(db).map_err(user_err)?.market_refresh_hours;
    Ok(market::ValuationConfig {
        refresh_hours: (hours as f64).clamp(1.0, 168.0),
        ..Default::default()
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshSummary {
    pub region: String,
    pub commodities: Option<market::IngestReport>,
    pub region_items: Option<market::IngestReport>,
    pub errors: Vec<String>,
}

/// Fetch both TSM datasets over HTTPS and ingest them. Partial success is
/// reported honestly; only a total failure becomes an Err.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn refresh_market(
    state: State<'_, AppState>,
    region: Option<String>,
) -> Result<RefreshSummary, String> {
    let slug = {
        let db = lock_db(&state)?;
        resolve_region(&db, region)?
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent(format!("wowgoldfarm/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("HTTP client failed: {e}"))?;
    let fetch = |url: &str| -> Result<String, String> {
        client
            .get(url)
            .send()
            .map_err(|e| format!("request failed: {e}"))?
            .error_for_status()
            .map_err(|e| format!("HTTP error: {e}"))?
            .text()
            .map_err(|e| format!("read failed: {e}"))
    };

    let mut summary = RefreshSummary {
        region: slug.clone(),
        commodities: None,
        region_items: None,
        errors: Vec::new(),
    };
    // Fetch both files before touching the DB (fail fast on network errors).
    let comm_text = fetch(&market::commodities_url(&slug));
    let region_text = fetch(&market::region_items_url(&slug));
    let mut db = lock_db(&state)?;
    let now = models::now_unix();
    match comm_text {
        Ok(text) => match market::ingest_commodities(&mut db, &slug, &text, now) {
            Ok(report) => summary.commodities = Some(report),
            Err(e) => summary.errors.push(format!("commodities: {}", e.user_message())),
        },
        Err(e) => summary.errors.push(format!("commodities: {e}")),
    }
    match region_text {
        Ok(text) => match market::ingest_region_items(&mut db, &slug, &text, now) {
            Ok(report) => summary.region_items = Some(report),
            Err(e) => summary.errors.push(format!("region items: {}", e.user_message())),
        },
        Err(e) => summary.errors.push(format!("region items: {e}")),
    }
    if summary.commodities.is_none() && summary.region_items.is_none() {
        return Err(summary.errors.join(" / "));
    }
    Ok(summary)
}

#[tauri::command]
fn get_market_status(
    state: State<'_, AppState>,
    region: Option<String>,
) -> Result<market::MarketStatus, String> {
    let db = lock_db(&state)?;
    let slug = resolve_region(&db, region)?;
    market::market_status(&db, &slug, models::now_unix()).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn get_item_valuation(
    state: State<'_, AppState>,
    item_id: i64,
) -> Result<market::ItemValuation, String> {
    let db = lock_db(&state)?;
    let cfg = valuation_cfg(&db)?;
    let signals = market::get_signals(&db, item_id).map_err(user_err)?;
    Ok(market::valuate(item_id, signals.as_ref(), &cfg, models::now_unix()))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn search_market_items(
    state: State<'_, AppState>,
    query: String,
    limit: usize,
) -> Result<Vec<market::MarketItemRow>, String> {
    let db = lock_db(&state)?;
    let cfg = valuation_cfg(&db)?;
    market::search_items(&db, &query, limit, &cfg, models::now_unix()).map_err(user_err)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands take owned args.
fn get_market_prices(
    state: State<'_, AppState>,
    resource_ids: Vec<String>,
) -> Result<market::MarketPrices, String> {
    let db = lock_db(&state)?;
    let cfg = valuation_cfg(&db)?;
    market::resolve_prices(&db, &resource_ids, &cfg, models::now_unix()).map_err(user_err)
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
            get_zone_observations,
            generate_routes,
            save_route,
            list_routes,
            get_route,
            delete_route,
            compare_routes,
            export_route_string,
            refresh_market,
            get_market_status,
            get_item_valuation,
            search_market_items,
            get_market_prices
        ])
        .run(tauri::generate_context!())
        // Process entry point: nothing recoverable to do if the runtime fails.
        .expect("failed to run wowgoldfarm");
}

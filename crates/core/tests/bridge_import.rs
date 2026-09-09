//! End-to-end: fake WoW install → validate → sync → SQLite → re-sync idempotent.
use wowgoldfarm_core::{bridge, migrations, storage};

const SV_SAMPLE: &str = include_str!("../../../fixtures/savedvariables_sample.lua");

/// Build a minimal fake WoW install in a temp dir and return (dir, db).
fn fake_wow(sv_text: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let retail = root.join("_retail_");
    let addon = retail.join("Interface").join("AddOns").join("GatherOptimizer");
    std::fs::create_dir_all(&addon).unwrap();
    std::fs::write(
        addon.join("GatherOptimizer.toc"),
        "## Interface: 120100\n## Title: GatherOptimizer\n## Version: 1.2.3\n",
    )
    .unwrap();
    let sv_dir = retail.join("WTF").join("Account").join("ACCT1").join("SavedVariables");
    std::fs::create_dir_all(&sv_dir).unwrap();
    std::fs::write(sv_dir.join("GatherOptimizer.lua"), sv_text).unwrap();
    (dir, root)
}

fn open_db() -> rusqlite::Connection {
    let mut conn = storage::open_memory().unwrap();
    migrations::migrate(&mut conn).unwrap();
    conn
}

#[test]
fn full_sync_imports_sessions_items_and_observations() {
    let (_dir, root) = fake_wow(SV_SAMPLE);
    let detection = bridge::validate_wow_dir(&root).unwrap();
    assert!(detection.addon_installed);
    assert_eq!(detection.addon_version.as_deref(), Some("1.2.3"));
    assert_eq!(detection.addon_interface.as_deref(), Some("120100"));
    assert_eq!(detection.accounts.len(), 1);

    let mut conn = open_db();
    let report = bridge::sync_from_addon(&mut conn, &detection, 1_700_100_000).unwrap();
    assert_eq!(report.sessions_imported, 2);
    assert_eq!(report.sessions_skipped, 0);
    assert_eq!(report.observations_imported, 2);
    assert!(report.warnings.is_empty());
    assert_eq!(report.files[0].export_timestamp, Some(1700080700));

    // Session row + aggregated items (ore 2+1=3 across two gathers).
    let active: i64 = conn
        .query_row(
            "SELECT active_seconds FROM farm_sessions WHERE id = 'Testtoon-Testrealm-1700000100-1'",
            rusqlite::params![],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(active, 1740);
    let ore_qty: f64 = conn
        .query_row(
            "SELECT qty FROM farm_session_items WHERE session_id LIKE '%-1' AND item_id = 12345",
            rusqlite::params![],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ore_qty, 3.0);
    let herb_qty: f64 = conn
        .query_row(
            "SELECT qty FROM farm_session_items WHERE item_id = 9",
            rusqlite::params![],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(herb_qty, 3.0);

    // Zone + resources resolved canonically.
    let zone: String = conn
        .query_row(
            "SELECT zone_id FROM farm_sessions WHERE external_id LIKE '%-1'",
            rusqlite::params![],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(zone, "map-2346");
    let obs_sources: Vec<String> = conn
        .prepare("SELECT DISTINCT source FROM resource_observations")
        .unwrap()
        .query_map(rusqlite::params![], |r| r.get(0))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();
    assert_eq!(obs_sources, vec!["addon".to_string()]);

    // Sync state persisted + fresh.
    let state = bridge::get_sync_state(&conn, 1_700_100_100).unwrap();
    assert_eq!(state.last_sync_at, Some(1_700_100_000));
    assert_eq!(state.last_export_at, Some(1700080700));
    assert_eq!(state.freshness, wowgoldfarm_core::models::Freshness::Fresh);
}

#[test]
fn resync_is_idempotent_for_sessions_and_snapshot_for_observations() {
    let (_dir, root) = fake_wow(SV_SAMPLE);
    let detection = bridge::validate_wow_dir(&root).unwrap();
    let mut conn = open_db();
    bridge::sync_from_addon(&mut conn, &detection, 100).unwrap();
    let second = bridge::sync_from_addon(&mut conn, &detection, 200).unwrap();
    assert_eq!(second.sessions_imported, 0);
    assert_eq!(second.sessions_skipped, 2);
    // Snapshot replace: still exactly 2 addon observations, never 4.
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM resource_observations", rusqlite::params![], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 2);
}

#[test]
fn schema_mismatch_skips_file_with_warning() {
    let bad = SV_SAMPLE.replace(
        "[\"schemaVersion\"] = 1,",
        "[\"schemaVersion\"] = 99,",
    );
    assert!(bad.contains("99"));
    let (_dir, root) = fake_wow(&bad);
    let detection = bridge::validate_wow_dir(&root).unwrap();
    let mut conn = open_db();
    let report = bridge::sync_from_addon(&mut conn, &detection, 100).unwrap();
    assert_eq!(report.sessions_imported, 0);
    assert_eq!(report.observations_imported, 0);
    assert!(!report.warnings.is_empty());
    assert!(report.warnings[0].contains("unsupported addon schema"));
}

#[test]
fn sync_state_unknown_before_first_sync() {
    let conn = open_db();
    let state = bridge::get_sync_state(&conn, 1_700_000_000).unwrap();
    assert_eq!(state.freshness, wowgoldfarm_core::models::Freshness::Unknown);
    assert!(state.last_sync_at.is_none());
}

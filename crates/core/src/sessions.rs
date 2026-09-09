//! Farm session analytics (Phase 6).
//!
//! Sessions arrive via addon sync as `imported`. Analysis snapshots per-item
//! unit values from the market and flips status to `analyzed`. Values come in
//! two flavors: THEN (snapshot at analyze time) and NOW (live market). Route
//! links attach expectations for expected-vs-actual comparison.

use crate::error::{AppError, Result};
use crate::market::{self, ValuationConfig};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const STATUS_IMPORTED: &str = "imported";
pub const STATUS_ANALYZED: &str = "analyzed";

// Math (cross-checked by fixtures/expected/session.json) ------------------------

/// One loot line with optional then/now unit prices (copper).
#[derive(Debug, Clone)]
pub struct ItemValue {
    pub qty: f64,
    pub unit_then: Option<i64>,
    pub unit_now: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct Totals {
    pub qty: f64,
    pub value_then: f64,
    pub value_now: f64,
    pub then_partial: bool,
    pub now_partial: bool,
}

/// Aggregate loot lines. Lines without a unit price are skipped and raise the
/// corresponding partial flag — totals never silently pretend complete.
pub fn totals_for(items: &[ItemValue]) -> Totals {
    let mut t = Totals::default();
    for item in items {
        let qty = item.qty.max(0.0);
        t.qty += qty;
        match item.unit_then {
            Some(u) => t.value_then += qty * u.max(0) as f64,
            None => t.then_partial = true,
        }
        match item.unit_now {
            Some(u) => t.value_now += qty * u.max(0) as f64,
            None => t.now_partial = true,
        }
    }
    t
}

/// Gold per hour, or None when no active time was recorded.
pub fn gph_for(value_copper: f64, active_seconds: i64) -> Option<f64> {
    if active_seconds <= 0 {
        return None;
    }
    Some(value_copper / (active_seconds as f64 / 3600.0))
}

/// Expected session total at a given GPH rate.
pub fn expected_total(expected_gph: f64, active_seconds: i64) -> f64 {
    expected_gph * active_seconds.max(0) as f64 / 3600.0
}

/// Actual-vs-expected delta in percent, or None without a baseline.
pub fn delta_pct(actual: f64, expected: f64) -> Option<f64> {
    if expected <= 0.0 {
        return None;
    }
    Some((actual - expected) / expected * 100.0)
}

// DTOs -------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionItemVal {
    pub item_id: i64,
    pub resource_id: Option<String>,
    pub display_name: String,
    pub qty: f64,
    pub unit_then: Option<i64>,
    pub unit_now: Option<i64>,
    pub value_then: Option<i64>,
    pub value_now: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub character: String,
    pub realm: String,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub route_id: Option<String>,
    pub status: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub active_seconds: i64,
    pub item_count: usize,
    pub total_qty: f64,
    pub value_then: i64,
    pub value_now: i64,
    pub then_partial: bool,
    pub now_partial: bool,
    pub gph_then: Option<f64>,
    pub gph_now: Option<f64>,
    pub expected_gph: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub summary: SessionSummary,
    pub notes: String,
    pub route_name: Option<String>,
    pub expected_value_per_loop: Option<i64>,
    pub items: Vec<SessionItemVal>,
    pub expected_total: Option<i64>,
    pub delta_pct_then: Option<f64>,
    pub delta_pct_now: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayBucket {
    pub day: String,
    pub sessions: i64,
    pub active_seconds: i64,
    pub value_then: i64,
    pub value_now: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBucket {
    pub key: String,
    pub display_name: String,
    pub qty: f64,
    pub value_then: i64,
    pub value_now: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    pub session_count: i64,
    pub active_seconds: i64,
    pub value_then: i64,
    pub value_now: i64,
    pub then_partial: bool,
    pub now_partial: bool,
    pub by_day: Vec<DayBucket>,
    pub by_resource: Vec<ResourceBucket>,
}

// Internals -----------------------------------------------------------------------------

struct SessionRow {
    id: String,
    character: String,
    realm: String,
    zone_id: Option<String>,
    zone_name: Option<String>,
    route_id: Option<String>,
    route_name: Option<String>,
    status: String,
    started_at: i64,
    ended_at: Option<i64>,
    active_seconds: i64,
    expected_gph: Option<i64>,
    expected_value: Option<i64>,
    notes: String,
}

fn get_row(conn: &Connection, id: &str) -> Result<SessionRow> {
    let row: Option<(
        String, String, Option<String>, Option<String>, Option<String>, Option<String>, String,
        i64, Option<i64>, i64, Option<i64>, Option<i64>, String,
    )> = conn
        .query_row(
            "SELECT s.id, s.character_name, s.realm, s.zone_id, z.name, s.route_id, r.name,
                    s.status, s.started_at, s.ended_at, s.active_seconds,
                    s.expected_gph, s.expected_value, s.notes
             FROM farm_sessions s
             LEFT JOIN zones z ON z.id = s.zone_id
             LEFT JOIN saved_routes r ON r.id = s.route_id
             WHERE s.id = ?1",
            rusqlite::params![id],
            |r| {
                Ok((
                    r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
                    r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?,
                    r.get(12)?,
                ))
            },
        )
        .optional()?;
    match row {
        None => Err(AppError::session(format!("unknown session `{id}`"))),
        Some(
            (id, character, realm, zone_id, zone_name, route_id, route_name, status,
             started_at, ended_at, active_seconds, expected_gph, expected_value, notes),
        ) => Ok(SessionRow {
            id, character, realm, zone_id, zone_name, route_id, route_name, status,
            started_at, ended_at, active_seconds, expected_gph, expected_value, notes,
        }),
    }
}

struct RawItem {
    item_id: i64,
    resource_id: Option<String>,
    qty: f64,
    unit_then: Option<i64>,
}

fn load_items(conn: &Connection, session_id: &str) -> Result<Vec<RawItem>> {
    let mut stmt = conn.prepare(
        "SELECT item_id, resource_id, qty, unit_value FROM farm_session_items
         WHERE session_id = ?1 ORDER BY item_id",
    )?;
    let rows = stmt.query_map(rusqlite::params![session_id], |r| {
        Ok(RawItem { item_id: r.get(0)?, resource_id: r.get(1)?, qty: r.get(2)?, unit_then: r.get(3)? })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>().map_err(AppError::from)
}

fn display_name(
    conn: &Connection,
    item_id: i64,
    resource_id: Option<&str>,
    cache: &mut HashMap<String, String>,
) -> String {
    if let Some(name) = cache.get(&format!("i{item_id}")) {
        return name.clone();
    }
    // Prefer the market item name, then the resource name, then a bare id.
    let market_name: Option<String> = conn
        .query_row(
            "SELECT item_name FROM market_items WHERE item_id = ?1",
            rusqlite::params![item_id],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None)
        .filter(|n: &String| !n.is_empty());
    if let Some(name) = market_name {
        cache.insert(format!("i{item_id}"), name.clone());
        return name;
    }
    if let Some(res) = resource_id {
        if let Some(name) = cache.get(res) {
            return name.clone();
        }
        let resource_name: Option<String> = conn
            .query_row("SELECT name FROM resources WHERE id = ?1", rusqlite::params![res], |r| {
                r.get(0)
            })
            .optional()
            .unwrap_or(None);
        if let Some(name) = resource_name {
            cache.insert(res.to_string(), name.clone());
            return name;
        }
    }
    format!("Item #{item_id}")
}

fn valuate_items(
    conn: &Connection,
    raw: &[RawItem],
    cfg: &ValuationConfig,
    now: i64,
    cache: &mut HashMap<String, String>,
) -> Result<Vec<SessionItemVal>> {
    let mut out = Vec::with_capacity(raw.len());
    for item in raw {
        let signals = market::get_signals(conn, item.item_id)?;
        let live = market::valuate(item.item_id, signals.as_ref(), cfg, now);
        let unit_now = if live.effective_value > 0 { Some(live.effective_value) } else { None };
        let qty = item.qty.max(0.0);
        out.push(SessionItemVal {
            item_id: item.item_id,
            resource_id: item.resource_id.clone(),
            display_name: display_name(conn, item.item_id, item.resource_id.as_deref(), cache),
            qty,
            unit_then: item.unit_then,
            unit_now,
            value_then: item.unit_then.map(|u| (qty * u.max(0) as f64).round() as i64),
            value_now: unit_now.map(|u| (qty * u.max(0) as f64).round() as i64),
        });
    }
    out.sort_by(|a, b| b.value_now.unwrap_or(0).cmp(&a.value_now.unwrap_or(0)));
    Ok(out)
}

fn summarize(
    conn: &Connection,
    row: &SessionRow,
    cfg: &ValuationConfig,
    now: i64,
    cache: &mut HashMap<String, String>,
) -> Result<(SessionSummary, Vec<SessionItemVal>)> {
    let raw = load_items(conn, &row.id)?;
    let items = valuate_items(conn, &raw, cfg, now, cache)?;
    let lines: Vec<ItemValue> = items
        .iter()
        .map(|i| ItemValue { qty: i.qty, unit_then: i.unit_then, unit_now: i.unit_now })
        .collect();
    let totals = totals_for(&lines);
    Ok((
        SessionSummary {
            id: row.id.clone(),
            character: row.character.clone(),
            realm: row.realm.clone(),
            zone_id: row.zone_id.clone(),
            zone_name: row.zone_name.clone(),
            route_id: row.route_id.clone(),
            status: row.status.clone(),
            started_at: row.started_at,
            ended_at: row.ended_at,
            active_seconds: row.active_seconds,
            item_count: items.len(),
            total_qty: totals.qty,
            value_then: totals.value_then.round() as i64,
            value_now: totals.value_now.round() as i64,
            then_partial: totals.then_partial,
            now_partial: totals.now_partial,
            gph_then: gph_for(totals.value_then, row.active_seconds),
            gph_now: gph_for(totals.value_now, row.active_seconds),
            expected_gph: row.expected_gph,
        },
        items,
    ))
}

// Commands -------------------------------------------------------------------------------

/// Analyze a session: snapshot missing unit prices from the market (first
/// snapshot wins — closest to "then"), flip `imported` → `analyzed`.
/// Idempotent; safe to run after every market refresh.
pub fn analyze_session(
    conn: &Connection,
    id: &str,
    cfg: &ValuationConfig,
    now: i64,
) -> Result<SessionDetail> {
    let row = get_row(conn, id)?;
    for item in load_items(conn, id)? {
        if item.unit_then.is_some() {
            continue;
        }
        let signals = market::get_signals(conn, item.item_id)?;
        let live = market::valuate(item.item_id, signals.as_ref(), cfg, now);
        if live.effective_value > 0 {
            conn.execute(
                "UPDATE farm_session_items SET unit_value = ?1
                 WHERE session_id = ?2 AND item_id = ?3",
                rusqlite::params![live.effective_value, id, item.item_id],
            )?;
        }
    }
    if row.status == STATUS_IMPORTED {
        conn.execute(
            "UPDATE farm_sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![STATUS_ANALYZED, now, id],
        )?;
    }
    get_session_detail(conn, id, cfg, now)
}

/// Attach (or detach, with None) a route's expectations to a session.
/// Snapshots the route's GPH + per-loop value so later route edits can't
/// rewrite history.
pub fn link_route(
    conn: &Connection,
    id: &str,
    route_id: Option<&str>,
    now: i64,
) -> Result<SessionDetail> {
    get_row(conn, id)?; // unknown session fails before anything is written
    match route_id {
        None => {
            conn.execute(
                "UPDATE farm_sessions SET route_id = NULL, expected_gph = NULL,
                 expected_value = NULL, updated_at = ?1 WHERE id = ?2",
                rusqlite::params![now, id],
            )?;
        }
        Some(route) => {
            let stats_json: Option<String> = conn
                .query_row(
                    "SELECT stats_json FROM saved_routes WHERE id = ?1",
                    rusqlite::params![route],
                    |r| r.get(0),
                )
                .optional()?;
            let stats_json =
                stats_json.ok_or_else(|| AppError::session(format!("unknown route `{route}`")))?;
            let stats: serde_json::Value = serde_json::from_str(&stats_json)
                .map_err(|_| AppError::provider(format!("route `{route}` has corrupt stats")))?;
            let gph = stats.get("gphExpected").and_then(serde_json::Value::as_f64);
            let per_loop = stats.get("effectiveValue").and_then(serde_json::Value::as_f64);
            conn.execute(
                "UPDATE farm_sessions SET route_id = ?1, expected_gph = ?2,
                 expected_value = ?3, updated_at = ?4 WHERE id = ?5",
                rusqlite::params![
                    route,
                    gph.map(|v| v.round() as i64),
                    per_loop.map(|v| v.round() as i64),
                    now,
                    id,
                ],
            )?;
        }
    }
    // cfg/now only affect live prices in the returned detail, not the link.
    get_session_detail(conn, id, &ValuationConfig::default(), now)
}

pub fn get_session_detail(
    conn: &Connection,
    id: &str,
    cfg: &ValuationConfig,
    now: i64,
) -> Result<SessionDetail> {
    let row = get_row(conn, id)?;
    let mut cache = HashMap::new();
    let (summary, items) = summarize(conn, &row, cfg, now, &mut cache)?;
    let expected_total = row
        .expected_gph
        .map(|g| expected_total(g as f64, row.active_seconds).round() as i64);
    let as_f64 = expected_total.map(|v| v as f64);
    Ok(SessionDetail {
        summary,
        notes: row.notes,
        route_name: row.route_name,
        expected_value_per_loop: row.expected_value,
        items,
        expected_total,
        delta_pct_then: as_f64.and_then(|e| delta_pct(totals_then(&items), e)),
        delta_pct_now: as_f64.and_then(|e| delta_pct(totals_now(&items), e)),
    })
}

fn totals_then(items: &[SessionItemVal]) -> f64 {
    items.iter().filter_map(|i| i.value_then.map(|v| v as f64)).sum()
}

fn totals_now(items: &[SessionItemVal]) -> f64 {
    items.iter().filter_map(|i| i.value_now.map(|v| v as f64)).sum()
}

pub fn list_sessions(
    conn: &Connection,
    zone_id: Option<&str>,
    limit: usize,
    cfg: &ValuationConfig,
    now: i64,
) -> Result<Vec<SessionSummary>> {
    let limit = limit.clamp(1, 200) as i64;
    let mut stmt = conn.prepare(
        "SELECT s.id FROM farm_sessions s
         WHERE s.zone_id = ?1 OR ?1 IS NULL
         ORDER BY s.started_at DESC, s.id DESC LIMIT ?2",
    )?;
    let ids: Vec<String> = stmt
        .query_map(rusqlite::params![zone_id, limit], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    let mut cache = HashMap::new();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let row = get_row(conn, &id)?;
        let (summary, _) = summarize(conn, &row, cfg, now, &mut cache)?;
        out.push(summary);
    }
    Ok(out)
}

pub fn session_stats(
    conn: &Connection,
    zone_id: Option<&str>,
    days: Option<i64>,
    cfg: &ValuationConfig,
    now: i64,
) -> Result<SessionStats> {
    let since = days.map(|d| now - d.max(1) * 86400);
    let mut stmt = conn.prepare(
        "SELECT s.id, s.started_at, s.active_seconds FROM farm_sessions s
         WHERE (s.zone_id = ?1 OR ?1 IS NULL) AND (?2 IS NULL OR s.started_at >= ?2)
         ORDER BY s.started_at DESC",
    )?;
    let rows: Vec<(String, i64, i64)> = stmt
        .query_map(rusqlite::params![zone_id, since], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<std::result::Result<_, _>>()?;

    struct DayAcc {
        sessions: i64,
        seconds: i64,
        then: f64,
        now: f64,
    }
    struct ResAcc {
        display_name: String,
        qty: f64,
        then: f64,
        now: f64,
    }
    let mut cache = HashMap::new();
    let mut stats = SessionStats {
        session_count: rows.len() as i64,
        active_seconds: 0,
        value_then: 0,
        value_now: 0,
        then_partial: false,
        now_partial: false,
        by_day: Vec::new(),
        by_resource: Vec::new(),
    };
    let mut days_map: HashMap<String, DayAcc> = HashMap::new();
    let mut res_map: HashMap<String, ResAcc> = HashMap::new();
    for (id, started_at, active_seconds) in &rows {
        stats.active_seconds += *active_seconds;
        let raw = load_items(conn, id)?;
        let items = valuate_items(conn, &raw, cfg, now, &mut cache)?;
        let lines: Vec<ItemValue> = items
            .iter()
            .map(|i| ItemValue { qty: i.qty, unit_then: i.unit_then, unit_now: i.unit_now })
            .collect();
        let totals = totals_for(&lines);
        stats.value_then += totals.value_then.round() as i64;
        stats.value_now += totals.value_now.round() as i64;
        stats.then_partial |= totals.then_partial;
        stats.now_partial |= totals.now_partial;

        let day: String = conn.query_row(
            "SELECT date(?1, 'unixepoch')",
            rusqlite::params![started_at],
            |r| r.get(0),
        )?;
        let day_acc = days_map.entry(day).or_insert(DayAcc { sessions: 0, seconds: 0, then: 0.0, now: 0.0 });
        day_acc.sessions += 1;
        day_acc.seconds += *active_seconds;
        day_acc.then += totals.value_then;
        day_acc.now += totals.value_now;

        for item in &items {
            let key = item
                .resource_id
                .clone()
                .unwrap_or_else(|| format!("item:{}", item.item_id));
            let res_acc = res_map.entry(key).or_insert_with(|| ResAcc {
                display_name: item.display_name.clone(),
                qty: 0.0,
                then: 0.0,
                now: 0.0,
            });
            res_acc.qty += item.qty.max(0.0);
            res_acc.then += item.value_then.unwrap_or(0) as f64;
            res_acc.now += item.value_now.unwrap_or(0) as f64;
        }
    }
    let mut by_day: Vec<DayBucket> = days_map
        .into_iter()
        .map(|(day, acc)| DayBucket {
            day,
            sessions: acc.sessions,
            active_seconds: acc.seconds,
            value_then: acc.then.round() as i64,
            value_now: acc.now.round() as i64,
        })
        .collect();
    by_day.sort_by(|a, b| b.day.cmp(&a.day));
    stats.by_day = by_day;

    let mut by_resource: Vec<ResourceBucket> = res_map
        .into_iter()
        .map(|(key, acc)| ResourceBucket {
            key,
            display_name: acc.display_name,
            qty: acc.qty,
            value_then: acc.then.round() as i64,
            value_now: acc.now.round() as i64,
        })
        .collect();
    by_resource.sort_by(|a, b| b.value_now.cmp(&a.value_now).then(a.key.cmp(&b.key)));
    stats.by_resource = by_resource;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::open_memory;

    const NOW: i64 = 1788868800; // 2026-09-08T12:00:00Z

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn mem_db() -> Connection {
        let mut conn = open_memory().unwrap();
        crate::migrations::migrate(&mut conn).unwrap();
        conn
    }

    fn opt_f64(v: &serde_json::Value) -> Option<f64> {
        if v.is_null() {
            None
        } else {
            Some(v.as_f64().unwrap())
        }
    }

    #[test]
    fn math_matches_shared_fixture() {
        let path = format!("{}/../../fixtures/expected/session.json", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        for case in v["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let items: Vec<ItemValue> = case["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| ItemValue {
                    qty: i["qty"].as_f64().unwrap(),
                    unit_then: i["unitThen"].as_i64(),
                    unit_now: i["unitNow"].as_i64(),
                })
                .collect();
            let t = totals_for(&items);
            let ex = &case["expect"];
            assert!(approx(t.qty, ex["totalQty"].as_f64().unwrap(), 1e-9), "{id}: qty");
            assert!(approx(t.value_then, ex["valueThen"].as_f64().unwrap(), 1e-9), "{id}: then");
            assert!(approx(t.value_now, ex["valueNow"].as_f64().unwrap(), 1e-9), "{id}: now");
            assert_eq!(t.then_partial, ex["thenPartial"].as_bool().unwrap(), "{id}: thenPartial");
            assert_eq!(t.now_partial, ex["nowPartial"].as_bool().unwrap(), "{id}: nowPartial");
            let active = case["activeSeconds"].as_i64().unwrap();
            let rate = case["expectedGph"].as_f64().unwrap();
            assert_eq!(gph_for(t.value_then, active), opt_f64(&ex["gphThen"]), "{id}: gphThen");
            assert_eq!(gph_for(t.value_now, active), opt_f64(&ex["gphNow"]), "{id}: gphNow");
            let exp = expected_total(rate, active);
            assert!(approx(exp, ex["expectedTotal"].as_f64().unwrap(), 1e-9), "{id}: expTotal");
            assert_eq!(delta_pct(t.value_then, exp), opt_f64(&ex["deltaThen"]), "{id}: dThen");
            assert_eq!(delta_pct(t.value_now, exp), opt_f64(&ex["deltaNow"]), "{id}: dNow");
        }
    }

    /// Two sessions across two UTC days + market rows for both loot items.
    fn seeded_db() -> Connection {
        let conn = mem_db();
        conn.execute(
            "INSERT INTO expansions (id, name, short) VALUES ('expansion-test', 'Test', 'T')",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO continents (id, expansion_id, name)
             VALUES ('continent-test', 'expansion-test', 'Test Continent')",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO zones (id, continent_id, name, map_id, yards_per_unit)
             VALUES ('zone-test', 'continent-test', 'Test Zone', 1, 10.0)",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO farm_categories (id, key, name) VALUES ('mining', 'mining', 'Mining')",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO resources (id, category_id, key, name, item_id) VALUES
             ('bismuth', 'mining', 'bismuth', 'Bismuth', 161502)",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO market_items (item_id, region, market_value, min_buyout, recent,
             historical, avg_sale_price, sale_rate, sold_per_day, item_name,
             market_updated_at, region_updated_at, fetched_at)
             VALUES
             (161502, 'eu', 52000, 51000, 51500, 50000, 49800, 0.42, 18.5,
              'Bismuth', 1788868800, 1788825600, 1788868800),
             (161504, 'eu', 125000, 120000, 122000, 110000, 115000, 0.61, 33.25,
              'Mycobloom', 1788868800, 1788825600, 1788868800)",
            rusqlite::params![],
        )
        .unwrap();
        // Day 1 (Sep 7, 11:00 UTC): 10 bismuth + 5 mycobloom, one hour active.
        conn.execute(
            "INSERT INTO farm_sessions (id, external_id, character_name, realm, zone_id,
             status, started_at, ended_at, active_seconds, notes, created_at, updated_at)
             VALUES ('s1', 's1', 'Testtoon', 'Realm', 'zone-test',
             'imported', 1788778800, 1788786000, 3600, '', 1788778800, 1788778800)",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO farm_session_items (session_id, item_id, resource_id, qty) VALUES
             ('s1', 161502, 'bismuth', 10.0),
             ('s1', 161504, NULL, 5.0)",
            rusqlite::params![],
        )
        .unwrap();
        // Day 2: 2 bismuth, 30 min active, no market gap.
        conn.execute(
            "INSERT INTO farm_sessions (id, external_id, character_name, realm, zone_id,
             status, started_at, ended_at, active_seconds, notes, created_at, updated_at)
             VALUES ('s2', 's2', 'Testtoon', 'Realm', NULL,
             'imported', 1788868800, 1788870600, 1800, '', 1788868800, 1788868800)",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO farm_session_items (session_id, item_id, resource_id, qty)
             VALUES ('s2', 161502, 'bismuth', 2.0)",
            rusqlite::params![],
        )
        .unwrap();
        conn
    }

    #[test]
    fn analyze_snapshots_first_price_wins() {
        let conn = seeded_db();
        let cfg = ValuationConfig::default();
        let detail = analyze_session(&conn, "s1", &cfg, NOW).unwrap();
        assert_eq!(detail.summary.status, "analyzed");
        // Bismuth effective 46912, mycobloom 105710 (see valuation.json).
        let bismuth = detail.items.iter().find(|i| i.item_id == 161502).unwrap();
        assert_eq!(bismuth.unit_then, Some(46_912));
        assert_eq!(bismuth.display_name, "Bismuth");
        let myco = detail.items.iter().find(|i| i.item_id == 161504).unwrap();
        assert_eq!(myco.unit_then, Some(105_710));
        assert_eq!(myco.display_name, "Mycobloom"); // market name, no resource link
        assert!(!detail.summary.then_partial);
        assert_eq!(detail.summary.value_then, 10 * 46_912 + 5 * 105_710);

        // Market moves → reanalyze keeps the original snapshots.
        conn.execute(
            "UPDATE market_items
             SET market_value = 999999, min_buyout = 200000,
                 historical = 200000, avg_sale_price = 200000
             WHERE item_id = 161502",
            rusqlite::params![],
        )
        .unwrap();
        let again = analyze_session(&conn, "s1", &cfg, NOW).unwrap();
        let b2 = again.items.iter().find(|i| i.item_id == 161502).unwrap();
        assert_eq!(b2.unit_then, Some(46_912));
        assert_ne!(b2.unit_now, Some(46_912));

        assert!(analyze_session(&conn, "nope", &cfg, NOW).is_err());
    }

    #[test]
    fn link_route_attaches_expectations() {
        let conn = seeded_db();
        conn.execute(
            "INSERT INTO saved_routes (id, name, zone_id, resource_ids_json, waypoints_json,
             excluded_json, config_json, stats_json, created_at, updated_at)
             VALUES ('rt-1', 'Loop', 'zone-test', '[]', '[]', '[]', '{}',
             '{\"gphExpected\": 1300000.0, \"effectiveValue\": 755200.0}', 1, 1)",
            rusqlite::params![],
        )
        .unwrap();
        let cfg = ValuationConfig::default();
        analyze_session(&conn, "s1", &cfg, NOW).unwrap();
        let linked = link_route(&conn, "s1", Some("rt-1"), NOW).unwrap();
        assert_eq!(linked.summary.expected_gph, Some(1_300_000));
        assert_eq!(linked.expected_value_per_loop, Some(755_200));
        assert_eq!(linked.route_name.as_deref(), Some("Loop"));
        // One active hour → expected total = 1.3M; actual ≈ 997k → ≈ -23.3%.
        assert_eq!(linked.expected_total, Some(1_300_000));
        let d = linked.delta_pct_then.unwrap();
        assert!(approx(d, -23.3, 0.1), "{d}");
        assert!(link_route(&conn, "s1", Some("rt-nope"), NOW).is_err());

        let unlinked = link_route(&conn, "s1", None, NOW).unwrap();
        assert_eq!(unlinked.summary.expected_gph, None);
        assert_eq!(unlinked.expected_total, None);
        assert_eq!(unlinked.delta_pct_then, None);
    }

    #[test]
    fn list_filters_and_stats_bucket() {
        let conn = seeded_db();
        let cfg = ValuationConfig::default();
        let all = list_sessions(&conn, None, 10, &cfg, NOW).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "s2"); // newest first
        let zoned = list_sessions(&conn, Some("zone-test"), 10, &cfg, NOW).unwrap();
        assert_eq!(zoned.len(), 1);

        let stats = session_stats(&conn, None, None, &cfg, NOW).unwrap();
        assert_eq!(stats.session_count, 2);
        assert_eq!(stats.active_seconds, 5400);
        assert_eq!(stats.by_day.len(), 2);
        assert_eq!(stats.by_day[0].day, "2026-09-08");
        let res = stats.by_resource.iter().find(|b| b.key == "bismuth").unwrap();
        assert!(approx(res.qty, 12.0, 1e-9));
        assert_eq!(res.display_name, "Bismuth");
        // Unmapped mycobloom buckets under its item key.
        assert!(stats.by_resource.iter().any(|b| b.key == "item:161504"));

        let day_only = session_stats(&conn, None, Some(1), &cfg, NOW).unwrap();
        assert_eq!(day_only.session_count, 1);
    }

    #[test]
    fn display_name_falls_back_to_bare_id() {
        let conn = seeded_db();
        conn.execute(
            "INSERT INTO farm_sessions (id, status, started_at, active_seconds)
             VALUES ('s3', 'imported', 1788868800, 60)",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO farm_session_items (session_id, item_id, qty)
             VALUES ('s3', 424242, 1.0)",
            rusqlite::params![],
        )
        .unwrap();
        let detail = analyze_session(&conn, "s3", &ValuationConfig::default(), NOW).unwrap();
        assert_eq!(detail.items[0].display_name, "Item #424242");
        assert!(detail.summary.then_partial); // no market data → no snapshot
        assert!(detail.summary.now_partial);
    }
}

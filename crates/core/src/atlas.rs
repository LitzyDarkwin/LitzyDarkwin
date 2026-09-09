//! Farming Atlas domain service.
//!
//! Phase 2 ships the canonical-entity helpers the addon bridge needs
//! (`ensure_*`); browsing, search and density APIs arrive in Phase 3.
//! Canonical rows are never provider-shaped: provider data is normalized here.

use crate::error::Result;
use crate::models::category;
use rusqlite::{Connection, OptionalExtension};

/// Placeholder hierarchy for zones the seed data does not know (yet).
pub const UNKNOWN_EXPANSION: &str = "unknown";
pub const UNKNOWN_CONTINENT: &str = "unknown";

const DEFAULT_CATEGORIES: &[(&str, &str)] = &[
    (category::MINING, "Mining"),
    (category::HERBALISM, "Herbalism"),
    (category::FISHING, "Fishing"),
    (category::SKINNING, "Skinning"),
    (category::TREASURE, "Treasure"),
    (category::MOB, "Mob farming"),
    (category::OTHER, "Other"),
];

/// Insert the 7 canonical categories if missing. Idempotent.
pub fn ensure_default_categories(conn: &Connection) -> Result<()> {
    for (key, name) in DEFAULT_CATEGORIES {
        conn.execute(
            "INSERT INTO farm_categories (id, key, name) VALUES (?1, ?1, ?2)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![key, name],
        )?;
    }
    Ok(())
}

/// Map an addon-side category hint to a canonical category key.
pub fn category_for_addon_cat(addon_cat: Option<&str>) -> &'static str {
    match addon_cat.unwrap_or("").trim().to_lowercase().as_str() {
        "mining" => category::MINING,
        "herbalism" => category::HERBALISM,
        "fishing" => category::FISHING,
        "skinning" => category::SKINNING,
        "treasure" => category::TREASURE,
        "mob" => category::MOB,
        _ => category::OTHER,
    }
}

/// Lowercase slug for display names (`"Ghost Iron Ore"` → `"ghost-iron-ore"`).
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut dash = false;
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.len() > 48 {
        out.truncate(48);
        while out.ends_with('-') {
            out.pop();
        }
    }
    if out.is_empty() {
        out.push_str("unknown");
    }
    out
}

fn ensure_unknown_hierarchy(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO expansions (id, name, short, sort_order) VALUES (?1, 'Unknown', '?', 9999)
         ON CONFLICT(id) DO NOTHING",
        rusqlite::params![UNKNOWN_EXPANSION],
    )?;
    conn.execute(
        "INSERT INTO continents (id, expansion_id, name) VALUES (?1, ?2, 'Unknown')
         ON CONFLICT(id) DO NOTHING",
        rusqlite::params![UNKNOWN_CONTINENT, UNKNOWN_EXPANSION],
    )?;
    Ok(())
}

/// Resolve (or lazily create) the canonical zone for a map sighting.
/// Lookup order: `map_id` → case-insensitive name → create placeholder.
pub fn ensure_zone_for_map(
    conn: &Connection,
    map_id: Option<i64>,
    zone_name: Option<&str>,
) -> Result<String> {
    if let Some(map) = map_id {
        let found: Option<String> = conn
            .query_row("SELECT id FROM zones WHERE map_id = ?1", rusqlite::params![map], |r| r.get(0))
            .optional()?;
        if let Some(id) = found {
            return Ok(id);
        }
    }
    if let Some(name) = zone_name.map(str::trim).filter(|n| !n.is_empty()) {
        let found: Option<String> = conn
            .query_row(
                "SELECT id FROM zones WHERE lower(name) = lower(?1) LIMIT 1",
                rusqlite::params![name],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = found {
            // Backfill the map id for next time.
            if let Some(map) = map_id {
                conn.execute(
                    "UPDATE zones SET map_id = ?1 WHERE id = ?2 AND map_id IS NULL",
                    rusqlite::params![map, id],
                )?;
            }
            return Ok(id);
        }
    }
    ensure_unknown_hierarchy(conn)?;
    let id = match map_id {
        Some(map) => format!("map-{map}"),
        None => format!("zone-{}", slugify(zone_name.unwrap_or("unknown"))),
    };
    let name = zone_name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("Unknown zone")
        .to_string();
    conn.execute(
        "INSERT INTO zones (id, continent_id, name, map_id) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO NOTHING",
        rusqlite::params![id, UNKNOWN_CONTINENT, name, map_id],
    )?;
    Ok(id)
}

/// Resolve (or lazily create) the canonical resource for a loot sighting.
/// Stable keys: `item:<itemID>` when known, else `name:<slug>`.
pub fn ensure_resource_for_item(
    conn: &Connection,
    item_id: Option<i64>,
    name: Option<&str>,
    addon_cat: Option<&str>,
) -> Result<String> {
    ensure_default_categories(conn)?;
    let key = match item_id {
        Some(id) => format!("item:{id}"),
        None => format!("name:{}", slugify(name.unwrap_or("unknown"))),
    };
    let found: Option<String> = conn
        .query_row("SELECT id FROM resources WHERE key = ?1", rusqlite::params![key], |r| r.get(0))
        .optional()?;
    if let Some(id) = found {
        if let Some(item) = item_id {
            conn.execute(
                "UPDATE resources SET item_id = ?1 WHERE id = ?2 AND item_id IS NULL",
                rusqlite::params![item, id],
            )?;
        }
        return Ok(id);
    }
    let display = name.map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Unknown loot");
    let cat = category_for_addon_cat(addon_cat);
    conn.execute(
        "INSERT INTO resources (id, category_id, key, name, item_id) VALUES (?1, ?2, ?1, ?3, ?4)
         ON CONFLICT(id) DO NOTHING",
        rusqlite::params![key, cat, display, item_id],
    )?;
    Ok(key)
}

// Atlas query service (Phase 3) --------------------------------------------------
// NOTE: module-level `use` may legally appear here; it keeps the Phase 2
// ensure-helpers diff clean.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeZone {
    pub id: String,
    pub name: String,
    pub map_id: Option<i64>,
    pub observation_count: i64,
    pub resource_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeContinent {
    pub id: String,
    pub name: String,
    pub zones: Vec<TreeZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeExpansion {
    pub id: String,
    pub name: String,
    pub short: String,
    pub continents: Vec<TreeContinent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasTree {
    pub expansions: Vec<TreeExpansion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceHit {
    pub id: String,
    pub name: String,
    pub category_id: String,
    pub category_name: String,
    pub item_id: Option<i64>,
    pub observation_count: i64,
    pub zones: Vec<ZoneRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneResourceSummary {
    pub resource_id: String,
    pub resource_name: String,
    pub category_id: String,
    pub item_id: Option<i64>,
    pub observations: i64,
    pub sources: Vec<String>,
    pub avg_yield: Option<f64>,
    pub newest_observation: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneDetail {
    pub id: String,
    pub name: String,
    pub map_id: Option<i64>,
    pub continent_id: String,
    pub continent_name: String,
    pub expansion_id: String,
    pub expansion_name: String,
    pub yards_per_unit: f64,
    pub observation_count: i64,
    pub resources: Vec<ZoneResourceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCount {
    pub resource_id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridCell {
    pub gx: i64,
    pub gy: i64,
    /// Centroid of member points (0-100 map space, y grows downward like WoW maps).
    pub cx: f64,
    pub cy: f64,
    pub count: usize,
    pub top_resource: Option<String>,
    pub by_resource: Vec<ResourceCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusteredObservations {
    pub grid: usize,
    pub cells: Vec<GridCell>,
    pub total_points: usize,
    pub truncated: bool,
}

/// Hard cap on raw observation rows scanned per map request (density-first UI).
pub const MAX_OBSERVATION_ROWS: usize = 5000;

/// Full hierarchy with per-zone observation/resource counts.
pub fn tree(conn: &Connection) -> Result<AtlasTree> {
    let mut expansions = Vec::new();
    let exp_rows: Vec<(String, String, String)> = conn
        .prepare("SELECT id, name, short FROM expansions ORDER BY sort_order, name")?
        .query_map(rusqlite::params![], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<std::result::Result<_, _>>()?;
    for (exp_id, exp_name, exp_short) in exp_rows {
        let mut continents = Vec::new();
        let cont_rows: Vec<(String, String)> = conn
            .prepare("SELECT id, name FROM continents WHERE expansion_id = ?1 ORDER BY name")?
            .query_map(rusqlite::params![exp_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?;
        for (cont_id, cont_name) in cont_rows {
            let zones: Vec<TreeZone> = conn
                .prepare(
                    "SELECT z.id, z.name, z.map_id, COUNT(o.id), COUNT(DISTINCT o.resource_id)
                     FROM zones z LEFT JOIN resource_observations o ON o.zone_id = z.id
                     WHERE z.continent_id = ?1 GROUP BY z.id ORDER BY z.name",
                )?
                .query_map(rusqlite::params![cont_id], |r| {
                    Ok(TreeZone {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        map_id: r.get(2)?,
                        observation_count: r.get(3)?,
                        resource_count: r.get(4)?,
                    })
                })?
                .collect::<std::result::Result<_, _>>()?;
            continents.push(TreeContinent { id: cont_id, name: cont_name, zones });
        }
        expansions.push(TreeExpansion {
            id: exp_id,
            name: exp_name,
            short: exp_short,
            continents,
        });
    }
    Ok(AtlasTree { expansions })
}

fn escape_like(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 2);
    for ch in query.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Search resources by name/key with optional category filter.
pub fn search_resources(
    conn: &Connection,
    query: &str,
    category: Option<&str>,
    limit: usize,
) -> Result<Vec<ResourceHit>> {
    let limit = limit.clamp(1, 100) as i64;
    let pattern = format!("%{}%", escape_like(query.trim()));
    let mut sql = String::from(
        "SELECT r.id, r.name, r.category_id, c.name, r.item_id, COUNT(o.id)
         FROM resources r JOIN farm_categories c ON c.id = r.category_id
         LEFT JOIN resource_observations o ON o.resource_id = r.id
         WHERE (r.name LIKE ?1 ESCAPE '\\' OR r.key LIKE ?1 ESCAPE '\\')",
    );
    if category.is_some() {
        sql.push_str(" AND r.category_id = ?2");
    }
    sql.push_str(" GROUP BY r.id ORDER BY COUNT(o.id) DESC, r.name LIMIT ?");
    // LIMIT placeholder index depends on whether the category filter exists.
    let rows: Vec<(String, String, String, String, Option<i64>, i64)> = if let Some(cat) = category
    {
        conn.prepare(&sql)?
            .query_map(rusqlite::params![pattern, cat, limit], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
            })?
            .collect::<std::result::Result<_, _>>()?
    } else {
        conn.prepare(&sql)?
            .query_map(rusqlite::params![pattern, limit], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
            })?
            .collect::<std::result::Result<_, _>>()?
    };
    let mut hits = Vec::with_capacity(rows.len());
    for (id, name, category_id, category_name, item_id, observation_count) in rows {
        let zones: Vec<ZoneRef> = conn
            .prepare(
                "SELECT DISTINCT z.id, z.name FROM zones z
                 JOIN resource_observations o ON o.zone_id = z.id
                 WHERE o.resource_id = ?1 ORDER BY z.name",
            )?
            .query_map(rusqlite::params![id], |r| {
                Ok(ZoneRef { id: r.get(0)?, name: r.get(1)? })
            })?
            .collect::<std::result::Result<_, _>>()?;
        hits.push(ResourceHit {
            id,
            name,
            category_id,
            category_name,
            item_id,
            observation_count,
            zones,
        });
    }
    Ok(hits)
}

/// Zone header + per-resource summary (counts, sources, yield, recency).
pub fn zone_detail(conn: &Connection, zone_id: &str) -> Result<ZoneDetail> {
    let header: Option<(String, Option<i64>, String, String, String, String, f64, i64)> = conn
        .query_row(
            "SELECT z.name, z.map_id, z.continent_id, c.name, c.expansion_id, e.name,
                    z.yards_per_unit, COUNT(o.id)
             FROM zones z JOIN continents c ON c.id = z.continent_id
             JOIN expansions e ON e.id = c.expansion_id
             LEFT JOIN resource_observations o ON o.zone_id = z.id
             WHERE z.id = ?1 GROUP BY z.id",
            rusqlite::params![zone_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )
        .optional()?;
    let Some((name, map_id, continent_id, continent_name, expansion_id, expansion_name, ypu, total)) =
        header
    else {
        return Err(crate::error::AppError::validation(format!("unknown zone `{zone_id}`")));
    };
    let resources: Vec<ZoneResourceSummary> = conn
        .prepare(
            "SELECT r.id, r.name, r.category_id, r.item_id, COUNT(o.id),
                    GROUP_CONCAT(DISTINCT o.source), AVG(o.yield_qty), MAX(o.observed_at)
             FROM resource_observations o JOIN resources r ON r.id = o.resource_id
             WHERE o.zone_id = ?1 GROUP BY r.id ORDER BY COUNT(o.id) DESC, r.name",
        )?
        .query_map(rusqlite::params![zone_id], |r| {
            let sources_csv: Option<String> = r.get(5)?;
            let mut sources: Vec<String> = sources_csv
                .map(|csv| csv.split(',').map(str::to_string).collect())
                .unwrap_or_default();
            sources.sort();
            Ok(ZoneResourceSummary {
                resource_id: r.get(0)?,
                resource_name: r.get(1)?,
                category_id: r.get(2)?,
                item_id: r.get(3)?,
                observations: r.get(4)?,
                sources,
                avg_yield: r.get(6)?,
                newest_observation: r.get(7)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;
    Ok(ZoneDetail {
        id: zone_id.to_string(),
        name,
        map_id,
        continent_id,
        continent_name,
        expansion_id,
        expansion_name,
        yards_per_unit: ypu,
        observation_count: total,
        resources,
    })
}

#[derive(Default)]
struct CellAcc {
    count: usize,
    sum_x: f64,
    sum_y: f64,
    by_resource: std::collections::BTreeMap<String, usize>,
}

/// Grid-aggregated observations for density rendering (never raw thousands of points).
pub fn zone_observations(
    conn: &Connection,
    zone_id: &str,
    resource_ids: &[String],
    grid: usize,
) -> Result<ClusteredObservations> {
    let grid = grid.clamp(4, 50);
    let mut sql = String::from(
        "SELECT x, y, resource_id FROM resource_observations WHERE zone_id = ?1",
    );
    if !resource_ids.is_empty() {
        let placeholders = vec!["?"; resource_ids.len()].join(", ");
        sql.push_str(&format!(" AND resource_id IN ({placeholders})"));
    }
    let cap = (MAX_OBSERVATION_ROWS + 1) as i64;
    sql.push_str(" LIMIT ?");

    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(resource_ids.len() + 2);
    params.push(&zone_id);
    for id in resource_ids {
        params.push(id);
    }
    params.push(&cap);

    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<(f64, f64, String)> = stmt
        .query_map(params.as_slice(), |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<std::result::Result<_, _>>()?;
    let truncated = rows.len() > MAX_OBSERVATION_ROWS;

    // Row-major accumulation for deterministic output order.
    let mut cells: std::collections::BTreeMap<(i64, i64), CellAcc> = Default::default();
    for (x, y, resource_id) in rows.into_iter().take(MAX_OBSERVATION_ROWS) {
        let gx = ((x / 100.0 * grid as f64).floor() as i64).clamp(0, grid as i64 - 1);
        let gy = ((y / 100.0 * grid as f64).floor() as i64).clamp(0, grid as i64 - 1);
        let acc = cells.entry((gy, gx)).or_default();
        acc.count += 1;
        acc.sum_x += x;
        acc.sum_y += y;
        *acc.by_resource.entry(resource_id).or_insert(0) += 1;
    }

    let mut out = Vec::with_capacity(cells.len());
    let mut total_points = 0usize;
    for ((gy, gx), acc) in cells {
        total_points += acc.count;
        let mut by_resource: Vec<ResourceCount> = acc
            .by_resource
            .into_iter()
            .map(|(resource_id, count)| ResourceCount { resource_id, count })
            .collect();
        by_resource.sort_by(|a, b| b.count.cmp(&a.count).then(a.resource_id.cmp(&b.resource_id)));
        out.push(GridCell {
            gx,
            gy,
            cx: acc.sum_x / acc.count as f64,
            cy: acc.sum_y / acc.count as f64,
            count: acc.count,
            top_resource: by_resource.first().map(|rc| rc.resource_id.clone()),
            by_resource,
        });
    }
    Ok(ClusteredObservations { grid, cells: out, total_points, truncated })
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
    fn categories_are_idempotent() {
        let conn = db();
        ensure_default_categories(&conn).unwrap();
        ensure_default_categories(&conn).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM farm_categories", rusqlite::params![], |r| r.get(0)).unwrap();
        assert_eq!(n, 7);
    }

    #[test]
    fn zone_lookup_prefers_map_id_then_backfills() {
        let conn = db();
        let a = ensure_zone_for_map(&conn, None, Some("Test Zone")).unwrap();
        let b = ensure_zone_for_map(&conn, Some(2346), Some("Test Zone")).unwrap();
        assert_eq!(a, b);
        let map: Option<i64> =
            conn.query_row("SELECT map_id FROM zones WHERE id = ?1", rusqlite::params![a], |r| r.get(0)).unwrap();
        assert_eq!(map, Some(2346));
        // Second sighting with the map id resolves directly.
        assert_eq!(ensure_zone_for_map(&conn, Some(2346), None).unwrap(), a);
    }

    #[test]
    fn resource_keys_are_stable() {
        let conn = db();
        let a = ensure_resource_for_item(&conn, Some(123), Some("Ghost Iron Ore"), Some("mining")).unwrap();
        let b = ensure_resource_for_item(&conn, Some(123), Some("Ghost Iron Ore"), Some("mining")).unwrap();
        assert_eq!(a, "item:123");
        assert_eq!(a, b);
        let c = ensure_resource_for_item(&conn, None, Some("Weird Fungus"), Some("herbalism")).unwrap();
        assert_eq!(c, "name:weird-fungus");
    }

    #[test]
    fn slugify_handles_garbage() {
        assert_eq!(slugify("Ghost Iron Ore"), "ghost-iron-ore");
        assert_eq!(slugify("  --A  B-- "), "a-b");
        assert_eq!(slugify("!!!"), "unknown");
    }

    fn seeded_db() -> Connection {
        let mut conn = db();
        crate::seed::load(&mut conn).unwrap();
        conn
    }

    #[test]
    fn tree_has_hierarchy_and_counts() {
        let conn = seeded_db();
        let t = tree(&conn).unwrap();
        assert_eq!(t.expansions.len(), 2);
        assert_eq!(t.expansions[0].id, "df"); // sort_order 90 before TWW 100
        let tww = t.expansions.iter().find(|e| e.id == "tww").unwrap();
        assert_eq!(tww.continents.len(), 1);
        assert_eq!(tww.continents[0].zones.len(), 4);
        let dorn = tww.continents[0].zones.iter().find(|z| z.id == "isle-of-dorn").unwrap();
        assert!(dorn.observation_count > 50);
        assert_eq!(dorn.resource_count, 3); // bismuth, ironclaw, mycobloom
    }

    #[test]
    fn search_matches_names_and_filters_categories() {
        let conn = seeded_db();
        let hits = search_resources(&conn, "ore", None, 10).unwrap();
        assert_eq!(hits.len(), 3); // ironclaw/serevite/draconium ("Bismuth" has no "ore")
        assert!(hits.iter().all(|h| h.name.to_lowercase().contains("ore")));
        let herbs = search_resources(&conn, "", Some("herbalism"), 50).unwrap();
        assert_eq!(herbs.len(), 4);
        assert!(herbs.iter().all(|h| h.category_id == "herbalism"));
        // LIKE metacharacters are escaped, not wildcards.
        assert!(search_resources(&conn, "%", None, 10).unwrap().is_empty());
        // Zones attached to hits.
        let myco = search_resources(&conn, "mycobloom", None, 5).unwrap();
        assert_eq!(myco.len(), 1);
        assert_eq!(myco[0].zones.len(), 4);
    }

    #[test]
    fn zone_detail_reports_per_resource_summary() {
        let conn = seeded_db();
        let d = zone_detail(&conn, "ringing-deeps").unwrap();
        assert_eq!(d.name, "The Ringing Deeps");
        assert_eq!(d.expansion_id, "tww");
        assert_eq!(d.resources.len(), 4); // bismuth, ironclaw, myco, luredrop
        let luredrop = d.resources.iter().find(|r| r.resource_id == "luredrop").unwrap();
        assert_eq!(luredrop.sources, vec!["seed".to_string()]);
        assert!(luredrop.avg_yield.unwrap() >= 2.0);
        assert!(zone_detail(&conn, "nope").is_err());
    }

    #[test]
    fn observations_grid_is_deterministic_and_capped() {
        let conn = seeded_db();
        let a = zone_observations(&conn, "isle-of-dorn", &[], 10).unwrap();
        let b = zone_observations(&conn, "isle-of-dorn", &[], 10).unwrap();
        assert_eq!(a.grid, 10);
        assert!(!a.truncated);
        assert!(a.total_points > 50);
        // Deterministic: same cells in the same order.
        assert_eq!(a.cells.len(), b.cells.len());
        for (ca, cb) in a.cells.iter().zip(b.cells.iter()) {
            assert_eq!((ca.gx, ca.gy, ca.count), (cb.gx, cb.gy, cb.count));
        }
        let sum: usize = a.cells.iter().map(|c| c.count).sum();
        assert_eq!(sum, a.total_points);
        // Resource filter narrows to one resource.
        let one = zone_observations(&conn, "isle-of-dorn", &["mycobloom".to_string()], 25).unwrap();
        assert!(one.total_points > 0 && one.total_points < a.total_points);
        for cell in &one.cells {
            assert_eq!(cell.by_resource.len(), 1);
            assert_eq!(cell.by_resource[0].resource_id, "mycobloom");
        }
        // Grid clamps to 4..=50.
        assert_eq!(zone_observations(&conn, "isle-of-dorn", &[], 0).unwrap().grid, 4);
        assert_eq!(zone_observations(&conn, "isle-of-dorn", &[], 999).unwrap().grid, 50);
    }
}

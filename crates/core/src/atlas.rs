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
}

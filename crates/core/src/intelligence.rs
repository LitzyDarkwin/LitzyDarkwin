//! Personal adaptive intelligence (Phase 7).
//!
//! Learns yields, zone efficiency and GPH stats from analyzed sessions via
//! full rebuilds (deterministic, no drift), then blends personal evidence
//! with priors per ALGORITHMS.md §7. Crowd evidence joins in Phase 8.
//!
//! Honesty notes: `hit_rate` is reserved (the addon reports no attempts, so
//! nothing is learned there); travel speed and gather time need loop counting
//! the addon does not persist yet — both stay documented priors.

use crate::error::{AppError, Result};
use crate::routes::{self, CandidateDto, RouteConfig, RouteObservation};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Blending (ALGORITHMS.md §7) ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendConfig {
    pub min_samples: f64,
    pub half_life_days: f64,
    pub prior_weight: f64,
}

impl Default for BlendConfig {
    fn default() -> Self {
        BlendConfig { min_samples: 10.0, half_life_days: 21.0, prior_weight: 1.0 }
    }
}

/// Exponential recency decay: 1.0 at age 0, 0.5 at one half-life.
pub fn recency_decay(age_seconds: i64, half_life_days: f64) -> f64 {
    if half_life_days <= 0.0 {
        return 0.0;
    }
    0.5f64.powf(age_seconds.max(0) as f64 / (half_life_days * 86400.0))
}

/// `min(samples / min_samples, 1) × recency_decay(age)`.
pub fn personal_weight(
    samples: f64,
    updated_at: i64,
    cfg: &BlendConfig,
    now: i64,
) -> f64 {
    (samples.max(0.0) / cfg.min_samples.max(1e-9)).min(1.0)
        * recency_decay(now - updated_at, cfg.half_life_days)
}

#[derive(Debug, Clone)]
pub struct Evidence {
    pub value: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlendedValue {
    pub value: f64,
    pub personal_weight: f64,
    pub used_personal: bool,
    pub explain: String,
}

/// Three-way blend with optional legs (crowd arrives in Phase 8).
/// Zero total weight falls back to the prior rather than dividing by zero.
pub fn blend(
    prior: f64,
    personal: Option<Evidence>,
    crowd: Option<Evidence>,
    cfg: &BlendConfig,
) -> BlendedValue {
    let pw = cfg.prior_weight.max(0.0);
    let mut num = prior * pw;
    let mut den = pw;
    let mut personal_w = 0.0;
    if let Some(e) = personal {
        if e.weight > 0.0 {
            num += e.value * e.weight;
            den += e.weight;
            personal_w = e.weight;
        }
    }
    if let Some(e) = crowd {
        if e.weight > 0.0 {
            num += e.value * e.weight;
            den += e.weight;
        }
    }
    let value = if den > 0.0 { num / den } else { prior };
    let explain = if personal_w > 0.0 {
        format!("prior {prior:.2} × {pw:.2} + personal × {personal_w:.2} → {value:.2}")
    } else {
        format!("no personal evidence yet — prior {prior:.2} unchanged")
    };
    BlendedValue { value, personal_weight: personal_w, used_personal: personal_w > 0.0, explain }
}

// Learning ------------------------------------------------------------------------------

pub const METRIC_EFFICIENCY: &str = "efficiency_global";
pub const METRIC_SESSIONS: &str = "sessions_analyzed";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildReport {
    pub sessions: i64,
    pub resources: i64,
    pub zones: i64,
}

/// Full rebuild of the personal tables from analyzed sessions.
/// Efficiency only ever compares linked sessions (value vs their own
/// expectations); unlinked sessions shape GPH stats but never efficiency.
pub fn rebuild_personal(conn: &Connection, now: i64) -> Result<RebuildReport> {
    conn.execute("DELETE FROM personal_resource_stats", rusqlite::params![])?;
    conn.execute(
        "INSERT INTO personal_resource_stats
           (resource_id, gathers, total_qty, avg_yield, avg_gold_per_gather, updated_at)
         SELECT i.resource_id,
           SUM(i.gathers),
           SUM(i.qty),
           SUM(i.qty) * 1.0 / SUM(i.gathers),
           COALESCE(
             SUM(CASE WHEN i.unit_value IS NULL THEN 0 ELSE i.qty * i.unit_value END) * 1.0 /
             NULLIF(SUM(CASE WHEN i.unit_value IS NULL THEN 0 ELSE i.gathers END), 0),
             0.0),
           ?1
         FROM farm_session_items i
         JOIN farm_sessions s ON s.id = i.session_id
         WHERE s.status = 'analyzed' AND i.resource_id IS NOT NULL AND i.gathers > 0
         GROUP BY i.resource_id",
        rusqlite::params![now],
    )?;
    let resources = conn.changes() as i64;

    conn.execute("DELETE FROM personal_zone_stats", rusqlite::params![])?;
    conn.execute(
        "INSERT INTO personal_zone_stats
           (zone_id, sessions, total_seconds, avg_gph, efficiency, updated_at)
         SELECT zone_id, COUNT(*), SUM(active_seconds),
           COALESCE(SUM(vt) * 3600.0 / NULLIF(SUM(active_seconds), 0), 0.0),
           COALESCE(
             SUM(CASE WHEN expected_gph IS NULL THEN 0 ELSE vt END) /
             NULLIF(SUM(CASE WHEN expected_gph IS NULL THEN 0
                      ELSE expected_gph * active_seconds / 3600.0 END), 0),
             1.0),
           ?1
         FROM (SELECT s.zone_id, s.active_seconds, s.expected_gph,
                 (SELECT COALESCE(SUM(qty * unit_value), 0) FROM farm_session_items
                  WHERE session_id = s.id AND unit_value IS NOT NULL) AS vt
               FROM farm_sessions s
               WHERE s.status = 'analyzed' AND s.zone_id IS NOT NULL) AS per_session
         GROUP BY zone_id",
        rusqlite::params![now],
    )?;
    let zones = conn.changes() as i64;

    let (sessions, linked, eff_num, eff_den): (i64, i64, f64, f64) = conn.query_row(
        "SELECT COUNT(*),
           SUM(CASE WHEN expected_gph IS NULL THEN 0 ELSE 1 END),
           COALESCE(SUM(CASE WHEN expected_gph IS NULL THEN 0 ELSE vt END), 0.0),
           COALESCE(SUM(CASE WHEN expected_gph IS NULL THEN 0
                      ELSE expected_gph * active_seconds / 3600.0 END), 0.0)
         FROM (SELECT s.expected_gph, s.active_seconds,
                 (SELECT COALESCE(SUM(qty * unit_value), 0) FROM farm_session_items
                  WHERE session_id = s.id AND unit_value IS NOT NULL) AS vt
               FROM farm_sessions s WHERE s.status = 'analyzed') AS per_session",
        rusqlite::params![],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let efficiency = if eff_den > 0.0 { eff_num / eff_den } else { 1.0 };
    for (key, value, samples) in
        [(METRIC_EFFICIENCY, efficiency, linked), (METRIC_SESSIONS, sessions as f64, sessions)]
    {
        conn.execute(
            "INSERT INTO personal_metrics (key, value, samples, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value,
               samples = excluded.samples, updated_at = excluded.updated_at",
            rusqlite::params![key, value, samples, now],
        )?;
    }
    Ok(RebuildReport { sessions, resources, zones })
}

// Profile ------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricEntry {
    pub key: String,
    pub value: f64,
    pub samples: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneEntry {
    pub zone_id: String,
    pub zone_name: Option<String>,
    pub sessions: i64,
    pub total_seconds: i64,
    pub avg_gph: f64,
    pub efficiency: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceEntry {
    pub resource_id: String,
    pub resource_name: Option<String>,
    pub gathers: i64,
    pub total_qty: f64,
    pub avg_yield: f64,
    pub avg_gold_per_gather: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalProfile {
    pub metrics: Vec<MetricEntry>,
    pub zones: Vec<ZoneEntry>,
    pub resources: Vec<ResourceEntry>,
}

pub fn get_profile(conn: &Connection) -> Result<PersonalProfile> {
    let mut stmt = conn.prepare(
        "SELECT key, value, samples, updated_at FROM personal_metrics ORDER BY key",
    )?;
    let metrics: Vec<MetricEntry> = stmt
        .query_map(rusqlite::params![], |r| {
            Ok(MetricEntry { key: r.get(0)?, value: r.get(1)?, samples: r.get(2)?, updated_at: r.get(3)? })
        })?
        .collect::<std::result::Result<_, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT p.zone_id, z.name, p.sessions, p.total_seconds, p.avg_gph,
                p.efficiency, p.updated_at
         FROM personal_zone_stats p LEFT JOIN zones z ON z.id = p.zone_id
         ORDER BY p.sessions DESC",
    )?;
    let zones: Vec<ZoneEntry> = stmt
        .query_map(rusqlite::params![], |r| {
            Ok(ZoneEntry {
                zone_id: r.get(0)?,
                zone_name: r.get(1)?,
                sessions: r.get(2)?,
                total_seconds: r.get(3)?,
                avg_gph: r.get(4)?,
                efficiency: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT p.resource_id, r.name, p.gathers, p.total_qty, p.avg_yield,
                p.avg_gold_per_gather, p.updated_at
         FROM personal_resource_stats p LEFT JOIN resources r ON r.id = p.resource_id
         ORDER BY p.gathers DESC",
    )?;
    let resources: Vec<ResourceEntry> = stmt
        .query_map(rusqlite::params![], |r| {
            Ok(ResourceEntry {
                resource_id: r.get(0)?,
                resource_name: r.get(1)?,
                gathers: r.get(2)?,
                total_qty: r.get(3)?,
                avg_yield: r.get(4)?,
                avg_gold_per_gather: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;
    Ok(PersonalProfile { metrics, zones, resources })
}

// Route personalization ---------------------------------------------------------------------

struct YieldStat {
    avg: f64,
    gathers: i64,
    updated_at: i64,
}

/// Blend each observation's expected yield toward the learned resource yield.
/// Returns (adjusted_count, explain lines).
pub fn personalize_observations(
    conn: &Connection,
    obs: &mut [RouteObservation],
    cfg: &BlendConfig,
    now: i64,
) -> Result<(usize, Vec<String>)> {
    let mut stmt = conn.prepare(
        "SELECT resource_id, avg_yield, gathers, updated_at FROM personal_resource_stats",
    )?;
    let rows: Vec<(String, f64, i64, i64)> = stmt
        .query_map(rusqlite::params![], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<std::result::Result<_, _>>()?;
    let stats: HashMap<String, YieldStat> = rows
        .into_iter()
        .map(|(id, avg, gathers, updated_at)| (id, YieldStat { avg, gathers, updated_at }))
        .collect();
    if stats.is_empty() {
        return Ok((0, vec!["no personal yield data yet — priors unchanged".to_string()]));
    }
    let mut adjusted = 0usize;
    let mut explained: HashMap<String, String> = HashMap::new();
    for ob in obs.iter_mut() {
        let Some(stat) = stats.get(&ob.resource_id) else { continue };
        let w = personal_weight(stat.gathers as f64, stat.updated_at, cfg, now);
        if w <= 0.0 {
            continue;
        }
        let before = ob.expected_yield;
        let blended = blend(before, Some(Evidence { value: stat.avg, weight: w }), None, cfg);
        if (blended.value - before).abs() > 1e-9 {
            adjusted += 1;
        }
        ob.expected_yield = blended.value;
        explained.entry(ob.resource_id.clone()).or_insert_with(|| {
            format!(
                "{}: yield {:.2} → {:.2} ({} gathers, w={:.2})",
                ob.resource_id, before, blended.value, stat.gathers, w
            )
        });
    }
    let mut lines: Vec<String> = explained.into_values().collect();
    lines.sort();
    if adjusted == 0 {
        lines.push("personal yields match priors — no change".to_string());
    }
    Ok((adjusted, lines))
}

/// Blended zone efficiency factor (1.0 neutral when unknown).
pub fn zone_efficiency(
    conn: &Connection,
    zone_id: &str,
    cfg: &BlendConfig,
    now: i64,
) -> Result<(f64, String)> {
    let row: Option<(f64, i64, i64)> = conn
        .query_row(
            "SELECT efficiency, sessions, updated_at FROM personal_zone_stats WHERE zone_id = ?1",
            rusqlite::params![zone_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match row {
        None => Ok((1.0, "no zone efficiency data — neutral 1.00".to_string())),
        Some((eff, sessions, updated_at)) => {
            let w = personal_weight(sessions as f64, updated_at, cfg, now);
            if w <= 0.0 {
                return Ok((1.0, "zone evidence fully decayed — neutral 1.00".to_string()));
            }
            let blended = blend(1.0, Some(Evidence { value: eff, weight: w }), None, cfg);
            Ok((
                blended.value,
                format!("zone efficiency {eff:.2} ({sessions} sessions, w={w:.2}) → ×{:.2}", blended.value),
            ))
        }
    }
}

fn zone_ypu(conn: &Connection, zone_id: &str) -> Result<f64> {
    let ypu: Option<f64> = conn
        .query_row("SELECT yards_per_unit FROM zones WHERE id = ?1", rusqlite::params![zone_id], |r| {
            r.get(0)
        })
        .optional()?;
    ypu.ok_or_else(|| AppError::route(format!("unknown zone `{zone_id}`")))
}

/// Full personalized pipeline: load → blend yields → optimize → scale values
/// by zone efficiency. Gross stays unadjusted (model reference); effective,
/// GPHs and bands scale.
#[allow(clippy::too_many_arguments)]
pub fn generate_personalized(
    conn: &Connection,
    zone_id: &str,
    resource_ids: &[String],
    config: &RouteConfig,
    prices: &HashMap<String, i64>,
    excluded: &[routes::ExcludedArea],
    blend_cfg: &BlendConfig,
    now: i64,
) -> Result<Vec<CandidateDto>> {
    let ypu = zone_ypu(conn, zone_id)?;
    let mut obs = routes::load_observations(conn, zone_id, resource_ids, excluded, prices)?;
    if obs.is_empty() {
        return Err(AppError::route(
            "no observations for this zone/resources (exclusions may cover everything)",
        ));
    }
    let total = obs.len();
    let (yields_blended, mut explain) = personalize_observations(conn, &mut obs, blend_cfg, now)?;
    let (efficiency, eff_line) = zone_efficiency(conn, zone_id, blend_cfg, now)?;
    explain.push(eff_line);
    let mut candidates = routes::generate_from_observations(&obs, ypu, config, prices)?;
    for cand in &mut candidates {
        cand.stats.effective_value *= efficiency;
        cand.stats.gph_expected *= efficiency;
        cand.stats.gph_conservative *= efficiency;
        cand.stats.band_hourly.p10 *= efficiency;
        cand.stats.band_hourly.median *= efficiency;
        cand.stats.band_hourly.p90 *= efficiency;
        cand.personal = Some(routes::PersonalAdjustment {
            efficiency,
            efficiency_zone: Some(zone_id.to_string()),
            yields_blended,
            observations: total,
            explain: explain.clone(),
        });
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::open_memory;

    const NOW: i64 = 1_700_100_000;
    const DAY: i64 = 86_400;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn mem_db() -> Connection {
        let mut conn = open_memory().unwrap();
        crate::migrations::migrate(&mut conn).unwrap();
        conn
    }

    fn seed_parents(conn: &Connection) {
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
            "INSERT INTO farm_categories (id, key, name) VALUES ('mining', 'mining', 'Mining')",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO resources (id, category_id, key, name, item_id) VALUES
             ('bismuth', 'mining', 'bismuth', 'Bismuth', 161502),
             ('mycobloom', 'mining', 'mycobloom', 'Mycobloom', 161504)",
            rusqlite::params![],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO zones (id, continent_id, name, map_id, yards_per_unit)
             VALUES ('zone-test', 'continent-test', 'Test Zone', 1, 10.0)",
            rusqlite::params![],
        )
        .unwrap();
    }

    /// s1 (linked, 1h): 10 bismuth @100 over 4 gathers; s2 (linked, 30m):
    /// 9 bismuth @100 over 3 gathers; s3 (unlinked, zoneless): 6 mycobloom @50.
    fn seed_sessions(conn: &Connection) {
        seed_parents(conn);
        for (id, zone, start, active, gph) in [
            ("s1", Some("zone-test"), NOW - 7 * DAY, 3600, Some(10_000)),
            ("s2", Some("zone-test"), NOW - DAY, 1800, Some(10_000)),
            ("s3", None, NOW - DAY, 1800, None),
        ] {
            conn.execute(
                "INSERT INTO farm_sessions (id, zone_id, status, started_at, active_seconds,
                 expected_gph, created_at, updated_at)
                 VALUES (?1, ?2, 'analyzed', ?3, ?4, ?5, ?3, ?3)",
                rusqlite::params![id, zone, start, active, gph],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO farm_session_items (session_id, item_id, resource_id, qty, unit_value, gathers)
             VALUES ('s1', 161502, 'bismuth', 10.0, 100, 4),
                    ('s2', 161502, 'bismuth', 9.0, 100, 3),
                    ('s3', 161504, 'mycobloom', 6.0, 50, 2)",
            rusqlite::params![],
        )
        .unwrap();
    }

    fn evidence(v: &serde_json::Value) -> Option<Evidence> {
        if v.is_null() {
            None
        } else {
            Some(Evidence { value: v["value"].as_f64().unwrap(), weight: v["weight"].as_f64().unwrap() })
        }
    }

    #[test]
    fn blend_matches_spec_fixture() {
        let path =
            format!("{}/../../fixtures/expected/blend.json", env!("CARGO_MANIFEST_DIR"));
        let fx: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let cases = fx["cases"].as_array().unwrap();
        assert!(!cases.is_empty());
        for case in cases {
            let id = case["id"].as_str().unwrap();
            match case["kind"].as_str().unwrap() {
                "decay" => {
                    let got = recency_decay(
                        case["ageSeconds"].as_i64().unwrap(),
                        case["halfLifeDays"].as_f64().unwrap(),
                    );
                    assert!(approx(got, case["expect"].as_f64().unwrap(), 1e-9), "{id}: {got}");
                }
                "weight" => {
                    let cfg = BlendConfig {
                        min_samples: case["minSamples"].as_f64().unwrap(),
                        half_life_days: case["halfLifeDays"].as_f64().unwrap(),
                        prior_weight: 1.0,
                    };
                    let got = personal_weight(
                        case["samples"].as_f64().unwrap(),
                        NOW - case["ageSeconds"].as_i64().unwrap(),
                        &cfg,
                        NOW,
                    );
                    assert!(approx(got, case["expect"].as_f64().unwrap(), 1e-9), "{id}: {got}");
                }
                "blend" => {
                    let cfg = BlendConfig {
                        prior_weight: case["priorWeight"].as_f64().unwrap(),
                        ..BlendConfig::default()
                    };
                    let got = blend(
                        case["prior"].as_f64().unwrap(),
                        evidence(&case["personal"]),
                        evidence(&case["crowd"]),
                        &cfg,
                    );
                    let ex = &case["expect"];
                    assert!(
                        approx(got.value, ex["value"].as_f64().unwrap(), 1e-9),
                        "{id}: {}",
                        got.value
                    );
                    assert!(
                        approx(got.personal_weight, ex["weight"].as_f64().unwrap(), 1e-9),
                        "{id}: {}",
                        got.personal_weight
                    );
                    assert_eq!(got.used_personal, ex["used"].as_bool().unwrap(), "{id}");
                }
                kind => panic!("unknown blend case kind: {kind}"),
            }
        }
    }

    #[test]
    fn blend_edges_are_honest() {
        let cfg = BlendConfig::default();
        assert_eq!(recency_decay(100, 0.0), 0.0);
        assert_eq!(recency_decay(-50, 21.0), 1.0);
        // Zero total weight falls back to the prior, never NaN.
        let flat = BlendConfig { prior_weight: 0.0, ..BlendConfig::default() };
        let got = blend(2.0, None, None, &flat);
        assert_eq!(got.value, 2.0);
        assert!(!got.used_personal);
        // Zero-weight evidence is ignored, not averaged in.
        let got = blend(2.0, Some(Evidence { value: 99.0, weight: 0.0 }), None, &cfg);
        assert_eq!(got.value, 2.0);
        assert!(!got.used_personal);
    }

    #[test]
    fn rebuild_learns_yields_gph_and_efficiency() {
        let conn = mem_db();
        seed_sessions(&conn);
        let report = rebuild_personal(&conn, NOW).unwrap();
        assert_eq!((report.sessions, report.resources, report.zones), (3, 2, 1));

        // Yields pool every analyzed session: 19 bismuth / 7 gathers.
        let profile = get_profile(&conn).unwrap();
        let bismuth = profile.resources.iter().find(|r| r.resource_id == "bismuth").unwrap();
        assert_eq!(bismuth.gathers, 7);
        assert!(approx(bismuth.total_qty, 19.0, 1e-9));
        assert!(approx(bismuth.avg_yield, 19.0 / 7.0, 1e-9), "{}", bismuth.avg_yield);
        assert!(approx(bismuth.avg_gold_per_gather, 1900.0 / 7.0, 1e-9));
        assert_eq!(bismuth.resource_name.as_deref(), Some("Bismuth"));
        let myco = profile.resources.iter().find(|r| r.resource_id == "mycobloom").unwrap();
        assert_eq!(myco.gathers, 2);
        assert!(approx(myco.avg_yield, 3.0, 1e-9));

        // Zone GPH pools linked + unlinked; efficiency only linked vs own expectations.
        let zone = profile.zones.iter().find(|z| z.zone_id == "zone-test").unwrap();
        assert_eq!((zone.sessions, zone.total_seconds), (2, 5400));
        assert!(approx(zone.avg_gph, 1900.0 * 3600.0 / 5400.0, 1e-9), "{}", zone.avg_gph);
        assert!(approx(zone.efficiency, 1900.0 / 15000.0, 1e-9), "{}", zone.efficiency);
        assert_eq!(zone.zone_name.as_deref(), Some("Test Zone"));

        let metric = |key: &str| {
            profile.metrics.iter().find(|m| m.key == key).unwrap().value
        };
        assert_eq!(metric(METRIC_SESSIONS), 3.0);
        assert!(approx(metric(METRIC_EFFICIENCY), 1900.0 / 15000.0, 1e-9));
    }

    #[test]
    fn rebuild_is_idempotent_and_skips_zero_gather_loot() {
        let conn = mem_db();
        seed_sessions(&conn);
        // Vendor trash with no gather count must not poison yields.
        conn.execute(
            "INSERT INTO farm_session_items (session_id, item_id, resource_id, qty, unit_value, gathers)
             VALUES ('s1', 999999, 'bismuth', 50.0, 1, 0)",
            rusqlite::params![],
        )
        .unwrap();
        rebuild_personal(&conn, NOW).unwrap();
        rebuild_personal(&conn, NOW).unwrap();
        let profile = get_profile(&conn).unwrap();
        assert_eq!(profile.resources.len(), 2);
        let bismuth = profile.resources.iter().find(|r| r.resource_id == "bismuth").unwrap();
        assert_eq!(bismuth.gathers, 7);
        assert!(approx(bismuth.avg_yield, 19.0 / 7.0, 1e-9));
    }

    #[test]
    fn zone_efficiency_blends_toward_learned() {
        let conn = mem_db();
        seed_sessions(&conn);
        let cfg = BlendConfig::default();
        // No rebuild yet → neutral, honest.
        let (eff, line) = zone_efficiency(&conn, "zone-test", &cfg, NOW).unwrap();
        assert_eq!(eff, 1.0);
        assert!(line.contains("neutral"), "{line}");
        rebuild_personal(&conn, NOW).unwrap();
        // 2 sessions → w=0.2; learned 1900/15000 blended with prior 1.0 (w=1).
        let learned = 1900.0 / 15000.0;
        let expect = (1.0 + learned * 0.2) / 1.2;
        let (eff, line) = zone_efficiency(&conn, "zone-test", &cfg, NOW).unwrap();
        assert!(approx(eff, expect, 1e-12), "{eff} vs {expect}");
        assert!(line.contains("×"), "{line}");
        // Unknown zone stays neutral.
        let (eff, _) = zone_efficiency(&conn, "zone-nope", &cfg, NOW).unwrap();
        assert_eq!(eff, 1.0);
    }

    #[test]
    fn personalize_blends_observation_yields() {
        let conn = mem_db();
        seed_sessions(&conn);
        rebuild_personal(&conn, NOW).unwrap();
        let cfg = BlendConfig::default();
        let mut obs = vec![
            RouteObservation {
                x: 1.0,
                y: 1.0,
                resource_id: "bismuth".to_string(),
                unit_value: 100,
                expected_yield: 2.0,
                spawn: 0.5,
                confidence: 0.4,
            },
            RouteObservation {
                x: 2.0,
                y: 2.0,
                resource_id: "unlearned".to_string(),
                unit_value: 100,
                expected_yield: 2.0,
                spawn: 0.5,
                confidence: 0.4,
            },
        ];
        let (adjusted, explain) = personalize_observations(&conn, &mut obs, &cfg, NOW).unwrap();
        assert_eq!(adjusted, 1);
        // 7 gathers → w=0.7; learned 19/7 blended with prior 2.0 (w=1).
        let expect = (2.0 + (19.0 / 7.0) * 0.7) / 1.7;
        assert!(approx(obs[0].expected_yield, expect, 1e-12), "{}", obs[0].expected_yield);
        assert_eq!(obs[1].expected_yield, 2.0);
        assert!(explain.iter().any(|l| l.contains("bismuth")), "{explain:?}");
    }

    #[test]
    fn personalized_pipeline_scales_effective_not_gross() {
        let conn = mem_db();
        seed_sessions(&conn);
        rebuild_personal(&conn, NOW).unwrap();
        for (x, y) in [
            (10.0, 10.0),
            (10.5, 10.0),
            (10.0, 10.5),
            (40.0, 40.0),
            (40.5, 40.0),
            (40.0, 40.5),
            (70.0, 20.0),
            (70.5, 20.0),
            (70.0, 20.5),
        ] {
            conn.execute(
                "INSERT INTO resource_observations
                 (zone_id, resource_id, x, y, yield_qty, source, observed_at)
                 VALUES ('zone-test', 'bismuth', ?1, ?2, 2.0, 'seed', 0)",
                rusqlite::params![x, y],
            )
            .unwrap();
        }
        let mut prices = HashMap::new();
        prices.insert("bismuth".to_string(), 100);
        let res = vec!["bismuth".to_string()];
        let cands = generate_personalized(
            &conn,
            "zone-test",
            &res,
            &RouteConfig::default(),
            &prices,
            &[],
            &BlendConfig::default(),
            NOW,
        )
        .unwrap();
        assert!(!cands.is_empty());
        let learned = 1900.0 / 15000.0;
        let expect_eff = (1.0 + learned * 0.2) / 1.2;
        for cand in &cands {
            let personal = cand.personal.as_ref().expect("personal block");
            assert!(approx(personal.efficiency, expect_eff, 1e-12));
            assert_eq!(personal.efficiency_zone.as_deref(), Some("zone-test"));
            assert_eq!(personal.observations, 9);
            assert!(personal.yields_blended > 0);
            assert!(cand.stats.gross_value > 0.0);
            assert!(cand.stats.effective_value < cand.stats.gross_value);
        }
        // Unknown zone fails before touching the optimizer.
        let err = generate_personalized(
            &conn,
            "zone-nope",
            &res,
            &RouteConfig::default(),
            &prices,
            &[],
            &BlendConfig::default(),
            NOW,
        )
        .unwrap_err();
        assert!(err.user_message().contains("zone-nope"), "{}", err.user_message());
    }
}

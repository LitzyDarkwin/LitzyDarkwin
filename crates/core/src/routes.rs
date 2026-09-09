//! Economic route optimization + simulation.
//!
//! Pipeline (ALGORITHMS.md §§2–4,6): eligible observations → grid clustering →
//! value estimation → prune → candidate tours (top-K starts + 2-opt) →
//! analytic simulation (P10/median/P90) → rank by expected GPH.
//!
//! Deterministic by construction: ordered maps, sorted processing, and
//! seed-mixed FNV tie-breaks. No RNG anywhere in this module.

use crate::error::{AppError, Result};
use crate::models::Band;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};

// Geo -------------------------------------------------------------------------

/// Euclidean distance in map units (0–100 space).
pub fn map_distance(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

// Config -----------------------------------------------------------------------

/// All optimizer weights live here — no mystery constants. Stored per route.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteConfig {
    pub seed: u64,
    pub cell_size: f64,
    pub max_diameter: f64,
    pub min_samples: usize,
    pub max_clusters: usize,
    pub candidate_starts: usize,
    pub two_opt_passes: usize,
    pub travel_speed_yps: f64,
    pub gather_seconds: f64,
    pub loop_overhead_seconds: f64,
}

impl Default for RouteConfig {
    fn default() -> Self {
        RouteConfig {
            seed: 20260909,
            cell_size: 4.0,
            max_diameter: 14.0,
            min_samples: 2,
            max_clusters: 24,
            candidate_starts: 3,
            two_opt_passes: 8,
            travel_speed_yps: 7.0,
            gather_seconds: 3.5,
            loop_overhead_seconds: 15.0,
        }
    }
}

/// Partial config from the UI: `None` keeps the default. Clamped on build.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteConfigDto {
    pub seed: Option<u64>,
    pub cell_size: Option<f64>,
    pub max_diameter: Option<f64>,
    pub min_samples: Option<usize>,
    pub max_clusters: Option<usize>,
    pub candidate_starts: Option<usize>,
    pub two_opt_passes: Option<usize>,
    pub travel_speed_yps: Option<f64>,
    pub gather_seconds: Option<f64>,
    pub loop_overhead_seconds: Option<f64>,
}

impl RouteConfigDto {
    pub fn into_config(self) -> RouteConfig {
        let d = RouteConfig::default();
        RouteConfig {
            seed: self.seed.unwrap_or(d.seed),
            cell_size: self.cell_size.unwrap_or(d.cell_size).clamp(1.0, 20.0),
            max_diameter: self.max_diameter.unwrap_or(d.max_diameter).clamp(4.0, 60.0),
            min_samples: self.min_samples.unwrap_or(d.min_samples).clamp(1, 50),
            max_clusters: self.max_clusters.unwrap_or(d.max_clusters).clamp(2, 64),
            candidate_starts: self.candidate_starts.unwrap_or(d.candidate_starts).clamp(1, 8),
            two_opt_passes: self.two_opt_passes.unwrap_or(d.two_opt_passes).clamp(0, 50),
            travel_speed_yps: self.travel_speed_yps.unwrap_or(d.travel_speed_yps).clamp(1.0, 30.0),
            gather_seconds: self.gather_seconds.unwrap_or(d.gather_seconds).clamp(0.5, 30.0),
            loop_overhead_seconds: self
                .loop_overhead_seconds
                .unwrap_or(d.loop_overhead_seconds)
                .clamp(0.0, 300.0),
        }
    }
}

// Value model -------------------------------------------------------------------

/// One eligible observation with its economic parameters resolved.
#[derive(Debug, Clone)]
pub struct RouteObservation {
    pub x: f64,
    pub y: f64,
    pub resource_id: String,
    /// Effective market value per unit, integer copper.
    pub unit_value: i64,
    pub expected_yield: f64,
    pub spawn: f64,
    pub confidence: f64,
}

/// Source priors (Phase 4 defaults; personal/crowd blending arrives in P7–P8).
/// (spawn probability, confidence)
pub fn source_prior(source: &str) -> (f64, f64) {
    match source {
        "addon" => (0.75, 0.7),
        "import" => (0.6, 0.5),
        "crowd" => (0.55, 0.45),
        _ => (0.5, 0.4), // "seed" and anything unknown: weak prior
    }
}

/// `(gross, effective)` node value in copper.
/// gross = spawn × yield × unit; effective = gross × (0.5 + 0.5×confidence).
pub fn expected_node_value(obs: &RouteObservation) -> (f64, f64) {
    let gross = obs.spawn * obs.expected_yield.max(0.0) * obs.unit_value.max(0) as f64;
    let modifier = 0.5 + 0.5 * obs.confidence.clamp(0.0, 1.0);
    (gross, gross * modifier)
}

// Clustering --------------------------------------------------------------------

/// Cap on points per clustering run (density grid upstream keeps this bounded).
pub const MAX_CLUSTER_POINTS: usize = 2000;
/// Skip cross-cell merges whose union would exceed this (diameter would too).
const MAX_MERGE_MEMBERS: usize = 800;

#[derive(Debug, Clone)]
pub struct NodeCluster {
    pub id: String,
    pub cx: f64,
    pub cy: f64,
    pub members: usize,
    pub qty: f64,
    pub value_gross: f64,
    pub value_effective: f64,
    pub member_values: Vec<f64>,
    pub resources: BTreeMap<String, f64>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Default)]
pub struct ClusterOutput {
    pub clusters: Vec<NodeCluster>,
    /// Points dropped by `min_samples` pruning.
    pub dropped: usize,
    /// Points skipped over MAX_CLUSTER_POINTS (input order, deterministic).
    pub truncated: usize,
}

struct UnionFind {
    parent: Vec<usize>,
    members: Vec<Vec<usize>>,
    diameter: Vec<f64>,
}

impl UnionFind {
    fn new(n: usize, cells: &[Vec<usize>], xs: &[f64], ys: &[f64]) -> Self {
        let mut uf = UnionFind {
            parent: (0..n).collect(),
            members: cells.to_vec(),
            diameter: vec![0.0; n],
        };
        for i in 0..n {
            uf.diameter[i] = diameter_of(&uf.members[i], xs, ys);
        }
        uf
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    /// Union roots if the merged diameter allows. Returns the new root or None.
    fn try_union(
        &mut self,
        a: usize,
        b: usize,
        max_diameter: f64,
        xs: &[f64],
        ys: &[f64],
    ) -> Option<usize> {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return Some(ra);
        }
        if self.members[ra].len() + self.members[rb].len() > MAX_MERGE_MEMBERS {
            return None;
        }
        // Cross diameter only (internal diameters already known).
        let mut cross: f64 = 0.0;
        for &i in &self.members[ra] {
            for &j in &self.members[rb] {
                cross = cross.max(map_distance(xs[i], ys[i], xs[j], ys[j]));
                if cross > max_diameter {
                    return None;
                }
            }
        }
        let dia = self.diameter[ra].max(self.diameter[rb]).max(cross);
        if dia > max_diameter {
            return None;
        }
        // Union by size (deterministic: smaller index wins ties).
        let (big, small) = match self.members[ra].len().cmp(&self.members[rb].len()) {
            std::cmp::Ordering::Greater => (ra, rb),
            std::cmp::Ordering::Less => (rb, ra),
            std::cmp::Ordering::Equal => {
                if ra < rb {
                    (ra, rb)
                } else {
                    (rb, ra)
                }
            }
        };
        let moved = std::mem::take(&mut self.members[small]);
        self.members[big].extend(moved);
        self.diameter[big] = dia;
        self.parent[small] = big;
        Some(big)
    }
}

fn diameter_of(members: &[usize], xs: &[f64], ys: &[f64]) -> f64 {
    let mut d: f64 = 0.0;
    for (a, &i) in members.iter().enumerate() {
        for &j in &members[a + 1..] {
            d = d.max(map_distance(xs[i], ys[i], xs[j], ys[j]));
        }
    }
    d
}

/// FNV-1a 64 with seed mixing, for deterministic tie-breaks.
fn fnv_seed(s: &str, seed: u64) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ seed;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn cluster_observations(
    obs: &[RouteObservation],
    cfg: &RouteConfig,
) -> ClusterOutput {
    let mut out = ClusterOutput::default();
    if obs.is_empty() {
        return out;
    }
    let take = obs.len().min(MAX_CLUSTER_POINTS);
    out.truncated = obs.len() - take;
    let obs = &obs[..take];
    let xs: Vec<f64> = obs.iter().map(|o| o.x).collect();
    let ys: Vec<f64> = obs.iter().map(|o| o.y).collect();

    // 1. Grid cells (BTreeMap = sorted order).
    let mut cells: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
    for (i, o) in obs.iter().enumerate() {
        let gx = (o.x / cfg.cell_size).floor() as i64;
        let gy = (o.y / cfg.cell_size).floor() as i64;
        cells.entry((gx, gy)).or_default().push(i);
    }
    let keys: Vec<(i64, i64)> = cells.keys().copied().collect();
    let index_of: HashMap<(i64, i64), usize> =
        keys.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let cell_vecs: Vec<Vec<usize>> = keys.iter().map(|k| cells[k].clone()).collect();

    // 2. Merge 8-neighborhood cells while the diameter allows (sorted pairs).
    let mut uf = UnionFind::new(keys.len(), &cell_vecs, &xs, &ys);
    const OFFSETS: [(i64, i64); 8] =
        [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];
    for (i, key) in keys.iter().enumerate() {
        for (dx, dy) in OFFSETS {
            let nb = (key.0 + dx, key.1 + dy);
            if let Some(&j) = index_of.get(&nb) {
                if j > i {
                    uf.try_union(i, j, cfg.max_diameter, &xs, &ys);
                }
            }
        }
    }

    // 3. Collect member lists from union-find roots (non-root buckets were
    // drained by moves during union, so roots hold the full sets).
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in 0..keys.len() {
        if uf.find(i) == i && !uf.members[i].is_empty() {
            groups.push(std::mem::take(&mut uf.members[i]));
        }
    }
    for g in &mut groups {
        g.sort_unstable();
    }
    // Rank groups by value desc, then first member (deterministic ids).
    let mut scored: Vec<(f64, Vec<usize>)> = Vec::new();
    for g in groups {
        if g.len() < cfg.min_samples {
            out.dropped += g.len();
            continue;
        }
        let value: f64 = g.iter().map(|&i| expected_node_value(&obs[i]).1).sum();
        scored.push((value, g));
    }
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1[0].cmp(&b.1[0]))
    });

    for (n, (value_effective, members)) in scored.into_iter().enumerate() {
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut gross = 0.0;
        let mut qty = 0.0;
        let mut conf = 0.0;
        let mut member_values = Vec::with_capacity(members.len());
        let mut resources: BTreeMap<String, f64> = BTreeMap::new();
        for &i in &members {
            let o = &obs[i];
            let (g, e) = expected_node_value(o);
            sx += o.x;
            sy += o.y;
            gross += g;
            qty += o.spawn * o.expected_yield.max(0.0);
            conf += o.confidence.clamp(0.0, 1.0);
            member_values.push(e);
            *resources.entry(o.resource_id.clone()).or_insert(0.0) += e;
        }
        let count = members.len();
        out.clusters.push(NodeCluster {
            id: format!("c{n}"),
            cx: sx / count as f64,
            cy: sy / count as f64,
            members: count,
            qty,
            value_gross: gross,
            value_effective,
            member_values,
            resources,
            confidence: conf / count as f64,
        });
    }
    out
}

// Tours --------------------------------------------------------------------------

/// Closed-loop length in yards visiting `order` in sequence.
pub fn tour_length_yards(order: &[usize], clusters: &[NodeCluster], ypu: f64) -> f64 {
    if order.len() < 2 {
        return 0.0;
    }
    let mut total = 0.0;
    for w in 0..order.len() {
        let a = &clusters[order[w]];
        let b = &clusters[order[(w + 1) % order.len()]];
        total += map_distance(a.cx, a.cy, b.cx, b.cy) * ypu;
    }
    total
}

/// Greedy nearest-neighbor tour from `start`. Ties: (distance, seed-mixed id hash).
pub fn nearest_neighbor(
    clusters: &[NodeCluster],
    start: usize,
    ypu: f64,
    seed: u64,
) -> Vec<usize> {
    let n = clusters.len();
    if n == 0 {
        return Vec::new();
    }
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut current = start.min(n - 1);
    order.push(current);
    visited[current] = true;
    while order.len() < n {
        let a = &clusters[current];
        let mut best: Option<(usize, f64, u64)> = None;
        for (i, c) in clusters.iter().enumerate() {
            if visited[i] {
                continue;
            }
            let d = map_distance(a.cx, a.cy, c.cx, c.cy) * ypu;
            let tie = fnv_seed(&c.id, seed);
            let better = match best {
                None => true,
                Some((_, bd, bt)) => d < bd || (d == bd && tie < bt),
            };
            if better {
                best = Some((i, d, tie));
            }
        }
        let Some((next, _, _)) = best else {
            break; // unreachable while order.len() < n, but never panic
        };
        visited[next] = true;
        order.push(next);
        current = next;
    }
    order
}

/// First-improvement 2-opt, deterministic scan order, bounded passes.
pub fn two_opt(
    order: &[usize],
    clusters: &[NodeCluster],
    ypu: f64,
    max_passes: usize,
) -> Vec<usize> {
    let n = order.len();
    if n < 4 {
        return order.to_vec();
    }
    let mut best = order.to_vec();
    let mut best_len = tour_length_yards(&best, clusters, ypu);
    for _ in 0..max_passes {
        let mut improved = false;
        'outer: for i in 0..n - 1 {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue; // same loop reversed
                }
                let mut candidate = best.clone();
                candidate[i + 1..=j].reverse();
                let len = tour_length_yards(&candidate, clusters, ypu);
                if len < best_len - 1e-9 {
                    best = candidate;
                    best_len = len;
                    improved = true;
                    break 'outer;
                }
            }
        }
        if !improved {
            break;
        }
    }
    best
}

// Candidates + simulation ------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RouteCandidate {
    pub order: Vec<usize>,
    pub loop_yards: f64,
    pub loop_seconds: f64,
    pub nodes: usize,
    pub qty: f64,
    pub gross: f64,
    pub effective: f64,
    pub gph_expected: f64,
    pub gph_conservative: f64,
    pub band_hourly: Band,
}

#[derive(Debug, Clone, Default)]
pub struct CandidateOutput {
    pub candidates: Vec<RouteCandidate>,
    pub pruned: usize,
}

/// Per-cluster spread from member-value dispersion (CV clamped 0.15–0.60);
/// tiny clusters fall back to a confidence-adjusted default.
fn cluster_spread(c: &NodeCluster) -> f64 {
    if c.member_values.len() >= 3 && c.value_effective > 0.0 {
        let mean = c.value_effective / c.member_values.len() as f64;
        let var: f64 = c.member_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / c.member_values.len() as f64;
        (var.sqrt() / mean).clamp(0.15, 0.60)
    } else {
        (0.35 + 0.25 * (1.0 - c.confidence.clamp(0.0, 1.0))).clamp(0.15, 0.60)
    }
}

pub fn simulate(order: &[usize], clusters: &[NodeCluster], cfg: &RouteConfig, ypu: f64) -> RouteCandidate {
    let loop_yards = tour_length_yards(order, clusters, ypu);
    let mut nodes = 0usize;
    let mut qty = 0.0;
    let mut gross = 0.0;
    let mut effective = 0.0;
    let mut variance = 0.0;
    for &i in order {
        let c = &clusters[i];
        nodes += c.members;
        qty += c.qty;
        gross += c.value_gross;
        effective += c.value_effective;
        let spread = cluster_spread(c);
        variance += (c.value_effective * spread).powi(2);
    }
    let loop_seconds = loop_yards / cfg.travel_speed_yps.max(0.1)
        + nodes as f64 * cfg.gather_seconds
        + cfg.loop_overhead_seconds;
    let hours = (loop_seconds / 3600.0).max(1e-9);
    let std = variance.sqrt();
    let p10 = (effective - 1.2815515655446004 * std).max(0.0);
    let p90 = effective + 1.2815515655446004 * std;
    RouteCandidate {
        order: order.to_vec(),
        loop_yards,
        loop_seconds,
        nodes,
        qty,
        gross,
        effective,
        gph_expected: effective / hours,
        gph_conservative: p10 / hours,
        band_hourly: Band { p10: p10 / hours, median: effective / hours, p90: p90 / hours },
    }
}

/// Top-K seeded starts → NN → 2-opt → simulate → rank by expected GPH.
pub fn build_candidates(
    clusters: &[NodeCluster],
    cfg: &RouteConfig,
    ypu: f64,
) -> Result<CandidateOutput> {
    if clusters.len() < 2 {
        return Err(AppError::route("need at least 2 node clusters — widen resources or zone"));
    }
    // Prune to the most valuable clusters (already value-ordered by id rank).
    let mut ranked: Vec<usize> = (0..clusters.len()).collect();
    ranked.sort_by(|&a, &b| {
        clusters[b]
            .value_effective
            .partial_cmp(&clusters[a].value_effective)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(clusters[a].id.cmp(&clusters[b].id))
    });
    let keep = cfg.max_clusters.min(clusters.len());
    let pruned = clusters.len() - keep;
    ranked.truncate(keep);
    // Work on the pruned slice; map indices back for output order.
    let pool: Vec<NodeCluster> = ranked.iter().map(|&i| clusters[i].clone()).collect();
    let starts = cfg.candidate_starts.min(pool.len());
    let mut candidates = Vec::with_capacity(starts);
    for s in 0..starts {
        let nn = nearest_neighbor(&pool, s, ypu, cfg.seed);
        let improved = two_opt(&nn, &pool, ypu, cfg.two_opt_passes);
        let mut sim = simulate(&improved, &pool, cfg, ypu);
        // Map pool indices back to global cluster indices.
        sim.order = improved.into_iter().map(|i| ranked[i]).collect();
        candidates.push(sim);
    }
    candidates.sort_by(|a, b| {
        b.gph_expected
            .partial_cmp(&a.gph_expected)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(CandidateOutput { candidates, pruned })
}

// DB integration ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcludedArea {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Waypoint {
    pub x: f64,
    pub y: f64,
    pub cluster_id: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStats {
    pub loop_yards: f64,
    pub loop_seconds: f64,
    pub nodes: usize,
    pub qty: f64,
    pub gross_value: f64,
    pub effective_value: f64,
    pub gph_expected: f64,
    pub gph_conservative: f64,
    pub band_hourly: Band,
    pub resolved_prices: HashMap<String, i64>,
    pub pruned_clusters: usize,
    pub dropped_points: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDto {
    pub rank: usize,
    pub waypoints: Vec<Waypoint>,
    pub stats: RouteStats,
    /// Effective config (post-defaults, post-clamp) so saves reproduce exactly.
    pub config: RouteConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRoute {
    pub id: String,
    pub name: String,
    pub zone_id: Option<String>,
    pub resource_ids: Vec<String>,
    pub waypoints: Vec<Waypoint>,
    pub excluded: Vec<ExcludedArea>,
    pub config: RouteConfig,
    pub stats: RouteStats,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteSummary {
    pub id: String,
    pub name: String,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub gph_expected: f64,
    pub waypoint_count: usize,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRouteInput {
    pub name: String,
    pub zone_id: Option<String>,
    pub resource_ids: Vec<String>,
    pub waypoints: Vec<Waypoint>,
    pub excluded: Vec<ExcludedArea>,
    pub config: RouteConfig,
    pub stats: RouteStats,
}

/// Map id for GO1 export (`None` when the zone row carries none).
pub fn zone_map_id(conn: &Connection, zone_id: &str) -> Result<Option<i64>> {
    Ok(zone_ypu(conn, zone_id)?.1)
}

fn zone_ypu(conn: &Connection, zone_id: &str) -> Result<(f64, Option<i64>)> {
    let row: Option<(f64, Option<i64>)> = conn
        .query_row(
            "SELECT yards_per_unit, map_id FROM zones WHERE id = ?1",
            rusqlite::params![zone_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.ok_or_else(|| AppError::route(format!("unknown zone `{zone_id}`")))
}

/// Load eligible observations with prices resolved. Missing prices are a hard
/// error naming the resources (routes must never silently value at zero).
pub fn load_observations(
    conn: &Connection,
    zone_id: &str,
    resource_ids: &[String],
    excluded: &[ExcludedArea],
    prices: &HashMap<String, i64>,
) -> Result<Vec<RouteObservation>> {
    if resource_ids.is_empty() {
        return Err(AppError::route("select at least one resource"));
    }
    let missing: Vec<&str> = resource_ids
        .iter()
        .filter(|id| !prices.contains_key(id.as_str()))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        return Err(AppError::route(format!(
            "missing prices for: {} (refresh market data or enter manual prices)",
            missing.join(", ")
        )));
    }
    let placeholders = vec!["?"; resource_ids.len()].join(", ");
    let sql = format!(
        "SELECT x, y, resource_id, yield_qty, source FROM resource_observations
         WHERE zone_id = ?1 AND resource_id IN ({placeholders})"
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(resource_ids.len() + 1);
    params.push(&zone_id);
    for id in resource_ids {
        params.push(id);
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<(f64, f64, String, Option<f64>, String)> = stmt
        .query_map(params.as_slice(), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<std::result::Result<_, _>>()?;

    let mut out = Vec::with_capacity(rows.len());
    for (x, y, resource_id, yield_qty, source) in rows {
        if excluded.iter().any(|e| map_distance(x, y, e.x, e.y) < e.radius.max(0.0)) {
            continue;
        }
        let (spawn, confidence) = source_prior(&source);
        out.push(RouteObservation {
            x,
            y,
            unit_value: prices.get(resource_id.as_str()).copied().unwrap_or(0),
            resource_id,
            expected_yield: yield_qty.unwrap_or(2.0).max(0.0),
            spawn,
            confidence,
        });
    }
    Ok(out)
}

/// Full pipeline: load → cluster → candidates → simulate → top-3 DTOs.
pub fn generate(
    conn: &Connection,
    zone_id: &str,
    resource_ids: &[String],
    config: &RouteConfig,
    prices: &HashMap<String, i64>,
    excluded: &[ExcludedArea],
) -> Result<Vec<CandidateDto>> {
    let (ypu, _) = zone_ypu(conn, zone_id)?;
    let obs = load_observations(conn, zone_id, resource_ids, excluded, prices)?;
    if obs.is_empty() {
        return Err(AppError::route("no observations for this zone/resources (exclusions may cover everything)"));
    }
    let clustered = cluster_observations(&obs, config);
    let built = build_candidates(&clustered.clusters, config, ypu)?;
    let mut dtos = Vec::new();
    for (rank, cand) in built.candidates.into_iter().take(3).enumerate() {
        let mut warnings = Vec::new();
        if built.pruned > 0 {
            warnings.push(format!("pruned {} low-value cluster(s)", built.pruned));
        }
        if clustered.dropped > 0 {
            warnings.push(format!("dropped {} point(s) below min samples", clustered.dropped));
        }
        if clustered.truncated > 0 {
            warnings.push(format!("truncated {} point(s) over cap", clustered.truncated));
        }
        let waypoints: Vec<Waypoint> = cand
            .order
            .iter()
            .map(|&i| {
                let c = &clustered.clusters[i];
                Waypoint { x: c.cx, y: c.cy, cluster_id: c.id.clone(), value: c.value_effective }
            })
            .collect();
        dtos.push(CandidateDto {
            rank: rank + 1,
            waypoints,
            config: config.clone(),
            stats: RouteStats {
                loop_yards: cand.loop_yards,
                loop_seconds: cand.loop_seconds,
                nodes: cand.nodes,
                qty: cand.qty,
                gross_value: cand.gross,
                effective_value: cand.effective,
                gph_expected: cand.gph_expected,
                gph_conservative: cand.gph_conservative,
                band_hourly: cand.band_hourly,
                resolved_prices: prices.clone(),
                pruned_clusters: built.pruned,
                dropped_points: clustered.dropped,
                warnings,
            },
        });
    }
    Ok(dtos)
}

static ROUTE_COUNTER: AtomicU64 = AtomicU64::new(1);

fn new_route_id(now: i64, name: &str) -> String {
    let n = ROUTE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut h: u64 = 0xcbf29ce484222325;
    for b in now.to_le_bytes().into_iter().chain(n.to_le_bytes()).chain(name.bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("rt-{now}-{h:016x}")
}

pub fn save_route(conn: &Connection, input: &SaveRouteInput, now: i64) -> Result<SavedRoute> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::route("route name is empty"));
    }
    if name.len() > 80 {
        return Err(AppError::route("route name is too long (max 80)"));
    }
    if input.waypoints.len() < 2 || input.waypoints.len() > 64 {
        return Err(AppError::route("routes need 2–64 waypoints"));
    }
    let id = new_route_id(now, name);
    conn.execute(
        "INSERT INTO saved_routes (id, name, zone_id, resource_ids_json, waypoints_json,
         excluded_json, config_json, stats_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        rusqlite::params![
            id,
            name,
            input.zone_id,
            serde_json::to_string(&input.resource_ids)?,
            serde_json::to_string(&input.waypoints)?,
            serde_json::to_string(&input.excluded)?,
            serde_json::to_string(&input.config)?,
            serde_json::to_string(&input.stats)?,
            now,
        ],
    )?;
    get_route(conn, &id)
}

fn row_to_saved(
    id: String,
    name: String,
    zone_id: Option<String>,
    resource_ids_json: String,
    waypoints_json: String,
    excluded_json: String,
    config_json: String,
    stats_json: String,
    created_at: i64,
    updated_at: i64,
) -> Result<SavedRoute> {
    let corrupt = |what: &str| AppError::provider(format!("saved route `{id}` has corrupt {what}"));
    Ok(SavedRoute {
        id: id.clone(),
        name,
        zone_id,
        resource_ids: serde_json::from_str(&resource_ids_json).map_err(|_| corrupt("resources"))?,
        waypoints: serde_json::from_str(&waypoints_json).map_err(|_| corrupt("waypoints"))?,
        excluded: serde_json::from_str(&excluded_json).map_err(|_| corrupt("exclusions"))?,
        config: serde_json::from_str(&config_json).map_err(|_| corrupt("config"))?,
        stats: serde_json::from_str(&stats_json).map_err(|_| corrupt("stats"))?,
        created_at,
        updated_at,
    })
}

pub fn get_route(conn: &Connection, id: &str) -> Result<SavedRoute> {
    let row: Option<(String, Option<String>, String, String, String, String, String, i64, i64)> =
        conn
            .query_row(
                "SELECT name, zone_id, resource_ids_json, waypoints_json, excluded_json,
                        config_json, stats_json, created_at, updated_at
                 FROM saved_routes WHERE id = ?1",
                rusqlite::params![id],
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
                        r.get(8)?,
                    ))
                },
            )
            .optional()?;
    match row {
        None => Err(AppError::route(format!("unknown route `{id}`"))),
        Some((name, zone_id, res, wps, exc, cfg, stats, created, updated)) => {
            row_to_saved(id.to_string(), name, zone_id, res, wps, exc, cfg, stats, created, updated)
        }
    }
}

pub fn list_routes(conn: &Connection, zone_id: Option<&str>) -> Result<Vec<RouteSummary>> {
    let mut sql = String::from(
        "SELECT r.id, r.name, r.zone_id, z.name, r.stats_json, r.waypoints_json, r.updated_at
         FROM saved_routes r LEFT JOIN zones z ON z.id = r.zone_id",
    );
    if zone_id.is_some() {
        sql.push_str(" WHERE r.zone_id = ?1");
    }
    sql.push_str(" ORDER BY r.updated_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let map_row = |r: &rusqlite::Row| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, i64>(6)?,
        ))
    };
    let rows: Vec<(String, String, Option<String>, Option<String>, String, String, i64)> =
        if let Some(zone) = zone_id {
            stmt.query_map(rusqlite::params![zone], map_row)?
                .collect::<std::result::Result<_, _>>()?
        } else {
            stmt.query_map(rusqlite::params![], map_row)?
                .collect::<std::result::Result<_, _>>()?
        };
    let mut out = Vec::with_capacity(rows.len());
    for (id, name, zone, zone_name, stats_json, waypoints_json, updated_at) in rows {
        let corrupt = |what| AppError::provider(format!("saved route `{id}` has corrupt {what}"));
        let stats: RouteStats =
            serde_json::from_str(&stats_json).map_err(|_| corrupt("stats"))?;
        let waypoints: Vec<Waypoint> =
            serde_json::from_str(&waypoints_json).map_err(|_| corrupt("waypoints"))?;
        out.push(RouteSummary {
            id,
            name,
            zone_id: zone,
            zone_name,
            gph_expected: stats.gph_expected,
            waypoint_count: waypoints.len(),
            updated_at,
        });
    }
    Ok(out)
}

pub fn delete_route(conn: &Connection, id: &str) -> Result<bool> {
    let n = conn.execute("DELETE FROM saved_routes WHERE id = ?1", rusqlite::params![id])?;
    Ok(n == 1)
}

/// Load the requested routes in order for side-by-side comparison.
pub fn compare_routes(conn: &Connection, ids: &[String]) -> Result<Vec<SavedRoute>> {
    if ids.is_empty() || ids.len() > 4 {
        return Err(AppError::route("compare 1–4 routes"));
    }
    ids.iter().map(|id| get_route(conn, id)).collect()
}

// GO1 import codec (Rust mirror of addon/Bridge.lua) --------------------------------
// GO1.<mapID>.<n>.<coords>.<check> — coords XXXYYY triplets, mod-997 checksum.

pub const GO1_PREFIX: &str = "GO1";
pub const GO1_MAX_WAYPOINTS: usize = 40;

fn go1_checksum(map_id: i64, n: usize, coords: &str) -> u32 {
    let s = format!("{map_id}.{n}.{coords}");
    let mut acc: u32 = 0;
    for (i, b) in s.bytes().enumerate() {
        if b.is_ascii_digit() {
            acc = (acc + u32::from(b - b'0') * (i as u32 + 1)) % 997;
        }
    }
    acc
}

/// Encode waypoints to a GO1 string (errors past 40 points — split the route).
pub fn go1_encode(map_id: i64, waypoints: &[(f64, f64)]) -> Result<String> {
    if waypoints.len() < 2 {
        return Err(AppError::route("need at least 2 waypoints to export"));
    }
    if waypoints.len() > GO1_MAX_WAYPOINTS {
        return Err(AppError::route(format!(
            "route has {} waypoints; the in-game import supports {} — split the route",
            waypoints.len(),
            GO1_MAX_WAYPOINTS
        )));
    }
    let mut coords = String::with_capacity(waypoints.len() * 6);
    for (x, y) in waypoints {
        let ix = ((x * 10.0).round() as i64).clamp(0, 999);
        let iy = ((y * 10.0).round() as i64).clamp(0, 999);
        coords.push_str(&format!("{ix:03}{iy:03}"));
    }
    let check = go1_checksum(map_id, waypoints.len(), &coords);
    Ok(format!("{}.{}.{}.{:03}", GO1_PREFIX, map_id, waypoints.len(), check))
}

/// Decode a GO1 string back to (map_id, waypoints). Strict shapes, mirroring
/// the Lua decoder: five dot-separated parts, digit-only fields, length and
/// checksum verified before any waypoint is trusted.
pub fn go1_decode(code: &str) -> Result<(i64, Vec<(f64, f64)>)> {
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let parts: Vec<&str> = code.split('.').collect();
    if parts.len() != 5 {
        return Err(AppError::route("unrecognized format (want GO1.<map>.<n>.<coords>.<check>)"));
    }
    if parts[0] != GO1_PREFIX {
        return Err(AppError::route(format!("unsupported version: {}", parts[0])));
    }
    let digits_only =
        |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits_only(parts[1]) {
        return Err(AppError::route("bad map id in import code"));
    }
    if !digits_only(parts[2]) {
        return Err(AppError::route("bad waypoint count in import code"));
    }
    let map_id: i64 =
        parts[1].parse().map_err(|_| AppError::route("bad map id in import code"))?;
    let n: usize =
        parts[2].parse().map_err(|_| AppError::route("bad waypoint count in import code"))?;
    if !(2..=64).contains(&n) {
        return Err(AppError::route("waypoint count out of range (2-64)"));
    }
    let (coords, check) = (parts[3], parts[4]);
    if coords.len() != n * 6 || !coords.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::route("length mismatch in import code"));
    }
    if check.len() != 3 || !check.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::route("bad checksum in import code"));
    }
    let want = go1_checksum(map_id, n, coords);
    let got: u32 = check.parse().unwrap_or(u32::MAX);
    if got != want {
        return Err(AppError::route("checksum mismatch — code was mistyped or truncated"));
    }
    let mut waypoints = Vec::with_capacity(n);
    for i in 0..n {
        let chunk = &coords[i * 6..i * 6 + 6];
        let ix: f64 =
            chunk[0..3].parse().map_err(|_| AppError::route("bad coordinate in import code"))?;
        let iy: f64 =
            chunk[3..6].parse().map_err(|_| AppError::route("bad coordinate in import code"))?;
        waypoints.push((ix / 10.0, iy / 10.0));
    }
    Ok((map_id, waypoints))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::open_memory;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn fixture(name: &str) -> serde_json::Value {
        let path = format!("{}/../../fixtures/expected/{name}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).expect("fixture readable");
        serde_json::from_str(&text).expect("fixture parses")
    }

    fn obs(x: f64, y: f64, resource: &str, unit: i64) -> RouteObservation {
        RouteObservation {
            x,
            y,
            resource_id: resource.to_string(),
            unit_value: unit,
            expected_yield: 2.0,
            spawn: 0.8,
            confidence: 0.6,
        }
    }

    fn tcluster(
        id: &str,
        x: f64,
        y: f64,
        eff: f64,
        members: Vec<f64>,
        conf: f64,
    ) -> NodeCluster {
        NodeCluster {
            id: id.to_string(),
            cx: x,
            cy: y,
            members: members.len(),
            qty: 0.0,
            value_gross: eff,
            value_effective: eff,
            member_values: members,
            resources: BTreeMap::new(),
            confidence: conf,
        }
    }

    #[test]
    fn geo_matches_shared_fixture() {
        let v = fixture("geo.json");
        for case in v["distances"].as_array().expect("distances array") {
            let a = &case["a"];
            let got =
                map_distance(a[0].as_f64().unwrap(), a[1].as_f64().unwrap(),
                    case["b"][0].as_f64().unwrap(), case["b"][1].as_f64().unwrap())
                    * case["yards_per_unit"].as_f64().unwrap();
            let want = case["yards"].as_f64().unwrap();
            assert!(approx(got, want, 1e-9), "{}: got {got}, want {want}", case["id"]);
        }
    }

    #[test]
    fn node_value_formula() {
        let o = obs(0.0, 0.0, "ore-test", 50_000);
        let (gross, eff) = expected_node_value(&o);
        // gross = 0.8 × 2 × 50000 = 80000; modifier = 0.5+0.5×0.6 = 0.8.
        assert!(approx(gross, 80_000.0, 1e-6), "gross {gross}");
        assert!(approx(eff, 64_000.0, 1e-6), "effective {eff}");
    }

    /// Hand geometry: 4 ore around (10,10), 2 herb at (12..13,12) in the
    /// diagonally-adjacent cell (must merge: diameter ≈3.6 ≤ 14), 3 ore at (50,50).
    fn hand_obs() -> Vec<RouteObservation> {
        vec![
            obs(10.0, 10.0, "ore-test", 50_000),
            obs(11.0, 10.0, "ore-test", 50_000),
            obs(10.0, 11.0, "ore-test", 50_000),
            obs(11.0, 11.0, "ore-test", 50_000),
            obs(12.0, 12.0, "herb-test", 120_000),
            obs(13.0, 12.0, "herb-test", 120_000),
            obs(50.0, 50.0, "ore-test", 50_000),
            obs(51.0, 50.0, "ore-test", 50_000),
            obs(50.0, 51.0, "ore-test", 50_000),
        ]
    }

    fn test_config() -> RouteConfig {
        RouteConfig { min_samples: 2, ..RouteConfig::default() }
    }

    #[test]
    fn clusters_match_hand_geometry() {
        let out = cluster_observations(&hand_obs(), &test_config());
        assert_eq!(out.clusters.len(), 2);
        assert_eq!(out.dropped, 0);
        let (c0, c1) = (&out.clusters[0], &out.clusters[1]);
        assert_eq!((c0.members, c1.members), (6, 3));
        assert!(approx(c0.cx, 67.0 / 6.0, 1e-9) && approx(c0.cy, 11.0, 1e-9));
        assert!(approx(c1.cx, 151.0 / 3.0, 1e-9) && approx(c1.cy, 151.0 / 3.0, 1e-9));
        // 4×64000 + 2×153600 = 563200; 3×64000 = 192000.
        assert!(approx(c0.value_effective, 563_200.0, 1e-3), "{}", c0.value_effective);
        assert!(approx(c1.value_effective, 192_000.0, 1e-3), "{}", c1.value_effective);
        assert!(approx(c0.value_gross, 704_000.0, 1e-3), "{}", c0.value_gross);
        assert_eq!(c0.id, "c0");
    }

    #[test]
    fn clusters_match_shared_fixture() {
        let v = fixture("cluster.json");
        for case in v["cases"].as_array().expect("cases array") {
            let cfg_json = &case["config"];
            let cfg = RouteConfig {
                cell_size: cfg_json["cellSize"].as_f64().unwrap(),
                max_diameter: cfg_json["maxDiameter"].as_f64().unwrap(),
                min_samples: cfg_json["minSamples"].as_u64().unwrap() as usize,
                ..RouteConfig::default()
            };
            let mut input = Vec::new();
            for o in case["observations"].as_array().expect("observations") {
                input.push(RouteObservation {
                    x: o["x"].as_f64().unwrap(),
                    y: o["y"].as_f64().unwrap(),
                    resource_id: o["resource"].as_str().unwrap().to_string(),
                    unit_value: o["unit"].as_i64().unwrap(),
                    expected_yield: o["yield"].as_f64().unwrap(),
                    spawn: o["spawn"].as_f64().unwrap(),
                    confidence: o["confidence"].as_f64().unwrap(),
                });
            }
            let out = cluster_observations(&input, &cfg);
            let ex = &case["expect"];
            let id = case["id"].as_str().unwrap();
            assert_eq!(out.clusters.len(), ex["count"].as_u64().unwrap() as usize, "{id}: count");
            for (i, c) in out.clusters.iter().enumerate() {
                assert_eq!(c.members, ex["sizes"][i].as_u64().unwrap() as usize, "{id}: size {i}");
                assert!(approx(c.cx, ex["centroids"][i][0].as_f64().unwrap(), 1e-9), "{id}: cx {i}");
                assert!(approx(c.cy, ex["centroids"][i][1].as_f64().unwrap(), 1e-9), "{id}: cy {i}");
                assert!(approx(c.value_effective, ex["values"][i].as_f64().unwrap(), 1e-3),
                    "{id}: value {i}");
            }
        }
    }

    #[test]
    fn min_samples_prunes_singletons() {
        let mut input = hand_obs();
        input.push(obs(90.0, 90.0, "ore-test", 50_000));
        let out = cluster_observations(&input, &test_config());
        assert_eq!(out.clusters.len(), 2);
        assert_eq!(out.dropped, 1);
    }

    #[test]
    fn two_opt_repairs_crossed_tour() {
        let clusters = vec![
            tcluster("c0", 0.0, 0.0, 1.0, vec![1.0], 1.0),
            tcluster("c1", 100.0, 0.0, 1.0, vec![1.0], 1.0),
            tcluster("c2", 0.0, 100.0, 1.0, vec![1.0], 1.0),
            tcluster("c3", 100.0, 100.0, 1.0, vec![1.0], 1.0),
        ];
        let bad = vec![0, 3, 1, 2];
        let before = tour_length_yards(&bad, &clusters, 1.0);
        assert!(approx(before, 200.0 + 200.0 * 2f64.sqrt(), 1e-6), "{before}");
        let fixed = two_opt(&bad, &clusters, 1.0, 8);
        let after = tour_length_yards(&fixed, &clusters, 1.0);
        assert!(approx(after, 400.0, 1e-6), "{after}");
        assert_eq!(fixed, vec![0, 1, 3, 2]);
    }

    #[test]
    fn routes_match_shared_fixture() {
        let v = fixture("route.json");
        for case in v["cases"].as_array().expect("cases array") {
            let id = case["id"].as_str().unwrap();
            let ypu = case["ypu"].as_f64().unwrap();
            let cfg_json = &case["config"];
            let cfg = RouteConfig {
                seed: cfg_json["seed"].as_u64().unwrap(),
                travel_speed_yps: cfg_json["travelSpeedYps"].as_f64().unwrap(),
                gather_seconds: cfg_json["gatherSeconds"].as_f64().unwrap(),
                loop_overhead_seconds: cfg_json["loopOverheadSeconds"].as_f64().unwrap(),
                two_opt_passes: cfg_json["twoOptPasses"].as_u64().unwrap() as usize,
                ..RouteConfig::default()
            };
            let mut clusters = Vec::new();
            for c in case["clusters"].as_array().expect("clusters") {
                clusters.push(NodeCluster {
                    id: c["id"].as_str().unwrap().to_string(),
                    cx: c["x"].as_f64().unwrap(),
                    cy: c["y"].as_f64().unwrap(),
                    members: c["members"].as_u64().unwrap() as usize,
                    qty: c["qty"].as_f64().unwrap(),
                    value_gross: c["gross"].as_f64().unwrap(),
                    value_effective: c["effective"].as_f64().unwrap(),
                    member_values: c["memberValues"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_f64().unwrap())
                        .collect(),
                    resources: BTreeMap::new(),
                    confidence: c["confidence"].as_f64().unwrap(),
                });
            }
            let ex = &case["expect"];
            let start = case["start"].as_u64().unwrap() as usize;
            let nn = nearest_neighbor(&clusters, start, ypu, cfg.seed);
            let want_nn: Vec<usize> = ex["nnOrder"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as usize)
                .collect();
            assert_eq!(nn, want_nn, "{id}: nn order");
            let init: Vec<usize> = match case.get("initialOrder") {
                Some(arr) => arr
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_u64().unwrap() as usize)
                    .collect(),
                None => nn.clone(),
            };
            let opt = two_opt(&init, &clusters, ypu, cfg.two_opt_passes);
            let want_opt: Vec<usize> = ex["optOrder"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as usize)
                .collect();
            assert_eq!(opt, want_opt, "{id}: 2-opt order");
            let sim = simulate(&opt, &clusters, &cfg, ypu);
            assert!(approx(sim.loop_yards, ex["loopYards"].as_f64().unwrap(), 1e-6), "{id}: yards");
            assert!(approx(sim.loop_seconds, ex["loopSeconds"].as_f64().unwrap(), 1e-6), "{id}: sec");
            assert!(approx(sim.gph_expected, ex["gphExpected"].as_f64().unwrap(), 1e-3), "{id}: gph");
            assert!(approx(sim.gph_conservative, ex["gphConservative"].as_f64().unwrap(), 1e-3),
                "{id}: conservative");
            assert!(approx(sim.band_hourly.p10, ex["band"][0].as_f64().unwrap(), 1e-3), "{id}: p10");
            assert!(approx(sim.band_hourly.median, ex["band"][1].as_f64().unwrap(), 1e-3),
                "{id}: median");
            assert!(approx(sim.band_hourly.p90, ex["band"][2].as_f64().unwrap(), 1e-3), "{id}: p90");
        }
    }

    #[test]
    fn simulation_band_is_hand_checked() {
        // A: 2 members → default spread (conf 0.5 → 0.475). B: 1 member, conf 1 → 0.35.
        let mut a = tcluster("c0", 0.0, 0.0, 100_000.0, vec![60_000.0, 40_000.0], 0.5);
        let mut b = tcluster("c1", 3.0, 4.0, 100_000.0, vec![100_000.0], 1.0);
        a.qty = 4.0;
        b.qty = 2.0;
        let clusters = vec![a, b];
        let cfg = RouteConfig::default();
        let sim = simulate(&[0, 1], &clusters, &cfg, 10.0);
        // Loop 100 yd: 100/7 + 3×3.5 + 15 = 39.7857…s.
        assert!(approx(sim.loop_yards, 100.0, 1e-9));
        assert!(approx(sim.loop_seconds, 100.0 / 7.0 + 10.5 + 15.0, 1e-9));
        assert_eq!(sim.nodes, 3);
        // variance = 47500² + 35000² = 3,481,250,000; p10 loop = 200000 − 1.28σ.
        let loop_p10 = sim.gph_conservative * (sim.loop_seconds / 3600.0);
        assert!(approx(loop_p10, 124_385.7, 2.0), "loop p10 {loop_p10}");
        assert!(sim.band_hourly.p10 <= sim.band_hourly.median);
        assert!(sim.band_hourly.median <= sim.band_hourly.p90);
        assert!(approx(sim.gph_expected, sim.band_hourly.median, 1e-9));
    }

    #[test]
    fn candidates_are_deterministic_per_seed() {
        let out = cluster_observations(&hand_obs(), &test_config());
        let cfg = RouteConfig::default();
        let a = build_candidates(&out.clusters, &cfg, 10.0).expect("builds");
        let b = build_candidates(&out.clusters, &cfg, 10.0).expect("builds");
        let sa = serde_json::to_string(&a.candidates.iter().map(|c| &c.order).collect::<Vec<_>>())
            .expect("json");
        let sb = serde_json::to_string(&b.candidates.iter().map(|c| &c.order).collect::<Vec<_>>())
            .expect("json");
        assert_eq!(sa, sb);
        // Ranked best-first and every tour covers all clusters.
        for w in a.candidates.windows(2) {
            assert!(w[0].gph_expected >= w[1].gph_expected);
        }
        for c in &a.candidates {
            let mut seen = c.order.clone();
            seen.sort_unstable();
            assert_eq!(seen, vec![0, 1]);
        }
    }

    #[test]
    fn go1_hand_vector() {
        // Hand checksum: "1.2.000000010010" → 1 + 6 + 12 + 15 = 34.
        let code = go1_encode(1, &[(0.0, 0.0), (1.0, 1.0)]).expect("encodes");
        assert_eq!(code, "GO1.1.2.000000010010.034");
        let (map, wps) = go1_decode(&code).expect("decodes");
        assert_eq!(map, 1);
        assert_eq!(wps, vec![(0.0, 0.0), (1.0, 1.0)]);
    }

    #[test]
    fn go1_matches_shared_vectors() {
        let v = fixture("go1.json");
        for vec in v["vectors"].as_array().expect("vectors") {
            let map = vec["map"].as_i64().unwrap();
            let wps: Vec<(f64, f64)> = vec["waypoints"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
                .collect();
            let code = go1_encode(map, &wps).expect("encodes");
            assert_eq!(code, vec["code"].as_str().unwrap());
            let (dm, dw) = go1_decode(&code).expect("decodes");
            assert_eq!(dm, map);
            assert_eq!(dw.len(), wps.len());
            for (got, want) in dw.iter().zip(wps.iter()) {
                assert!(approx(got.0, want.0, 1e-9) && approx(got.1, want.1, 1e-9));
            }
        }
    }

    #[test]
    fn go1_rejects_garbage() {
        assert!(go1_decode("GO1.1.2.000000010010.035").is_err()); // bad checksum
        assert!(go1_decode("GO2.1.2.000000010010.034").is_err()); // version
        assert!(go1_decode("GO1.1.2.00000001.034").is_err()); // length
        assert!(go1_decode("nope").is_err());
        assert!(go1_decode("GO1.1.1.000000.034").is_err()); // count < 2
        assert!(go1_encode(1, &[(0.0, 0.0); 41]).is_err()); // over cap
        assert!(go1_encode(1, &[(0.0, 0.0)]).is_err()); // under min
    }

    fn seeded_db() -> Connection {
        let mut conn = open_memory().unwrap();
        crate::migrations::migrate(&mut conn).unwrap();
        // DB-shaped twin of the hand-9 geometry (fixtures/observations_sample.json).
        let path = format!(
            "{}/../../fixtures/observations_sample.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let zone = &v["zone"];
        let zone_id = zone["id"].as_str().unwrap();
        conn.execute(
            "INSERT INTO zones (id, name, map_id, yards_per_unit) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                zone_id,
                zone["name"].as_str().unwrap(),
                zone["mapId"].as_i64().unwrap(),
                zone["yardsPerUnit"].as_f64().unwrap(),
            ],
        )
        .unwrap();
        let rows = v["observations"].as_array().unwrap();
        for o in rows {
            conn.execute(
                "INSERT INTO resource_observations
                 (zone_id, resource_id, x, y, yield_qty, source, observed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                rusqlite::params![
                    zone_id,
                    o["resource"].as_str().unwrap(),
                    o["x"].as_f64().unwrap(),
                    o["y"].as_f64().unwrap(),
                    o["yield"].as_f64().unwrap(),
                    o["source"].as_str().unwrap(),
                ],
            )
            .unwrap();
        }
        assert_eq!(rows.len(), 9);
        conn
    }

    #[test]
    fn load_requires_prices() {
        let conn = seeded_db();
        let res = vec!["ore-test".to_string()];
        let empty: HashMap<String, i64> = HashMap::new();
        let err = load_observations(&conn, "zone-test", &res, &[], &empty).unwrap_err();
        assert!(err.user_message().contains("ore-test"), "{}", err.user_message());
        let mut prices = HashMap::new();
        prices.insert("ore-test".to_string(), 50_000);
        let got = load_observations(&conn, "zone-test", &res, &[], &prices).expect("loads");
        assert_eq!(got.len(), 7);
    }

    #[test]
    fn exclusions_remove_points() {
        let conn = seeded_db();
        let res = vec!["ore-test".to_string(), "herb-test".to_string()];
        let mut prices = HashMap::new();
        prices.insert("ore-test".to_string(), 50_000);
        prices.insert("herb-test".to_string(), 120_000);
        let all = ExcludedArea { x: 30.0, y: 30.0, radius: 100.0 };
        let got =
            load_observations(&conn, "zone-test", &res, std::slice::from_ref(&all), &prices)
                .expect("loads");
        assert!(got.is_empty());
        let none =
            load_observations(&conn, "zone-test", &res, &[], &prices).expect("loads");
        assert_eq!(none.len(), 9);
    }

    #[test]
    fn generate_save_list_compare_delete() {
        let conn = seeded_db();
        let res = vec!["ore-test".to_string(), "herb-test".to_string()];
        let mut prices = HashMap::new();
        prices.insert("ore-test".to_string(), 50_000);
        prices.insert("herb-test".to_string(), 120_000);
        let cfg = RouteConfig::default();
        let cands = generate(&conn, "zone-test", &res, &cfg, &prices, &[]).expect("generates");
        assert!(!cands.is_empty() && cands.len() <= 3);
        assert!(cands[0].waypoints.len() >= 2);
        // Deterministic: identical JSON twice.
        let again = generate(&conn, "zone-test", &res, &cfg, &prices, &[]).expect("generates");
        assert_eq!(
            serde_json::to_string(&cands).expect("json"),
            serde_json::to_string(&again).expect("json")
        );

        let input = SaveRouteInput {
            name: "Test Loop".to_string(),
            zone_id: Some("zone-test".to_string()),
            resource_ids: res.clone(),
            waypoints: cands[0].waypoints.clone(),
            excluded: vec![],
            config: cfg.clone(),
            stats: cands[0].stats.clone(),
        };
        let saved = save_route(&conn, &input, 1_700_000_000).expect("saves");
        assert!(saved.id.starts_with("rt-"));
        let listed = list_routes(&conn, Some("zone-test")).expect("lists");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].zone_name.as_deref(), Some("Test Zone"));
        assert_eq!(listed[0].waypoint_count, saved.waypoints.len());
        let fetched = get_route(&conn, &saved.id).expect("gets");
        assert_eq!(fetched.name, "Test Loop");
        let compared = compare_routes(&conn, std::slice::from_ref(&saved.id)).expect("compares");
        assert_eq!(compared.len(), 1);
        assert!(delete_route(&conn, &saved.id).expect("deletes"));
        assert!(!delete_route(&conn, &saved.id).expect("deletes"));
        assert!(list_routes(&conn, None).expect("lists").is_empty());
    }
}

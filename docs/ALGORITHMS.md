# ALGORITHMS

All formulas implemented in `crates/core` and cross-checked by
`scripts/verify-algorithms.py` against `fixtures/expected/*.json`.

## 1. Geo math

- Coordinates normalized to 0–100 per map. Distance in "yards" approximated with
  `YARDS_PER_UNIT` scale per map (default 10.0, stored per zone, documented estimate).
- Haversine not needed (single-map Euclidean); cross-map travel uses fixed portal/
  flight penalties from config (`travel.portal_penalty_s`).

## 2. Clustering (routes.rs)

Grid-hash clustering on normalized coords:

1. cell size = `cell_size` (default 4.0 units ≈ 40 yd).
2. merge 8-neighborhood (Chebyshev ≤ 1) non-empty cells via union-find while the
   exact merged diameter ≤ `max_diameter` (default 14.0 units). Union by size;
   ties broken by smaller cell index.
3. drop groups with `members < min_samples` (default 2) — seed points included,
   since a lone weak prior is noise, not a cluster.
4. cluster value = Σ `ExpectedNodeValue` of member observations (see §4);
   clusters ranked by value desc (ties: first member index) and labeled c0….

Performance guards: at most `MAX_CLUSTER_POINTS` (2000) points per run (input
order, deterministic; the rest counted as truncated); cross-cell merges whose
union exceeds 800 members are skipped (such a union exceeds any sane diameter).
Deterministic: sorted cells/pairs/members, no hash-order dependence.

## 3. Route construction + improvement

1. Prune: keep the top `max_clusters` (default 24) clusters by value.
2. Seed: greedy nearest-neighbor tours from each of the top `candidate_starts`
   (default 3) clusters. Ties: (distance, FNV-1a-64(id, seed)) — exact integer
   math, identical across implementations.
3. Improve: first-improvement 2-opt in deterministic scan order, bounded by
   `two_opt_passes` (default 8).
4. Rank all candidates by expected GPH desc; the top 3 are returned.

Everything deterministic for a fixed seed; no RNG anywhere in the pipeline.

## 4. Node / cluster valuation

```text
ExpectedNodeValue =
    SpawnProbability
  × ExpectedYield
  × EffectiveMarketValue
  × ConfidenceModifier
```

- `SpawnProbability`: blend(canonical prior, personal hit-rate, crowd prior) — §7.
- `ExpectedYield`: blend(global prior qty, personal avg yield) by sample size.
- `EffectiveMarketValue`: from market engine (§5), copper per unit.
- `ConfidenceModifier`: 0.5 + 0.5 × node confidence (bounded, documented).
- Phase-4 source priors (spawn, confidence): addon (0.75, 0.7), import
  (0.6, 0.5), crowd (0.55, 0.45), seed/unknown (0.5, 0.4). §7 blending
  supersedes these once personal/crowd evidence exists.

## 5. Market valuation (market.rs)

Inputs per item: marketValue, minBuyout, recent, historical (live file),
avgSalePrice, saleRate, soldPerDay (region file), `market_updated_at`,
`region_updated_at`, `fetched_at`. Tunables in `ValuationConfig` (defaults below).

- `MarketValue` = live quote clamped to the baseline band. Baseline =
  max(historical, avgSalePrice); if live > 2.5 × baseline pin to the ceiling
  (flag `spiked`), if live < baseline/3 raise to the floor (flag `floored`).
  No baseline → clamp off (flag `no-baseline`); no live quote → value from
  history (flag `live-unknown`, display 0).
- `ConservativeSellValue` = min(clamped market, minBuyout?, avgSale?, historical?)
  over positive signals × liquidity haircut (0.90 + 0.10 × saleRate).
  Only `recent` positive → fall back to it (flag `recent-only`).
- `EffectiveRouteValue` = ConservativeSellValue × freshness penalty, where
  penalty = 1 while age ≤ refresh_hours (default 12), then
  max(0.5, 1 − 0.125 × (age/refresh − 1)); flag `stale` below 1.
- Confidence = 0.35 × agreement + 0.25 × liquidity + 0.25 × recency
  + 0.15 × coverage: agreement = 1 − CV over positive price signals
  (0.5 for a single signal); liquidity = 0.5 × soldPerDay/20 + 0.5 × saleRate
  (clamped); recency = freshness penalty; coverage = signals present / 5.
  Flag `thin` when soldPerDay < 1.
- Missing/zero signals → Unknown (zeros, confidence 0, flag `no-data`).
  Stale data usable with penalty; missing data → Unknown (routes show the
  gap explicitly, never a fake price).

## 6. Simulation (routes.rs)

Per candidate route, Monte-Carlo-free analytic bands:

- Expected loop time = travel (loop yards / speed) + nodes × gather_time
  + `loop_overhead_seconds` (mount/dismount/startup, default 15 s).
- Speed profile: generic default (7 yd/s mounted + stops) → personal learned speed.
- Per-cluster spread from member-value dispersion: CV = std/mean clamped to
  [0.15, 0.60]; clusters with < 3 members use 0.35 + 0.25 × (1 − confidence).
- Loop variance = Σ (value_i × spread_i)² (clusters treated as independent);
  P10/P90 = effective ∓ 1.2816σ, P10 floored at 0.
- Hourly figures = loop figures / loop hours. Conservative GPH = P10 hourly.

Outputs: loop_s, nodes, qty, gross/ effective value, GPH (expected/conservative),
P10/median/P90 value per hour.

## 7. Belief blending (intelligence.rs + crowd.rs)

```text
blended = (prior×prior_w + personal×personal_w + crowd×crowd_w) / Σw
personal_w = min(samples / min_samples, 1) × recency_decay(age)
crowd_w    = agreement × diversity × recency × min(contributors/crowd_min, 1), capped ≤ 0.35
```

Personal always wins with enough evidence. Decay half-life configurable (default 21 days).

Phase 7 implementation notes (`intelligence.rs`): learning is a deterministic
full rebuild over `analyzed` sessions (uniform sums in SQL; recency decay
applies at read time via `updated_at`, so stored stats never drift). Yield
pools every analyzed session (zero-gather loot excluded); zone GPH pools
linked + unlinked sessions, but efficiency compares linked sessions against
their own expectations only — unknown zones stay neutral 1.0. Personalized
routes blend observation yields, then scale effective/GPH/bands by zone
efficiency while gross stays the unadjusted model reference. `hit_rate` is
reserved: the addon reports no attempts, so nothing is learned there.

## 8. Opportunity score (recommendations)

```text
OpportunityScore =
    ExpectedGPH × MarketConfidence × RouteConfidence × PersonalConfidence × FreshnessModifier
```

Ranked across zones/resources/routes with per-factor explanation strings.
Never a bare "Farm X" — always why + risks.

## 9. Session analytics (sessions.rs)

Sessions arrive via addon sync as `imported`; analysis snapshots per-item unit
values (market effective value) and flips status to `analyzed`. Snapshots are
first-wins: re-analysis only fills missing prices, so THEN-values stay stable
while NOW-values track the live market.

- Totals: Σ qty × unit over priced lines; unpriced lines set `thenPartial` /
  `nowPartial` instead of silently contributing zero.
- GPH = value / active_hours (None without active time).
- Route links snapshot the route's GPH + per-loop value; expected session
  total = GPH × active_hours; delta % = (actual − expected) / expected × 100
  (None without a baseline).
- Stats bucket by UTC day (`date(started_at, 'unixepoch')`) and by resource
  (unmapped loot buckets under `item:<id>`).

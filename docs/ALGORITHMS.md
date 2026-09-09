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

Inputs per item: marketValue, minBuyout, historical, saleAvg, saleRate?, quantity?,
`market_updated_at`, `region_updated_at`.

- `MarketValue` = preferred live signal (commodities → marketValue/minBuyout composite).
- `ConservativeSellValue` = min(MarketValue, saleAvg?, historical) with liquidity haircut.
- `EffectiveRouteValue` = ConservativeSellValue × freshness penalty × spike clamp.
- Spike protection: if `MarketValue > clamp × max(historical, saleAvg)` (default clamp 2.5),
  pin to clamp × baseline and degrade confidence; symmetric floor at 1/3.
- Confidence from: signal agreement, sale volume/rate, recency, provider health.
- Stale data usable with penalty; missing data → Unknown (route shows risk, never fake price).

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

## 8. Opportunity score (recommendations)

```text
OpportunityScore =
    ExpectedGPH × MarketConfidence × RouteConfidence × PersonalConfidence × FreshnessModifier
```

Ranked across zones/resources/routes with per-factor explanation strings.
Never a bare "Farm X" — always why + risks.

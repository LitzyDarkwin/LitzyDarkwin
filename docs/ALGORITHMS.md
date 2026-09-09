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

1. cell size = `cluster.cell_size` (default 4.0 units ≈ 40 yd).
2. merge adjacent non-empty cells while cluster diameter ≤ `cluster.max_diameter`.
3. drop clusters with `observations < cluster.min_samples` unless seeded prior.
4. cluster value = Σ `ExpectedNodeValue` of member observations (see §4).

Deterministic: sorted cell iteration, no hash-order dependence.

## 3. Route construction + improvement

1. Seed: greedy nearest-neighbor from best-value cluster (seeded tie-break).
2. Insert: greedy cheapest-insertion for remaining candidate clusters.
3. Improve: bounded 2-opt (`two_opt.max_passes`, default 8) + capped candidate count
   (`candidates.max_clusters`, default 24). Everything seeded/deterministic.

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

- Expected loop time = travel (path length / speed profile) + gathers × gather_time.
- Speed profile: generic default (7 yd/s mounted + stops) → personal learned speed.
- Expected qty/value per loop → per-hour rates; P10/P90 via yield/spawn variance model
  (lognormal-ish spread from observed variance, min-sample guarded).

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

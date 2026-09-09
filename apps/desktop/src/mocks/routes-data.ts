/**
 * Canned route-engine responses for the mock backend. These are fixed sample
 * DTOs (not computed) so the Routes page can be reviewed without the Rust
 * core; the real optimizer lives behind the Tauri commands.
 */
import type {
  RouteCandidate,
  RouteConfig,
  RouteStats,
  SavedRoute,
  Waypoint,
} from '../types/domain';

export const MOCK_ROUTE_CONFIG: RouteConfig = {
  seed: 20260909,
  cellSize: 4,
  maxDiameter: 14,
  minSamples: 2,
  maxClusters: 24,
  candidateStarts: 3,
  twoOptPasses: 8,
  travelSpeedYps: 7,
  gatherSeconds: 3.5,
  loopOverheadSeconds: 15,
};

function stats(over: Partial<RouteStats>): RouteStats {
  return {
    loopYards: 4200,
    loopSeconds: 900,
    nodes: 18,
    qty: 29,
    grossValue: 1_900_000,
    effectiveValue: 1_520_000,
    gphExpected: 6_080_000,
    gphConservative: 4_300_000,
    bandHourly: { p10: 4_300_000, median: 6_080_000, p90: 7_860_000 },
    resolvedPrices: { bismuth: 50_000, mycobloom: 120_000 },
    prunedClusters: 0,
    droppedPoints: 0,
    warnings: [],
    ...over,
  };
}

function wp(x: number, y: number, clusterId: string, value: number): Waypoint {
  return { x, y, clusterId, value };
}

export const MOCK_CANDIDATES: RouteCandidate[] = [
  {
    rank: 1,
    config: MOCK_ROUTE_CONFIG,
    personal: null,
    waypoints: [
      wp(30, 62, 'c0', 420_000),
      wp(44, 55, 'c1', 380_000),
      wp(58, 60, 'c2', 360_000),
      wp(52, 74, 'c3', 360_000),
    ],
    stats: stats({}),
  },
  {
    rank: 2,
    config: MOCK_ROUTE_CONFIG,
    personal: null,
    waypoints: [
      wp(30, 62, 'c0', 420_000),
      wp(58, 60, 'c2', 360_000),
      wp(44, 55, 'c1', 380_000),
      wp(36, 72, 'c4', 300_000),
    ],
    stats: stats({
      loopYards: 4600,
      loopSeconds: 980,
      nodes: 16,
      qty: 26,
      effectiveValue: 1_460_000,
      gphExpected: 5_360_000,
      gphConservative: 3_800_000,
      bandHourly: { p10: 3_800_000, median: 5_360_000, p90: 6_920_000 },
      prunedClusters: 1,
      warnings: ['pruned 1 low-value cluster(s)'],
    }),
  },
];

export function mockSavedRoute(): SavedRoute {
  const first = MOCK_CANDIDATES[0];
  return {
    id: 'rt-mock-1',
    name: 'Dorn Bismuth Loop (sample)',
    zoneId: 'isle-of-dorn',
    resourceIds: ['bismuth', 'mycobloom'],
    waypoints: first.waypoints,
    excluded: [],
    config: { ...MOCK_ROUTE_CONFIG },
    stats: { ...first.stats },
    createdAt: 1_750_000_000,
    updatedAt: 1_750_000_000,
  };
}

/**
 * Canned personal-intelligence responses for the mock backend. Values are
 * hand-computed (personalized = plain × 0.85 efficiency, gross untouched)
 * so the UI can be reviewed without the Rust core.
 */
import type {
  PersonalProfile,
  RebuildReport,
  RouteCandidate,
  RouteStats,
} from '../types/domain';
import { MOCK_CANDIDATES } from './routes-data';

const EFFICIENCY = 0.85;

function scaleStats(s: RouteStats): RouteStats {
  return {
    ...s,
    effectiveValue: Math.round(s.effectiveValue * EFFICIENCY),
    gphExpected: Math.round(s.gphExpected * EFFICIENCY),
    gphConservative: Math.round(s.gphConservative * EFFICIENCY),
    bandHourly: {
      p10: Math.round(s.bandHourly.p10 * EFFICIENCY),
      median: Math.round(s.bandHourly.median * EFFICIENCY),
      p90: Math.round(s.bandHourly.p90 * EFFICIENCY),
    },
  };
}

export const MOCK_PERSONAL_CANDIDATES: RouteCandidate[] = MOCK_CANDIDATES.map((c) => ({
  ...c,
  stats: scaleStats(c.stats),
  personal: {
    efficiency: EFFICIENCY,
    efficiencyZone: 'isle-of-dorn',
    yieldsBlended: 12,
    observations: 40,
    explain: [
      'bismuth: yield 2.00 → 2.41 (120 gathers, w=1.00)',
      'mycobloom: yield 2.00 → 2.13 (40 gathers, w=1.00)',
      'zone efficiency 0.82 (8 sessions, w=0.80) → ×0.85',
    ],
  },
}));

export const MOCK_PROFILE: PersonalProfile = {
  metrics: [
    { key: 'efficiency_global', value: 0.85, samples: 10, updatedAt: 1_750_000_000 },
    { key: 'sessions_analyzed', value: 12, samples: 12, updatedAt: 1_750_000_000 },
  ],
  zones: [
    {
      zoneId: 'isle-of-dorn',
      zoneName: 'Isle of Dorn',
      sessions: 8,
      totalSeconds: 21600,
      avgGph: 5_500_000,
      efficiency: 0.85,
      updatedAt: 1_750_000_000,
    },
  ],
  resources: [
    {
      resourceId: 'bismuth',
      resourceName: 'Bismuth',
      gathers: 120,
      totalQty: 300,
      avgYield: 2.5,
      avgGoldPerGather: 125_000,
      updatedAt: 1_750_000_000,
    },
    {
      resourceId: 'mycobloom',
      resourceName: 'Mycobloom',
      gathers: 40,
      totalQty: 88,
      avgYield: 2.2,
      avgGoldPerGather: 264_000,
      updatedAt: 1_750_000_000,
    },
  ],
};

export const MOCK_REBUILD_REPORT: RebuildReport = {
  sessions: 12,
  resources: 2,
  zones: 1,
};

/**
 * Deterministic mock atlas data for browser preview + tests.
 * Shapes mirror the real `get_atlas_tree`/`search_resources`/`get_zone_detail`/
 * `get_zone_observations` DTOs. Observation grids come from a seeded LCG so
 * they are stable across renders.
 */
import type {
  AtlasTree,
  ClusteredObservations,
  GridCell,
  ResourceHit,
  ZoneDetail,
} from '../types/domain';

export const MOCK_TREE: AtlasTree = {
  expansions: [
    {
      id: 'tww',
      name: 'The War Within',
      short: 'TWW',
      continents: [
        {
          id: 'khaz-algar',
          name: 'Khaz Algar',
          zones: [
            { id: 'isle-of-dorn', name: 'Isle of Dorn', mapId: 2248, observationCount: 80, resourceCount: 3 },
            { id: 'ringing-deeps', name: 'The Ringing Deeps', mapId: 2214, observationCount: 88, resourceCount: 4 },
          ],
        },
      ],
    },
    {
      id: 'df',
      name: 'Dragonflight',
      short: 'DF',
      continents: [
        {
          id: 'dragon-isles',
          name: 'Dragon Isles',
          zones: [
            { id: 'ohnahran-plains', name: "Ohn'ahran Plains", mapId: 2023, observationCount: 60, resourceCount: 3 },
          ],
        },
      ],
    },
  ],
};

const DETAILS: Record<string, ZoneDetail> = {
  'isle-of-dorn': {
    id: 'isle-of-dorn',
    name: 'Isle of Dorn',
    mapId: 2248,
    continentId: 'khaz-algar',
    continentName: 'Khaz Algar',
    expansionId: 'tww',
    expansionName: 'The War Within',
    yardsPerUnit: 10,
    observationCount: 80,
    resources: [
      { resourceId: 'bismuth', resourceName: 'Bismuth', categoryId: 'mining', itemId: 161502, observations: 26, sources: ['seed'], avgYield: 2.3, newestObservation: null },
      { resourceId: 'ironclaw-ore', resourceName: 'Ironclaw Ore', categoryId: 'mining', itemId: 161503, observations: 30, sources: ['seed'], avgYield: 2.2, newestObservation: null },
      { resourceId: 'mycobloom', resourceName: 'Mycobloom', categoryId: 'herbalism', itemId: 161504, observations: 24, sources: ['seed'], avgYield: 2.4, newestObservation: null },
    ],
  },
  'ringing-deeps': {
    id: 'ringing-deeps',
    name: 'The Ringing Deeps',
    mapId: 2214,
    continentId: 'khaz-algar',
    continentName: 'Khaz Algar',
    expansionId: 'tww',
    expansionName: 'The War Within',
    yardsPerUnit: 10,
    observationCount: 88,
    resources: [
      { resourceId: 'bismuth', resourceName: 'Bismuth', categoryId: 'mining', itemId: 161502, observations: 30, sources: ['seed'], avgYield: 2.3, newestObservation: null },
      { resourceId: 'ironclaw-ore', resourceName: 'Ironclaw Ore', categoryId: 'mining', itemId: 161503, observations: 28, sources: ['seed'], avgYield: 2.2, newestObservation: null },
      { resourceId: 'mycobloom', resourceName: 'Mycobloom', categoryId: 'herbalism', itemId: 161504, observations: 22, sources: ['seed'], avgYield: 2.4, newestObservation: null },
      { resourceId: 'luredrop', resourceName: 'Luredrop', categoryId: 'herbalism', itemId: 161505, observations: 8, sources: ['seed'], avgYield: 2.1, newestObservation: null },
    ],
  },
  'ohnahran-plains': {
    id: 'ohnahran-plains',
    name: "Ohn'ahran Plains",
    mapId: 2023,
    continentId: 'dragon-isles',
    continentName: 'Dragon Isles',
    expansionId: 'df',
    expansionName: 'Dragonflight',
    yardsPerUnit: 10,
    observationCount: 60,
    resources: [
      { resourceId: 'serevite-ore', resourceName: 'Serevite Ore', categoryId: 'mining', itemId: 161506, observations: 26, sources: ['seed'], avgYield: 2.3, newestObservation: null },
      { resourceId: 'draconium-ore', resourceName: 'Draconium Ore', categoryId: 'mining', itemId: 161507, observations: 10, sources: ['seed'], avgYield: 1.2, newestObservation: null },
      { resourceId: 'hochenblume', resourceName: 'Hochenblume', categoryId: 'herbalism', itemId: 161508, observations: 24, sources: ['seed'], avgYield: 2.5, newestObservation: null },
    ],
  },
};

/** Tiny deterministic PRNG (mulberry32) for stable mock grids. */
function rng(seed: number) {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function seedOf(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function mockObservations(
  zoneId: string,
  resourceIds: string[],
  grid: number,
): ClusteredObservations {
  const detail = DETAILS[zoneId];
  const pool =
    resourceIds.length > 0
      ? resourceIds
      : (detail?.resources.map((r) => r.resourceId) ?? ['mock-resource']);
  const rand = rng(seedOf(`${zoneId}|${pool.join(',')}|${grid}`));
  const cellMap = new Map<string, { gx: number; gy: number; counts: Map<string, number>; sx: number; sy: number }>();
  const points = 40 + Math.floor(rand() * 40);
  for (let i = 0; i < points; i++) {
    // Clustered around 2-3 drifting centers for a map-like look.
    const center = Math.floor(rand() * 3);
    const cx = [30, 60, 45][center] + (rand() - 0.5) * 24;
    const cy = [35, 55, 70][center] + (rand() - 0.5) * 24;
    const x = Math.min(99, Math.max(1, cx));
    const y = Math.min(99, Math.max(1, cy));
    const gx = Math.min(grid - 1, Math.floor((x / 100) * grid));
    const gy = Math.min(grid - 1, Math.floor((y / 100) * grid));
    const key = `${gx},${gy}`;
    let cell = cellMap.get(key);
    if (!cell) {
      cell = { gx, gy, counts: new Map(), sx: 0, sy: 0 };
      cellMap.set(key, cell);
    }
    const res = pool[Math.floor(rand() * pool.length)];
    cell.counts.set(res, (cell.counts.get(res) ?? 0) + 1);
    cell.sx += x;
    cell.sy += y;
  }
  const cells: GridCell[] = [...cellMap.values()]
    .sort((a, b) => a.gy - b.gy || a.gx - b.gx)
    .map((c) => {
      const byResource = [...c.counts.entries()]
        .map(([resourceId, count]) => ({ resourceId, count }))
        .sort((a, b) => b.count - a.count || a.resourceId.localeCompare(b.resourceId));
      const total = byResource.reduce((n, r) => n + r.count, 0);
      return {
        gx: c.gx,
        gy: c.gy,
        cx: c.sx / total,
        cy: c.sy / total,
        count: total,
        topResource: byResource[0]?.resourceId ?? null,
        byResource,
      };
    });
  return { grid, cells, totalPoints: points, truncated: false };
}

export function mockSearch(query: string, category?: string | null): ResourceHit[] {
  const q = query.trim().toLowerCase();
  const all: ResourceHit[] = [
    { id: 'bismuth', name: 'Bismuth', categoryId: 'mining', categoryName: 'Mining', itemId: 161502, observationCount: 56, zones: [{ id: 'isle-of-dorn', name: 'Isle of Dorn' }, { id: 'ringing-deeps', name: 'The Ringing Deeps' }] },
    { id: 'ironclaw-ore', name: 'Ironclaw Ore', categoryId: 'mining', categoryName: 'Mining', itemId: 161503, observationCount: 58, zones: [{ id: 'isle-of-dorn', name: 'Isle of Dorn' }, { id: 'ringing-deeps', name: 'The Ringing Deeps' }] },
    { id: 'mycobloom', name: 'Mycobloom', categoryId: 'herbalism', categoryName: 'Herbalism', itemId: 161504, observationCount: 46, zones: [{ id: 'isle-of-dorn', name: 'Isle of Dorn' }, { id: 'ringing-deeps', name: 'The Ringing Deeps' }] },
    { id: 'luredrop', name: 'Luredrop', categoryId: 'herbalism', categoryName: 'Herbalism', itemId: 161505, observationCount: 8, zones: [{ id: 'ringing-deeps', name: 'The Ringing Deeps' }] },
    { id: 'serevite-ore', name: 'Serevite Ore', categoryId: 'mining', categoryName: 'Mining', itemId: 161506, observationCount: 26, zones: [{ id: 'ohnahran-plains', name: "Ohn'ahran Plains" }] },
  ];
  return all.filter(
    (r) =>
      (category == null || r.categoryId === category) &&
      (q === '' || r.name.toLowerCase().includes(q) || r.id.includes(q)),
  );
}

export function mockZoneDetail(zoneId: string): ZoneDetail {
  const detail = DETAILS[zoneId];
  if (!detail) throw new Error(`Unknown zone \`${zoneId}\``);
  return detail;
}

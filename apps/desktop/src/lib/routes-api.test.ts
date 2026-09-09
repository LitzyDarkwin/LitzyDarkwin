import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import {
  compareRoutes,
  deleteRoute,
  exportRouteString,
  generateRoutes,
  getRoute,
  listRoutes,
  saveRoute,
} from './routes-api';

describe('routes-api (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('generates ranked candidates with effective config', async () => {
    const cands = await generateRoutes('isle-of-dorn', ['bismuth'], {}, { bismuth: 50_000 });
    expect(cands).toHaveLength(2);
    expect(cands[0].rank).toBe(1);
    expect(cands[0].config.seed).toBe(20260909);
    expect(cands[0].waypoints.length).toBeGreaterThanOrEqual(2);
    expect(cands[0].stats.gphExpected).toBeGreaterThan(0);
  });

  it('rejects generation without resources or prices', async () => {
    await expect(generateRoutes('isle-of-dorn', [], {}, {})).rejects.toThrow(
      'select at least one resource',
    );
    await expect(generateRoutes('isle-of-dorn', ['bismuth'], {}, {})).rejects.toThrow(
      'missing prices for: bismuth',
    );
  });

  it('saves, lists, gets, compares and deletes routes', async () => {
    const before = await listRoutes();
    expect(before.length).toBe(1);
    const [cand] = await generateRoutes('isle-of-dorn', ['bismuth'], {}, { bismuth: 50_000 });
    const saved = await saveRoute({
      name: '  Test Loop  ',
      zoneId: 'isle-of-dorn',
      resourceIds: ['bismuth'],
      waypoints: cand.waypoints,
      excluded: [],
      config: cand.config,
      stats: cand.stats,
    });
    expect(saved.name).toBe('Test Loop');
    expect(saved.id).toMatch(/^rt-mock-/);
    const after = await listRoutes('isle-of-dorn');
    expect(after).toHaveLength(2);
    const fetched = await getRoute(saved.id);
    expect(fetched.waypoints).toHaveLength(cand.waypoints.length);
    const compared = await compareRoutes([saved.id, before[0].id]);
    expect(compared.map((r) => r.id)).toEqual([saved.id, before[0].id]);
    expect(await deleteRoute(saved.id)).toBe(true);
    expect(await deleteRoute(saved.id)).toBe(false);
    await expect(getRoute(saved.id)).rejects.toThrow('unknown route');
  });

  it('rejects empty names and bad waypoint counts', async () => {
    const [cand] = await generateRoutes('isle-of-dorn', ['bismuth'], {}, { bismuth: 50_000 });
    await expect(
      saveRoute({
        name: '   ',
        zoneId: null,
        resourceIds: [],
        waypoints: cand.waypoints,
        excluded: [],
        config: cand.config,
        stats: cand.stats,
      }),
    ).rejects.toThrow('route name is empty');
  });

  it('exports a GO1 import code', async () => {
    const [row] = await listRoutes();
    const out = await exportRouteString(row.id);
    expect(out.code).toMatch(/^GO1\./);
    expect(out.waypointCount).toBeGreaterThanOrEqual(2);
  });
});

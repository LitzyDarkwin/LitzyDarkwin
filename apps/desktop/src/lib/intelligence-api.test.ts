import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import { getPersonalProfile, rebuildPersonal } from './intelligence-api';
import { generatePersonalizedRoutes } from './routes-api';

describe('intelligence-api (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('reads the personal profile with metrics, zones and resources', async () => {
    const profile = await getPersonalProfile();
    const keys = Object.fromEntries(profile.metrics.map((m) => [m.key, m.value]));
    expect(keys['sessions_analyzed']).toBe(12);
    expect(keys['efficiency_global']).toBe(0.85);
    expect(profile.zones).toHaveLength(1);
    expect(profile.zones[0].zoneName).toBe('Isle of Dorn');
    expect(profile.zones[0].efficiency).toBe(0.85);
    expect(profile.resources.map((r) => r.resourceId)).toEqual(['bismuth', 'mycobloom']);
    expect(profile.resources[0].avgYield).toBe(2.5);
  });

  it('rebuilds personal stats and reports counts', async () => {
    const report = await rebuildPersonal();
    expect(report).toEqual({ sessions: 12, resources: 2, zones: 1 });
  });

  it('generates personalized candidates with scaled effective values', async () => {
    const cands = await generatePersonalizedRoutes(
      'isle-of-dorn',
      ['bismuth'],
      {},
      { bismuth: 50_000 },
    );
    expect(cands.length).toBeGreaterThan(0);
    const [first] = cands;
    expect(first.personal).not.toBeNull();
    expect(first.personal?.efficiency).toBe(0.85);
    // Gross is the unadjusted model reference; effective scales by efficiency.
    expect(first.stats.grossValue).toBe(1_900_000);
    expect(first.stats.effectiveValue).toBe(Math.round(1_520_000 * 0.85));
    expect(first.stats.gphExpected).toBe(Math.round(6_080_000 * 0.85));
    expect(first.personal?.explain.length).toBeGreaterThan(0);
  });

  it('validates personalized generation like the plain one', async () => {
    await expect(generatePersonalizedRoutes('isle-of-dorn', [], {}, {})).rejects.toThrow(
      'select at least one resource',
    );
    await expect(
      generatePersonalizedRoutes('isle-of-dorn', ['bismuth'], {}, {}),
    ).rejects.toThrow('missing prices for: bismuth');
  });
});

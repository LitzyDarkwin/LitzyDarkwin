import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import { getAtlasTree, getZoneDetail, getZoneObservations, searchResources } from './atlas-api';

describe('atlas-api (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('loads the zone tree', async () => {
    const tree = await getAtlasTree();
    expect(tree.expansions.length).toBe(2);
    const zones = tree.expansions.flatMap((e) => e.continents.flatMap((c) => c.zones));
    expect(zones.map((z) => z.id)).toContain('isle-of-dorn');
  });

  it('searches with category filter', async () => {
    const ores = await searchResources('ore', 'mining');
    expect(ores.length).toBeGreaterThan(0);
    expect(ores.every((r) => r.categoryId === 'mining')).toBe(true);
    const all = await searchResources('', undefined);
    expect(all.length).toBeGreaterThanOrEqual(ores.length);
  });

  it('loads zone detail or throws for unknown zones', async () => {
    const d = await getZoneDetail('isle-of-dorn');
    expect(d.name).toBe('Isle of Dorn');
    expect(d.resources.length).toBe(3);
    await expect(getZoneDetail('nope')).rejects.toThrow('Unknown zone');
  });

  it('loads deterministic observation grids', async () => {
    const a = await getZoneObservations('isle-of-dorn', [], 20);
    const b = await getZoneObservations('isle-of-dorn', [], 20);
    expect(a.cells.length).toBe(b.cells.length);
    expect(a.totalPoints).toBe(b.totalPoints);
    const filtered = await getZoneObservations('isle-of-dorn', ['mycobloom'], 20);
    expect(filtered.cells.every((c) => c.byResource.every((r) => r.resourceId === 'mycobloom'))).toBe(true);
  });
});

import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import {
  getItemValuation,
  getMarketPrices,
  getMarketStatus,
  refreshMarket,
  searchMarketItems,
} from './market-api';

describe('market-api (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('reports market status with dataset health', async () => {
    const status = await getMarketStatus();
    expect(status.provider).toBe('tsm');
    expect(status.region).toBe('eu');
    expect(status.itemCount).toBe(5);
    expect(status.datasets.map((d) => d.dataset)).toEqual(['commodities', 'region-items']);
    expect(status.freshness).toBe('fresh');
  });

  it('searches items by name or id', async () => {
    const bloom = await searchMarketItems('bloom');
    expect(bloom.map((r) => r.itemId)).toEqual([161504]);
    const byId = await searchMarketItems('161502');
    expect(byId.map((r) => r.itemName)).toEqual(['Bismuth']);
    expect(await searchMarketItems('zzz-no-such-item')).toEqual([]);
  });

  it('valuates items with an explainable breakdown', async () => {
    const v = await getItemValuation(161502);
    expect(v.effectiveValue).toBe(46_912);
    expect(v.flags).toEqual([]);
    expect(v.explain.length).toBeGreaterThan(0);
    const thin = await getItemValuation(161505);
    expect(thin.flags).toContain('live-unknown');
    const missing = await getItemValuation(999_999);
    expect(missing.flags).toEqual(['no-data']);
    expect(missing.effectiveValue).toBe(0);
  });

  it('refreshes with per-dataset reports', async () => {
    const summary = await refreshMarket();
    expect(summary.region).toBe('eu');
    expect(summary.commodities?.rowsUpserted).toBe(5);
    expect(summary.commodities?.rowsSkipped).toBe(2);
    expect(summary.errors).toEqual([]);
  });

  it('resolves resource prices with explicit missing entries', async () => {
    const out = await getMarketPrices(['bismuth', 'mycobloom', 'serevite-ore']);
    expect(out.prices).toEqual({ bismuth: 46_912, mycobloom: 105_710 });
    expect(out.missing).toHaveLength(1);
    expect(out.missing[0]).toContain('serevite-ore');
  });
});

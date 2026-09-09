/**
 * Canned market responses for the mock backend. Item ids/values mirror
 * fixtures/tsm_*_sample.csv and fixtures/expected/valuation.json so the
 * Market page preview stays coherent with the Rust fixtures.
 */
import type {
  ItemValuation,
  MarketItemRow,
  MarketPrices,
  MarketStatus,
  RefreshSummary,
} from '../types/domain';

const NOW = 1_788_872_400; // 2026-09-08T13:00:00Z, one hour after the snapshot

export const MOCK_MARKET_STATUS: MarketStatus = {
  provider: 'tsm',
  region: 'eu',
  itemCount: 5,
  freshness: 'fresh',
  datasets: [
    {
      dataset: 'commodities',
      updatedAt: 1_788_868_800,
      fetchedAt: NOW,
      freshness: 'fresh',
      failures: 0,
      lastError: null,
    },
    {
      dataset: 'region-items',
      updatedAt: 1_788_825_600,
      fetchedAt: NOW,
      freshness: 'fresh',
      failures: 0,
      lastError: null,
    },
  ],
};

export const MOCK_MARKET_ITEMS: MarketItemRow[] = [
  { itemId: 161502, itemName: 'Bismuth', marketValue: 52_000, effectiveValue: 46_912, confidence: 0.91, freshness: 'fresh' },
  { itemId: 161503, itemName: 'Ironclaw, Ore', marketValue: 48_000, effectiveValue: 43_945, confidence: 0.88, freshness: 'fresh' },
  { itemId: 161504, itemName: 'Mycobloom', marketValue: 125_000, effectiveValue: 105_710, confidence: 0.93, freshness: 'fresh' },
  { itemId: 161505, itemName: 'Luredrop', marketValue: 0, effectiveValue: 83_260, confidence: 0.66, freshness: 'fresh' },
  { itemId: 161507, itemName: 'Hochenblume', marketValue: 90_000, effectiveValue: 79_200, confidence: 0.69, freshness: 'fresh' },
];

const EXPLAIN: Record<number, { conservative: number; explain: string[]; flags: string[] }> = {
  161502: {
    conservative: 46_912,
    flags: [],
    explain: [
      'Live market value 52000 within baseline band.',
      'Conservative = min basket (avg-sale) × 94% liquidity haircut = 46912.',
      'Effective = conservative × 100% freshness = 46912.',
    ],
  },
  161505: {
    conservative: 83_260,
    flags: ['live-unknown', 'thin'],
    explain: [
      'No live quote; valuing from history.',
      'Conservative = min basket (avg-sale) × 90% liquidity haircut = 83260.',
      'Effective = conservative × 100% freshness = 83260.',
    ],
  },
};

export function mockValuation(itemId: number): ItemValuation {
  const row = MOCK_MARKET_ITEMS.find((r) => r.itemId === itemId);
  if (!row) {
    return {
      itemId,
      marketValue: 0,
      conservativeValue: 0,
      effectiveValue: 0,
      confidence: 0,
      freshness: 'unknown',
      flags: ['no-data'],
      explain: ['No market data cached for this item yet.'],
    };
  }
  const extra = EXPLAIN[itemId] ?? {
    conservative: row.effectiveValue,
    flags: [],
    explain: [`Valued at ${row.effectiveValue} copper per unit.`],
  };
  return {
    itemId,
    marketValue: row.marketValue,
    conservativeValue: extra.conservative,
    effectiveValue: row.effectiveValue,
    confidence: row.confidence,
    freshness: row.freshness,
    flags: extra.flags,
    explain: extra.explain,
  };
}

export function mockSearchItems(query: string, limit: number): MarketItemRow[] {
  const q = query.trim().toLowerCase();
  const id = Number(q);
  return MOCK_MARKET_ITEMS.filter(
    (r) => r.itemName.toLowerCase().includes(q) || (Number.isInteger(id) && r.itemId === id),
  )
    .sort((a, b) => b.effectiveValue - a.effectiveValue)
    .slice(0, Math.max(1, Math.min(limit, 100)));
}

export function mockRefresh(): RefreshSummary {
  return {
    region: 'eu',
    commodities: {
      dataset: 'commodities',
      rowsTotal: 7,
      rowsUpserted: 5,
      rowsSkipped: 2,
      snapshotUpdatedAt: 1_788_868_800,
    },
    regionItems: {
      dataset: 'region-items',
      rowsTotal: 4,
      rowsUpserted: 4,
      rowsSkipped: 0,
      snapshotUpdatedAt: 1_788_825_600,
    },
    errors: [],
  };
}

/** Resource → effective copper/unit, mirroring resolve_prices over the samples. */
const RESOURCE_PRICES: Record<string, number> = {
  bismuth: 46_912,
  'ironclaw-ore': 43_945,
  mycobloom: 105_710,
  luredrop: 83_260,
};

export function mockMarketPrices(resourceIds: string[]): MarketPrices {
  const prices: Record<string, number> = {};
  const missing: string[] = [];
  for (const id of resourceIds) {
    if (id in RESOURCE_PRICES) prices[id] = RESOURCE_PRICES[id];
    else missing.push(`${id}: no market data (refresh market)`);
  }
  return { prices, missing };
}

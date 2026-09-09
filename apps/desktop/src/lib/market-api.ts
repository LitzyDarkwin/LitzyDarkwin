import { invokeSafe } from './tauri';
import type {
  ItemValuation,
  MarketItemRow,
  MarketPrices,
  MarketStatus,
  RefreshSummary,
} from '../types/domain';

export function refreshMarket(region?: string): Promise<RefreshSummary> {
  return invokeSafe<RefreshSummary>('refresh_market', { region: region ?? null });
}

export function getMarketStatus(region?: string): Promise<MarketStatus> {
  return invokeSafe<MarketStatus>('get_market_status', { region: region ?? null });
}

export function getItemValuation(itemId: number): Promise<ItemValuation> {
  return invokeSafe<ItemValuation>('get_item_valuation', { itemId });
}

export function searchMarketItems(query: string, limit = 25): Promise<MarketItemRow[]> {
  return invokeSafe<MarketItemRow[]>('search_market_items', { query, limit });
}

export function getMarketPrices(resourceIds: string[]): Promise<MarketPrices> {
  return invokeSafe<MarketPrices>('get_market_prices', { resourceIds });
}

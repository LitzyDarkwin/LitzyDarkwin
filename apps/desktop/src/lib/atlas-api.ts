import { invokeSafe } from './tauri';
import type {
  AtlasTree,
  ClusteredObservations,
  ResourceHit,
  ZoneDetail,
} from '../types/domain';

export function getAtlasTree(): Promise<AtlasTree> {
  return invokeSafe<AtlasTree>('get_atlas_tree');
}

export function searchResources(
  query: string,
  category?: string,
  limit = 25,
): Promise<ResourceHit[]> {
  return invokeSafe<ResourceHit[]>('search_resources', { query, category: category ?? null, limit });
}

export function getZoneDetail(zoneId: string): Promise<ZoneDetail> {
  return invokeSafe<ZoneDetail>('get_zone_detail', { zoneId });
}

export function getZoneObservations(
  zoneId: string,
  resourceIds: string[] = [],
  grid = 20,
): Promise<ClusteredObservations> {
  return invokeSafe<ClusteredObservations>('get_zone_observations', {
    zoneId,
    resourceIds,
    grid,
  });
}

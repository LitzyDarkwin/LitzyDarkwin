import { invokeSafe } from './tauri';
import type {
  ExcludedArea,
  RouteCandidate,
  RouteConfigDto,
  RouteExport,
  RouteSummary,
  SaveRouteInput,
  SavedRoute,
} from '../types/domain';

export function generateRoutes(
  zoneId: string,
  resourceIds: string[],
  config: RouteConfigDto = {},
  prices: Record<string, number> = {},
  excluded: ExcludedArea[] = [],
): Promise<RouteCandidate[]> {
  return invokeSafe<RouteCandidate[]>('generate_routes', {
    zoneId,
    resourceIds,
    config,
    prices,
    excluded,
  });
}

export function generatePersonalizedRoutes(
  zoneId: string,
  resourceIds: string[],
  config: RouteConfigDto = {},
  prices: Record<string, number> = {},
  excluded: ExcludedArea[] = [],
): Promise<RouteCandidate[]> {
  return invokeSafe<RouteCandidate[]>('generate_personalized_routes', {
    zoneId,
    resourceIds,
    config,
    prices,
    excluded,
  });
}

export function saveRoute(input: SaveRouteInput): Promise<SavedRoute> {
  return invokeSafe<SavedRoute>('save_route', { input });
}

export function listRoutes(zoneId?: string): Promise<RouteSummary[]> {
  return invokeSafe<RouteSummary[]>('list_routes', { zoneId: zoneId ?? null });
}

export function getRoute(id: string): Promise<SavedRoute> {
  return invokeSafe<SavedRoute>('get_route', { id });
}

export function deleteRoute(id: string): Promise<boolean> {
  return invokeSafe<boolean>('delete_route', { id });
}

export function compareRoutes(ids: string[]): Promise<SavedRoute[]> {
  return invokeSafe<SavedRoute[]>('compare_routes', { ids });
}

export function exportRouteString(id: string): Promise<RouteExport> {
  return invokeSafe<RouteExport>('export_route_string', { id });
}

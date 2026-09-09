import { invokeSafe } from './tauri';
import type { SessionDetail, SessionStats, SessionSummary } from '../types/domain';

export function listSessions(zoneId?: string, limit = 50): Promise<SessionSummary[]> {
  return invokeSafe<SessionSummary[]>('list_sessions', { zoneId: zoneId ?? null, limit });
}

export function getSession(id: string): Promise<SessionDetail> {
  return invokeSafe<SessionDetail>('get_session', { id });
}

export function linkSessionRoute(id: string, routeId: string | null): Promise<SessionDetail> {
  return invokeSafe<SessionDetail>('link_session_route', { id, routeId: routeId ?? null });
}

export function analyzeSession(id: string): Promise<SessionDetail> {
  return invokeSafe<SessionDetail>('analyze_session', { id });
}

export function getSessionStats(zoneId?: string, days?: number): Promise<SessionStats> {
  return invokeSafe<SessionStats>('get_session_stats', {
    zoneId: zoneId ?? null,
    days: days ?? null,
  });
}

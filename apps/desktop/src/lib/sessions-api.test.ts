import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import {
  analyzeSession,
  getSession,
  getSessionStats,
  linkSessionRoute,
  listSessions,
} from './sessions-api';

describe('sessions-api (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('lists sessions newest first with zone filter', async () => {
    const all = await listSessions();
    expect(all.map((s) => s.id)).toEqual(['s2', 's1']);
    const zoned = await listSessions('isle-of-dorn');
    expect(zoned.map((s) => s.id)).toEqual(['s1']);
  });

  it('reads session detail with then/now totals', async () => {
    const detail = await getSession('s1');
    expect(detail.summary.valueThen).toBe(997_670);
    expect(detail.summary.valueNow).toBe(997_670);
    expect(detail.summary.gphThen).toBe(997_670);
    expect(detail.items).toHaveLength(2);
    expect(detail.expectedTotal).toBeNull();
    await expect(getSession('nope')).rejects.toThrow('unknown session');
  });

  it('analyzes imported sessions by snapshotting prices', async () => {
    const before = await getSession('s2');
    expect(before.summary.status).toBe('imported');
    expect(before.summary.thenPartial).toBe(true);
    const after = await analyzeSession('s2');
    expect(after.summary.status).toBe('analyzed');
    expect(after.items[0].unitThen).toBe(46_912);
    expect(after.summary.thenPartial).toBe(false);
    expect(after.summary.valueThen).toBe(93_824);
  });

  it('links and unlinks route expectations', async () => {
    const linked = await linkSessionRoute('s1', 'rt-mock-1');
    expect(linked.summary.expectedGph).toBe(6_080_000);
    expect(linked.expectedTotal).toBe(6_080_000);
    expect(linked.deltaPctThen).toBeCloseTo(-83.59, 1);
    expect(linked.routeName).toContain('Dorn');
    const unlinked = await linkSessionRoute('s1', null);
    expect(unlinked.summary.expectedGph).toBeNull();
    expect(unlinked.expectedTotal).toBeNull();
    await expect(linkSessionRoute('s1', 'rt-nope')).rejects.toThrow('unknown route');
  });

  it('aggregates stats by day and resource', async () => {
    const stats = await getSessionStats();
    expect(stats.sessionCount).toBe(2);
    expect(stats.activeSeconds).toBe(5400);
    expect(stats.byDay.map((d) => d.day)).toEqual(['2026-09-08', '2026-09-07']);
    const bismuth = stats.byResource.find((b) => b.key === 'bismuth');
    expect(bismuth?.qty).toBe(12);
    expect(bismuth?.displayName).toBe('Bismuth');
  });
});

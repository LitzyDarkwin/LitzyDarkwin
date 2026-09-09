/**
 * Canned session analytics for the mock backend. Shapes mirror the Rust
 * sessions fixtures (s1/s2) with mock-atlas zones; `refreshDetail` keeps
 * derived totals coherent after mock mutations (analyze/link).
 */
import type {
  SessionDetail,
  SessionItemVal,
  SessionStats,
  SessionSummary,
} from '../types/domain';

function item(
  itemId: number,
  resourceId: string | null,
  displayName: string,
  qty: number,
  unitThen: number | null,
  unitNow: number | null,
): SessionItemVal {
  return {
    itemId,
    resourceId,
    displayName,
    qty,
    unitThen,
    unitNow,
    valueThen: unitThen == null ? null : Math.round(qty * unitThen),
    valueNow: unitNow == null ? null : Math.round(qty * unitNow),
  };
}

/** Recompute summary totals + expectation deltas from items + link. */
export function refreshDetail(detail: SessionDetail): SessionDetail {
  const s = detail.summary;
  let qty = 0;
  let then = 0;
  let now = 0;
  let thenPartial = false;
  let nowPartial = false;
  for (const it of detail.items) {
    qty += it.qty;
    if (it.valueThen == null) thenPartial = true;
    else then += it.valueThen;
    if (it.valueNow == null) nowPartial = true;
    else now += it.valueNow;
  }
  const hours = s.activeSeconds > 0 ? s.activeSeconds / 3600 : 0;
  const expectedTotal =
    s.expectedGph != null && hours > 0 ? Math.round(s.expectedGph * hours) : null;
  const delta = (actual: number) =>
    expectedTotal != null && expectedTotal > 0
      ? ((actual - expectedTotal) / expectedTotal) * 100
      : null;
  detail.summary = {
    ...s,
    itemCount: detail.items.length,
    totalQty: qty,
    valueThen: then,
    valueNow: now,
    thenPartial,
    nowPartial,
    gphThen: hours > 0 ? then / hours : null,
    gphNow: hours > 0 ? now / hours : null,
  };
  detail.expectedTotal = expectedTotal;
  detail.deltaPctThen = delta(then);
  detail.deltaPctNow = delta(now);
  return detail;
}

function makeDetail(
  summary: Omit<SessionSummary, 'itemCount' | 'totalQty' | 'valueThen' | 'valueNow' | 'thenPartial' | 'nowPartial' | 'gphThen' | 'gphNow'>,
  notes: string,
  items: SessionItemVal[],
): SessionDetail {
  return refreshDetail({
    summary: {
      ...summary,
      itemCount: 0,
      totalQty: 0,
      valueThen: 0,
      valueNow: 0,
      thenPartial: false,
      nowPartial: false,
      gphThen: null,
      gphNow: null,
    },
    notes,
    routeName: null,
    expectedValuePerLoop: null,
    items,
    expectedTotal: null,
    deltaPctThen: null,
    deltaPctNow: null,
  });
}

export function mockSessionDetails(): SessionDetail[] {
  return [
    makeDetail(
      {
        id: 's1',
        character: 'Testtoon',
        realm: 'Realm',
        zoneId: 'isle-of-dorn',
        zoneName: 'Isle of Dorn',
        routeId: null,
        status: 'analyzed',
        startedAt: 1_788_778_800,
        endedAt: 1_788_786_000,
        activeSeconds: 3600,
        expectedGph: null,
      },
      'Evening farm',
      [
        item(161502, 'bismuth', 'Bismuth', 10, 46_912, 46_912),
        item(161504, 'mycobloom', 'Mycobloom', 5, 105_710, 105_710),
      ],
    ),
    makeDetail(
      {
        id: 's2',
        character: 'Testtoon',
        realm: 'Realm',
        zoneId: 'ringing-deeps',
        zoneName: 'The Ringing Deeps',
        routeId: null,
        status: 'imported',
        startedAt: 1_788_868_800,
        endedAt: 1_788_870_600,
        activeSeconds: 1800,
        expectedGph: null,
      },
      '',
      [item(161502, 'bismuth', 'Bismuth', 2, null, 46_912)],
    ),
  ];
}

export function toSummary(detail: SessionDetail): SessionSummary {
  return { ...detail.summary };
}

/** Aggregate mock details into stats (mirrors session_stats shape). */
export function mockSessionStats(details: SessionDetail[]): SessionStats {
  const byDay = new Map<string, { sessions: number; seconds: number; then: number; now: number }>();
  const byRes = new Map<string, { name: string; qty: number; then: number; now: number }>();
  let seconds = 0;
  let then = 0;
  let now = 0;
  let thenPartial = false;
  let nowPartial = false;
  for (const d of details) {
    const s = d.summary;
    seconds += s.activeSeconds;
    then += s.valueThen;
    now += s.valueNow;
    thenPartial ||= s.thenPartial;
    nowPartial ||= s.nowPartial;
    const day = new Date(s.startedAt * 1000).toISOString().slice(0, 10);
    const acc = byDay.get(day) ?? { sessions: 0, seconds: 0, then: 0, now: 0 };
    acc.sessions += 1;
    acc.seconds += s.activeSeconds;
    acc.then += s.valueThen;
    acc.now += s.valueNow;
    byDay.set(day, acc);
    for (const it of d.items) {
      const key = it.resourceId ?? `item:${it.itemId}`;
      const racc = byRes.get(key) ?? { name: it.displayName, qty: 0, then: 0, now: 0 };
      racc.qty += it.qty;
      racc.then += it.valueThen ?? 0;
      racc.now += it.valueNow ?? 0;
      byRes.set(key, racc);
    }
  }
  return {
    sessionCount: details.length,
    activeSeconds: seconds,
    valueThen: then,
    valueNow: now,
    thenPartial,
    nowPartial,
    byDay: [...byDay.entries()]
      .map(([day, a]) => ({
        day,
        sessions: a.sessions,
        activeSeconds: a.seconds,
        valueThen: a.then,
        valueNow: a.now,
      }))
      .sort((a, b) => (b.day < a.day ? -1 : 1)),
    byResource: [...byRes.entries()]
      .map(([key, a]) => ({
        key,
        displayName: a.name,
        qty: a.qty,
        valueThen: a.then,
        valueNow: a.now,
      }))
      .sort((a, b) => b.valueNow - a.valueNow),
  };
}

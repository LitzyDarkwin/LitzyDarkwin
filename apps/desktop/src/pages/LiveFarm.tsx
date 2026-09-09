import { useCallback, useEffect, useState } from 'react';
import { EmptyState, ErrorState } from '../components/States';
import { getAtlasTree } from '../lib/atlas-api';
import {
  formatAgo,
  formatDuration,
  formatGold,
  formatInt,
  formatQty,
  formatTime,
} from '../lib/format';
import { listRoutes } from '../lib/routes-api';
import {
  analyzeSession,
  getSession,
  getSessionStats,
  linkSessionRoute,
  listSessions,
} from '../lib/sessions-api';
import { getSyncState, syncAddon } from '../lib/sync-api';
import { toMessage } from '../lib/tauri';
import type {
  Freshness,
  RouteSummary,
  SessionDetail,
  SessionStats,
  SessionSummary,
  SyncStateInfo,
} from '../types/domain';

function FreshBadge({ value }: { value: Freshness }) {
  return <span className={`badge badge-${value}`}>{value}</span>;
}

function Delta({ value }: { value: number | null }) {
  if (value == null) return <span>—</span>;
  const cls = value >= 0 ? 'delta-plus' : 'delta-minus';
  const sign = value >= 0 ? '+' : '';
  return <span className={cls}>{`${sign}${value.toFixed(1)}%`}</span>;
}

function Gph({ then, now }: { then: number | null; now: number | null }) {
  return (
    <span>
      {then != null ? `${formatGold(then)}/h` : '—'}
      {' → '}
      {now != null ? `${formatGold(now)}/h` : '—'}
    </span>
  );
}

export default function LiveFarm() {
  const [sync, setSync] = useState<SyncStateInfo | null>(null);
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);

  const [zones, setZones] = useState<{ id: string; name: string }[]>([]);
  const [zoneFilter, setZoneFilter] = useState('');

  const [stats, setStats] = useState<SessionStats | null>(null);
  const [history, setHistory] = useState<SessionSummary[]>([]);
  const [routes, setRoutes] = useState<RouteSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<SessionDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    const [syncState, sessionStats, sessions, savedRoutes] = await Promise.all([
      getSyncState(),
      getSessionStats(),
      listSessions(),
      listRoutes(),
    ]);
    setSync(syncState);
    setStats(sessionStats);
    setHistory(sessions);
    setRoutes(savedRoutes);
  }, []);

  useEffect(() => {
    reload().catch((e: unknown) => setError(toMessage(e)));
    getAtlasTree()
      .then((tree) => {
        const all: { id: string; name: string }[] = [];
        for (const exp of tree.expansions) {
          for (const cont of exp.continents) {
            for (const z of cont.zones) all.push({ id: z.id, name: z.name });
          }
        }
        setZones(all);
      })
      .catch((e: unknown) => setError(toMessage(e)));
  }, [reload]);

  useEffect(() => {
    listSessions(zoneFilter || undefined)
      .then(setHistory)
      .catch((e: unknown) => setError(toMessage(e)));
  }, [zoneFilter]);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      return;
    }
    let live = true;
    getSession(selectedId)
      .then((d) => live && setDetail(d))
      .catch((e: unknown) => live && setError(toMessage(e)));
    return () => {
      live = false;
    };
  }, [selectedId]);

  const onSync = async () => {
    setSyncing(true);
    setSyncMsg(null);
    try {
      const report = await syncAddon();
      setSyncMsg(
        `Synced ${report.sessionsImported} new, ${report.sessionsUpdated} updated, ` +
          `${report.observationsImported} observations.`,
      );
      await reload();
      if (selectedId) setDetail(await getSession(selectedId));
    } catch (e: unknown) {
      setSyncMsg(toMessage(e));
    } finally {
      setSyncing(false);
    }
  };

  const onLink = async (routeId: string) => {
    if (!selectedId) return;
    try {
      const updated = await linkSessionRoute(selectedId, routeId || null);
      setDetail(updated);
      setHistory((prev) =>
        prev.map((s) => (s.id === selectedId ? updated.summary : s)),
      );
    } catch (e: unknown) {
      setError(toMessage(e));
    }
  };

  const onAnalyze = async () => {
    if (!selectedId) return;
    try {
      const updated = await analyzeSession(selectedId);
      setDetail(updated);
      const sessionStats = await getSessionStats();
      setStats(sessionStats);
    } catch (e: unknown) {
      setError(toMessage(e));
    }
  };

  return (
    <div className="page">
      <header className="page-head">
        <h1>Live Farm</h1>
        <p className="page-sub">
          Synced addon sessions, valued then and now, against route expectations.
        </p>
      </header>
      {error && <ErrorState message={error} />}

      <div className="grid-2">
        <section className="card" aria-label="Sync">
          <h2>Addon sync</h2>
          {sync ? (
            <div data-testid="sync-state">
              <div className="stat-grid">
                <div className="stat">
                  <div className="stat-label">Freshness</div>
                  <div className="stat-value">
                    <FreshBadge value={sync.freshness} />
                  </div>
                </div>
                <div className="stat">
                  <div className="stat-label">Last sync</div>
                  <div className="stat-value">{formatAgo(sync.lastSyncAt)}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">Last export</div>
                  <div className="stat-value">{formatAgo(sync.lastExportAt)}</div>
                </div>
              </div>
              <div className="hint">
                WoW flushes SavedVariables on logout/reload — sync after playing.
              </div>
              <div className="btn-row">
                <button
                  type="button"
                  className="btn btn-primary"
                  data-testid="sync-btn"
                  disabled={syncing}
                  onClick={onSync}
                >
                  {syncing ? 'Syncing…' : 'Sync now'}
                </button>
              </div>
              {syncMsg && (
                <div className="hint" data-testid="sync-msg">
                  {syncMsg}
                </div>
              )}
            </div>
          ) : (
            <div className="hint">Loading sync state…</div>
          )}
        </section>

        <section className="card" aria-label="Totals">
          <h2>Totals</h2>
          {!stats && <div className="hint">Loading…</div>}
          {stats && (
            <div data-testid="stats-card">
              <div className="stat-grid">
                <div className="stat">
                  <div className="stat-label">Sessions</div>
                  <div className="stat-value">{formatInt(stats.sessionCount)}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">Active time</div>
                  <div className="stat-value">{formatDuration(stats.activeSeconds)}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">Value then{stats.thenPartial ? ' *' : ''}</div>
                  <div className="stat-value">{formatGold(stats.valueThen)}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">Value now{stats.nowPartial ? ' *' : ''}</div>
                  <div className="stat-value">{formatGold(stats.valueNow)}</div>
                </div>
              </div>
              {(stats.thenPartial || stats.nowPartial) && (
                <div className="hint">* partial: some loot lacks a price.</div>
              )}
              {stats.byDay.length > 0 && (
                <div className="table-wrap">
                  <table className="data" data-testid="stats-by-day">
                    <thead>
                      <tr>
                        <th>Day</th>
                        <th>Sessions</th>
                        <th>Time</th>
                        <th>Value now</th>
                      </tr>
                    </thead>
                    <tbody>
                      {stats.byDay.map((d) => (
                        <tr key={d.day}>
                          <td>{d.day}</td>
                          <td>{d.sessions}</td>
                          <td>{formatDuration(d.activeSeconds)}</td>
                          <td>{formatGold(d.valueNow)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
              {stats.byResource.length > 0 && (
                <div className="table-wrap">
                  <table className="data" data-testid="stats-by-resource">
                    <thead>
                      <tr>
                        <th>Resource</th>
                        <th>Qty</th>
                        <th>Value now</th>
                      </tr>
                    </thead>
                    <tbody>
                      {stats.byResource.map((r) => (
                        <tr key={r.key}>
                          <td>{r.displayName}</td>
                          <td>{formatQty(r.qty)}</td>
                          <td>{formatGold(r.valueNow)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          )}
        </section>
      </div>

      <section className="card" aria-label="History">
        <h2>History</h2>
        <div className="field">
          <label htmlFor="history-zone">Zone filter</label>
          <select
            id="history-zone"
            data-testid="zone-filter"
            value={zoneFilter}
            onChange={(e) => setZoneFilter(e.target.value)}
          >
            <option value="">All zones</option>
            {zones.map((z) => (
              <option key={z.id} value={z.id}>
                {z.name}
              </option>
            ))}
          </select>
        </div>
        {history.length === 0 && (
          <EmptyState title="No sessions yet" hint="Sync the addon after a farm run." />
        )}
        {history.length > 0 && (
          <div className="table-wrap">
            <table className="data" data-testid="history-table">
              <thead>
                <tr>
                  <th>Started</th>
                  <th>Character</th>
                  <th>Zone</th>
                  <th>Time</th>
                  <th>Qty</th>
                  <th>Value then → now</th>
                  <th>GPH then → now</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {history.map((s) => (
                  <tr
                    key={s.id}
                    data-testid={`session-${s.id}`}
                    onClick={() => setSelectedId(s.id)}
                    className={s.id === selectedId ? 'selected' : undefined}
                    style={{ cursor: 'pointer' }}
                  >
                    <td>{formatTime(s.startedAt)}</td>
                    <td>{s.character || '—'}</td>
                    <td>{s.zoneName ?? '—'}</td>
                    <td>{formatDuration(s.activeSeconds)}</td>
                    <td>{formatQty(s.totalQty)}</td>
                    <td>
                      {formatGold(s.valueThen)}
                      {s.thenPartial ? '*' : ''} → {formatGold(s.valueNow)}
                      {s.nowPartial ? '*' : ''}
                    </td>
                    <td>
                      <Gph then={s.gphThen} now={s.gphNow} />
                    </td>
                    <td>{s.status}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      {detail && (
        <section className="card" aria-label="Session detail">
          <h2>
            Session {detail.summary.id} — {formatTime(detail.summary.startedAt)}
          </h2>
          <div data-testid="session-detail">
            {detail.notes && <p className="muted">{detail.notes}</p>}
            <div className="table-wrap">
              <table className="data" data-testid="items-table">
                <thead>
                  <tr>
                    <th>Item</th>
                    <th>Qty</th>
                    <th>Unit then</th>
                    <th>Unit now</th>
                    <th>Value then</th>
                    <th>Value now</th>
                  </tr>
                </thead>
                <tbody>
                  {detail.items.map((it) => (
                    <tr key={it.itemId} data-testid={`loot-${it.itemId}`}>
                      <td>{it.displayName}</td>
                      <td>{formatQty(it.qty)}</td>
                      <td>{it.unitThen != null ? formatGold(it.unitThen) : '—'}</td>
                      <td>{it.unitNow != null ? formatGold(it.unitNow) : '—'}</td>
                      <td>{it.valueThen != null ? formatGold(it.valueThen) : '—'}</td>
                      <td>{it.valueNow != null ? formatGold(it.valueNow) : '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="grid-2">
              <div className="field">
                <label htmlFor="link-route">Linked route (expectations)</label>
                <select
                  id="link-route"
                  data-testid="link-route-select"
                  value={detail.summary.routeId ?? ''}
                  onChange={(e) => onLink(e.target.value)}
                >
                  <option value="">No route</option>
                  {routes.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.name} ({formatGold(r.gphExpected)}/h)
                    </option>
                  ))}
                </select>
              </div>
              <div data-testid="expected-panel">
                {detail.expectedTotal != null ? (
                  <div className="stat-grid">
                    <div className="stat">
                      <div className="stat-label">Expected total</div>
                      <div className="stat-value">{formatGold(detail.expectedTotal)}</div>
                    </div>
                    <div className="stat">
                      <div className="stat-label">Δ then</div>
                      <div className="stat-value">
                        <Delta value={detail.deltaPctThen} />
                      </div>
                    </div>
                    <div className="stat">
                      <div className="stat-label">Δ now</div>
                      <div className="stat-value">
                        <Delta value={detail.deltaPctNow} />
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="hint">Link a route to compare expected vs realized.</div>
                )}
              </div>
            </div>
            {detail.summary.status === 'imported' && (
              <div className="btn-row">
                <button
                  type="button"
                  className="btn btn-secondary"
                  data-testid="analyze-btn"
                  onClick={onAnalyze}
                >
                  Analyze (snapshot prices)
                </button>
              </div>
            )}
          </div>
        </section>
      )}
    </div>
  );
}

import { useCallback, useEffect, useState } from 'react';
import { EmptyState, ErrorState } from '../components/States';
import {
  formatDuration,
  formatGold,
  formatInt,
  formatQty,
  formatTime,
} from '../lib/format';
import { getPersonalProfile, rebuildPersonal } from '../lib/intelligence-api';
import { toMessage } from '../lib/tauri';
import type { PersonalProfile } from '../types/domain';

function metric(profile: PersonalProfile, key: string): number | null {
  return profile.metrics.find((m) => m.key === key)?.value ?? null;
}

export default function Intelligence() {
  const [profile, setProfile] = useState<PersonalProfile | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [rebuilding, setRebuilding] = useState(false);
  const [rebuildMsg, setRebuildMsg] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setError(null);
    try {
      setProfile(await getPersonalProfile());
    } catch (e: unknown) {
      setError(toMessage(e));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const onRebuild = async () => {
    setRebuilding(true);
    setRebuildMsg(null);
    try {
      const report = await rebuildPersonal();
      setRebuildMsg(
        `Rebuilt from ${formatInt(report.sessions)} sessions, ` +
          `${formatInt(report.resources)} resources, ${formatInt(report.zones)} zones.`,
      );
      await reload();
    } catch (e: unknown) {
      setError(toMessage(e));
    } finally {
      setRebuilding(false);
    }
  };

  const learned = (profile?.zones.length ?? 0) + (profile?.resources.length ?? 0) > 0;

  return (
    <div className="page">
      <header className="page-head">
        <h1>Intelligence</h1>
        <p className="page-sub">
          Your learned farming profile — yields, zone efficiency and prediction accuracy.
        </p>
      </header>

      {error && <ErrorState message={error} onRetry={() => void reload()} />}

      <section className="card" aria-label="Learning summary">
        <h2>Learning</h2>
        {!profile && !error && <p className="muted">Loading profile…</p>}
        {profile && (
          <div className="stat-grid">
            <div className="stat">
              <span className="stat-label">Sessions analyzed</span>
              <span className="stat-value" data-testid="metric-sessions">
                {formatInt(metric(profile, 'sessions_analyzed'))}
              </span>
            </div>
            <div className="stat">
              <span className="stat-label">Global efficiency</span>
              <span className="stat-value" data-testid="metric-efficiency">
                ×{(metric(profile, 'efficiency_global') ?? 1).toFixed(2)}
              </span>
            </div>
            <div className="stat">
              <span className="stat-label">Zones learned</span>
              <span className="stat-value" data-testid="metric-zones">
                {formatInt(profile.zones.length)}
              </span>
            </div>
            <div className="stat">
              <span className="stat-label">Resources learned</span>
              <span className="stat-value" data-testid="metric-resources">
                {formatInt(profile.resources.length)}
              </span>
            </div>
          </div>
        )}
        <div className="btn-row">
          <button
            type="button"
            className="btn btn-secondary"
            data-testid="rebuild-btn"
            disabled={rebuilding}
            onClick={() => void onRebuild()}
          >
            {rebuilding ? 'Rebuilding…' : 'Rebuild from sessions'}
          </button>
        </div>
        {rebuildMsg && (
          <p className="muted" data-testid="rebuild-msg">
            {rebuildMsg}
          </p>
        )}
        <p className="hint">
          Rebuilds are deterministic full passes over analyzed sessions — no drift, and unlinked
          sessions never shape efficiency.
        </p>
      </section>

      {profile && !learned && (
        <div className="card">
          <EmptyState
            title="No personal data yet"
            hint="Sync the addon and analyze sessions to start learning."
          />
        </div>
      )}

      {profile && profile.zones.length > 0 && (
        <section className="card" aria-label="Zone efficiency">
          <h2>Zone efficiency</h2>
          <div className="table-wrap">
          <table className="data" data-testid="zones-table">
            <thead>
              <tr>
                <th>Zone</th>
                <th>Sessions</th>
                <th>Time</th>
                <th>Avg GPH</th>
                <th>Efficiency</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {profile.zones.map((z) => (
                <tr key={z.zoneId} data-testid={`zone-${z.zoneId}`}>
                  <td>{z.zoneName ?? z.zoneId}</td>
                  <td>{formatInt(z.sessions)}</td>
                  <td>{formatDuration(z.totalSeconds)}</td>
                  <td>{formatGold(z.avgGph)}/h</td>
                  <td>×{z.efficiency.toFixed(2)}</td>
                  <td>{formatTime(z.updatedAt)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
        </section>
      )}

      {profile && profile.resources.length > 0 && (
        <section className="card" aria-label="Resource yields">
          <h2>Resource yields</h2>
          <div className="table-wrap">
          <table className="data" data-testid="resources-table">
            <thead>
              <tr>
                <th>Resource</th>
                <th>Gathers</th>
                <th>Total qty</th>
                <th>Avg yield</th>
                <th>Gold/gather</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {profile.resources.map((r) => (
                <tr key={r.resourceId} data-testid={`resource-${r.resourceId}`}>
                  <td>{r.resourceName ?? r.resourceId}</td>
                  <td>{formatInt(r.gathers)}</td>
                  <td>{formatQty(r.totalQty)}</td>
                  <td>{r.avgYield.toFixed(2)}</td>
                  <td>{formatGold(r.avgGoldPerGather)}</td>
                  <td>{formatTime(r.updatedAt)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          </div>
        </section>
      )}
    </div>
  );
}

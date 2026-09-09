import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { formatDuration, formatGold, formatTime } from '../lib/format';
import { listSessions } from '../lib/sessions-api';
import { useAppStore } from '../stores/app';
import type { SessionSummary } from '../types/domain';

export default function Dashboard() {
  const { status, statusError, loading } = useAppStore();
  const [recent, setRecent] = useState<SessionSummary[]>([]);

  useEffect(() => {
    if (!status) return;
    listSessions(undefined, 5).then(setRecent).catch(() => {});
  }, [status]);
  return (
    <div className="page">
      <header className="page-head">
        <h1>Dashboard</h1>
        <p className="page-sub">What to farm, where, and what it is worth — for you, right now.</p>
      </header>
      {loading && <div className="card">Connecting to backend…</div>}
      {statusError && (
        <div className="card card-error" role="alert">
          Backend unreachable: {statusError}
        </div>
      )}
      {status && (
        <div className="grid">
          <section className="card">
            <h2>Recommendation</h2>
            <p className="muted">Best-farm engine arrives in Phase 9. Start with the Atlas.</p>
            <Link className="btn btn-primary" to="/atlas">Open Atlas</Link>
          </section>
          <section className="card">
            <h2>System</h2>
            <dl className="kv">
              <dt>App version</dt>
              <dd>{status.version}</dd>
              <dt>Schema</dt>
              <dd>v{status.schemaVersion}</dd>
              <dt>Sync</dt>
              <dd>{status.syncState}</dd>
            </dl>
          </section>
          <section className="card">
            <h2>Recent sessions</h2>
            {recent.length === 0 && (
              <p className="muted">No sessions yet — sync the addon after a farm run.</p>
            )}
            {recent.length > 0 && (
              <div className="table-wrap">
                <table className="data" data-testid="recent-sessions">
                  <thead>
                    <tr>
                      <th>Started</th>
                      <th>Zone</th>
                      <th>Time</th>
                      <th>Value now</th>
                    </tr>
                  </thead>
                  <tbody>
                    {recent.map((s) => (
                      <tr key={s.id}>
                        <td>{formatTime(s.startedAt)}</td>
                        <td>{s.zoneName ?? '—'}</td>
                        <td>{formatDuration(s.activeSeconds)}</td>
                        <td>{formatGold(s.valueNow)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            <Link className="btn btn-secondary" to="/live">
              Open Live Farm
            </Link>
          </section>
        </div>
      )}
    </div>
  );
}

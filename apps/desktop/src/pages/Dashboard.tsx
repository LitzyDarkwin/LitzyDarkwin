import { Link } from 'react-router-dom';
import { useAppStore } from '../stores/app';

export default function Dashboard() {
  const { status, statusError, loading } = useAppStore();
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
        </div>
      )}
    </div>
  );
}

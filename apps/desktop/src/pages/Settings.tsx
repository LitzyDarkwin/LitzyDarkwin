import { EmptyState } from '../components/States';

export default function Settings() {
  return (
    <div className="page">
      <header className="page-head">
        <h1>Settings</h1>
        <p className="page-sub">WoW location, sync, market, privacy and diagnostics.</p>
      </header>
      <div className="card">
        <EmptyState
          title="Settings arrive in Phase 2"
          hint="Backend-backed settings and diagnostics will live here."
        />
      </div>
    </div>
  );
}

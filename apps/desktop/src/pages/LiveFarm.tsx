import { EmptyState } from '../components/States';

export default function LiveFarm() {
  return (
    <div className="page">
      <header className="page-head">
        <h1>Live Farm</h1>
        <p className="page-sub">Active session tracking and expected vs realized GPH.</p>
      </header>
      <div className="card">
        <EmptyState
          title="Live Farm arrives in Phase 6"
          hint="Session lifecycle, sync freshness and realized GPH will live here."
        />
      </div>
    </div>
  );
}

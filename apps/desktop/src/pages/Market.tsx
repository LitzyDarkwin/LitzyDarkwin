import { EmptyState } from '../components/States';

export default function Market() {
  return (
    <div className="page">
      <header className="page-head">
        <h1>Market</h1>
        <p className="page-sub">Prices, conservative values, freshness and confidence.</p>
      </header>
      <div className="card">
        <EmptyState
          title="Market engine arrives in Phase 5"
          hint="TSM-backed valuations with spike protection will live here."
        />
      </div>
    </div>
  );
}

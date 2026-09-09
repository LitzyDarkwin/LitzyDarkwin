import { EmptyState } from '../components/States';

export default function RoutesPage() {
  return (
    <div className="page">
      <header className="page-head">
        <h1>Routes</h1>
        <p className="page-sub">Economic route optimizer and simulator.</p>
      </header>
      <div className="card">
        <EmptyState
          title="Route engine arrives in Phase 4"
          hint="Generate, simulate, compare and save farming routes here."
        />
      </div>
    </div>
  );
}

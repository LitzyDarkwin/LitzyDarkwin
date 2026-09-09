import { EmptyState } from '../components/States';

export default function Intelligence() {
  return (
    <div className="page">
      <header className="page-head">
        <h1>Intelligence</h1>
        <p className="page-sub">Your learned farming profile and prediction accuracy.</p>
      </header>
      <div className="card">
        <EmptyState
          title="Intelligence arrives in Phase 7"
          hint="Personal efficiency, zone strengths and explainable advice will live here."
        />
      </div>
    </div>
  );
}

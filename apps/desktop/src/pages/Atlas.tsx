import { EmptyState } from '../components/States';

export default function Atlas() {
  return (
    <div className="page">
      <header className="page-head">
        <h1>Atlas</h1>
        <p className="page-sub">Expansions, zones, resources and node density.</p>
      </header>
      <div className="card">
        <EmptyState
          title="Atlas arrives in Phase 3"
          hint="Zone browser, resource filters and density maps will live here."
        />
      </div>
    </div>
  );
}

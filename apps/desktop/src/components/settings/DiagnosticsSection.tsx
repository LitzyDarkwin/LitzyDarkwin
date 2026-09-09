import { formatAgo } from '../../lib/format';
import type { DiagnosticReport } from '../../types/domain';
import { FreshnessBadge } from '../Badges';
import { ErrorState, LoadingState } from '../States';

interface DiagnosticsSectionProps {
  report: DiagnosticReport | null;
  loading: boolean;
  error: string | null;
  onRefresh: () => void;
}

export default function DiagnosticsSection({ report, loading, error, onRefresh }: DiagnosticsSectionProps) {
  return (
    <section className="card" aria-labelledby="diag-heading">
      <h2 id="diag-heading">Diagnostics</h2>
      {loading && <LoadingState label="Collecting diagnostics…" />}
      {error && <ErrorState message={error} onRetry={onRefresh} />}
      {report && (
        <>
          <dl className="kv">
            <dt>App version</dt>
            <dd>{report.appVersion}</dd>
            <dt>Schema</dt>
            <dd>{report.migrationState}</dd>
            <dt>Database</dt>
            <dd className="mono">{report.dbPath}</dd>
            <dt>WoW detected</dt>
            <dd>{report.wowDetected ? 'yes' : 'no'}</dd>
            <dt>Addon detected</dt>
            <dd>
              {report.addonDetected ? `yes${report.addonVersion ? ` (v${report.addonVersion})` : ''}` : 'no'}
            </dd>
            <dt>Last sync</dt>
            <dd>
              {report.lastSyncAt ? formatAgo(report.lastSyncAt) : 'never'}{' '}
              <FreshnessBadge value={report.syncFreshness} />
            </dd>
            <dt>Market</dt>
            <dd>
              {report.marketProvider} <FreshnessBadge value={report.marketFreshness} />
            </dd>
            <dt>Sessions / observations / routes</dt>
            <dd>
              {report.sessionCount} / {report.observationCount} / {report.routeCount}
            </dd>
          </dl>
          {report.lastMarketError && (
            <p className="small" style={{ color: 'var(--warn)' }}>
              Last market error: {report.lastMarketError}
            </p>
          )}
          {report.warnings.length > 0 && (
            <ul className="explain">
              {report.warnings.map((w, i) => (
                <li key={i} className="flat">⚠ {w}</li>
              ))}
            </ul>
          )}
          <div className="btn-row">
            <button type="button" className="btn btn-secondary" onClick={onRefresh}>
              Refresh
            </button>
          </div>
        </>
      )}
    </section>
  );
}

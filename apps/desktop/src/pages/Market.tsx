import { useEffect, useState } from 'react';
import { EmptyState, ErrorState, LoadingState } from '../components/States';
import { formatGold, formatInt, formatPct, formatTime } from '../lib/format';
import {
  getItemValuation,
  getMarketStatus,
  refreshMarket,
  searchMarketItems,
} from '../lib/market-api';
import { toMessage } from '../lib/tauri';
import type {
  Freshness,
  ItemValuation,
  MarketItemRow,
  MarketStatus,
  RefreshSummary,
} from '../types/domain';

function FreshBadge({ value }: { value: Freshness }) {
  return <span className={`badge badge-${value}`}>{value}</span>;
}

function ConfidenceBar({ value }: { value: number }) {
  return (
    <span className="conf-bar" title={`${Math.round(value * 100)}% confidence`}>
      <span className="conf-fill" style={{ width: `${Math.round(value * 100)}%` }} />
    </span>
  );
}

export default function Market() {
  const [status, setStatus] = useState<MarketStatus | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [refresh, setRefresh] = useState<RefreshSummary | null>(null);

  const [query, setQuery] = useState('bloom');
  const [results, setResults] = useState<MarketItemRow[]>([]);
  const [searchError, setSearchError] = useState<string | null>(null);

  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [valuation, setValuation] = useState<ItemValuation | null>(null);

  useEffect(() => {
    let live = true;
    getMarketStatus()
      .then((s) => live && setStatus(s))
      .catch((e: unknown) => live && setStatusError(toMessage(e)));
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    if (query.trim() === '') {
      setResults([]);
      return;
    }
    const timer = setTimeout(() => {
      searchMarketItems(query.trim(), 25)
        .then((rows) => {
          setResults(rows);
          setSearchError(null);
        })
        .catch((e: unknown) => setSearchError(toMessage(e)));
    }, 200);
    return () => clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    if (selectedId == null) {
      setValuation(null);
      return;
    }
    let live = true;
    getItemValuation(selectedId)
      .then((v) => live && setValuation(v))
      .catch((e: unknown) => live && setSearchError(toMessage(e)));
    return () => {
      live = false;
    };
  }, [selectedId]);

  const onRefresh = async () => {
    setRefreshing(true);
    try {
      const summary = await refreshMarket();
      setRefresh(summary);
      setStatus(await getMarketStatus());
      setStatusError(null);
    } catch (e: unknown) {
      setStatusError(toMessage(e));
    } finally {
      setRefreshing(false);
    }
  };

  const showValuation = valuation ?? null;

  return (
    <div className="page">
      <header className="page-head">
        <h1>Market</h1>
        <p className="page-sub">
          TSM public data (EU): live commodity quotes plus daily region statistics.
        </p>
      </header>

      <section className="card" aria-label="Market status">
        <h2>Status</h2>
        {statusError && <ErrorState message={statusError} />}
        {!status && !statusError && <LoadingState label="Loading market status…" />}
        {status && (
          <div data-testid="market-status">
            <div className="stat-grid">
              <div className="stat">
                <div className="stat-label">Provider / region</div>
                <div className="stat-value">
                  {status.provider} / {status.region}
                </div>
              </div>
              <div className="stat">
                <div className="stat-label">Cached items</div>
                <div className="stat-value">{formatInt(status.itemCount)}</div>
              </div>
              <div className="stat">
                <div className="stat-label">Freshness</div>
                <div className="stat-value">
                  <FreshBadge value={status.freshness} />
                </div>
              </div>
            </div>
            <div className="table-wrap">
              <table className="data">
                <thead>
                  <tr>
                    <th>Dataset</th>
                    <th>Snapshot</th>
                    <th>Fetched</th>
                    <th>Freshness</th>
                    <th>Health</th>
                  </tr>
                </thead>
                <tbody>
                  {status.datasets.map((d) => (
                    <tr key={d.dataset} data-testid={`dataset-${d.dataset}`}>
                      <td>{d.dataset}</td>
                      <td>{d.updatedAt ? formatTime(d.updatedAt) : '—'}</td>
                      <td>{d.fetchedAt ? formatTime(d.fetchedAt) : '—'}</td>
                      <td>
                        <FreshBadge value={d.freshness} />
                      </td>
                      <td>
                        {d.failures > 0 ? (
                          <span className="error-text">
                            {d.failures} failure(s){d.lastError ? `: ${d.lastError}` : ''}
                          </span>
                        ) : (
                          'ok'
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="btn-row">
              <button
                type="button"
                className="btn btn-primary"
                data-testid="refresh-btn"
                disabled={refreshing}
                onClick={onRefresh}
              >
                {refreshing ? 'Refreshing…' : 'Refresh market data'}
              </button>
            </div>
            {refresh && (
              <div data-testid="refresh-result">
                {refresh.errors.length > 0 && (
                  <ul className="warn-list">
                    {refresh.errors.map((e, i) => (
                      <li key={i}>{e}</li>
                    ))}
                  </ul>
                )}
                {[refresh.commodities, refresh.regionItems].map(
                  (r) =>
                    r && (
                      <div key={r.dataset} className="hint">
                        {r.dataset}: {formatInt(r.rowsUpserted)} upserted,{' '}
                        {formatInt(r.rowsSkipped)} skipped
                        {r.snapshotUpdatedAt ? ` (snapshot ${formatTime(r.snapshotUpdatedAt)})` : ''}.
                      </div>
                    ),
                )}
              </div>
            )}
          </div>
        )}
      </section>

      <div className="grid-2">
        <section className="card" aria-label="Item search">
          <h2>Items</h2>
          <div className="field">
            <label htmlFor="market-search">Search by name or item id</label>
            <input
              id="market-search"
              data-testid="search-input"
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="e.g. bismuth or 161502"
            />
          </div>
          {searchError && <ErrorState message={searchError} />}
          {results.length === 0 && !searchError && (
            <EmptyState title="No matches" hint="Try a different name or id." />
          )}
          {results.length > 0 && (
            <div className="table-wrap">
              <table className="data" data-testid="results-table">
                <thead>
                  <tr>
                    <th>Item</th>
                    <th>Market</th>
                    <th>Effective</th>
                    <th>Confidence</th>
                  </tr>
                </thead>
                <tbody>
                  {results.map((r) => (
                    <tr
                      key={r.itemId}
                      data-testid={`item-${r.itemId}`}
                      onClick={() => setSelectedId(r.itemId)}
                      className={r.itemId === selectedId ? 'selected' : undefined}
                      style={{ cursor: 'pointer' }}
                    >
                      <td>
                        {r.itemName} <span className="hint">#{r.itemId}</span>
                      </td>
                      <td>{r.marketValue > 0 ? formatGold(r.marketValue) : '—'}</td>
                      <td>{formatGold(r.effectiveValue)}</td>
                      <td>
                        <ConfidenceBar value={r.confidence} /> {formatPct(r.confidence)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>

        <section className="card" aria-label="Valuation detail">
          <h2>Valuation</h2>
          {!showValuation && (
            <EmptyState title="No item selected" hint="Pick a row to see the value breakdown." />
          )}
          {showValuation && (
            <div data-testid="valuation-detail">
              <div className="stat-grid">
                <div className="stat">
                  <div className="stat-label">Market (clamped)</div>
                  <div className="stat-value">{formatGold(showValuation.marketValue)}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">Conservative</div>
                  <div className="stat-value">{formatGold(showValuation.conservativeValue)}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">Effective / unit</div>
                  <div className="stat-value">{formatGold(showValuation.effectiveValue)}</div>
                </div>
                <div className="stat">
                  <div className="stat-label">Confidence</div>
                  <div className="stat-value">{formatPct(showValuation.confidence)}</div>
                </div>
              </div>
              <div className="field">
                <span className="field-label">Freshness</span>
                <FreshBadge value={showValuation.freshness} />
              </div>
              {showValuation.flags.length > 0 && (
                <ul className="chip-list">
                  {showValuation.flags.map((f) => (
                    <li key={f} className="chip" data-testid={`flag-${f}`}>
                      {f}
                    </li>
                  ))}
                </ul>
              )}
              <ul className="explain">
                {showValuation.explain.map((line, i) => (
                  <li key={i}>{line}</li>
                ))}
              </ul>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

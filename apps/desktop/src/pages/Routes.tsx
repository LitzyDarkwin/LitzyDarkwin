import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import RouteMap from '../components/RouteMap';
import { EmptyState, ErrorState, LoadingState } from '../components/States';
import { getAtlasTree, getZoneDetail } from '../lib/atlas-api';
import {
  formatAgo,
  formatDuration,
  formatGold,
  formatInt,
  formatQty,
} from '../lib/format';
import { getMarketPrices } from '../lib/market-api';
import {
  compareRoutes,
  deleteRoute,
  exportRouteString,
  generateRoutes,
  getRoute,
  listRoutes,
  saveRoute,
} from '../lib/routes-api';
import { toMessage } from '../lib/tauri';
import type {
  ExcludedArea,
  RouteCandidate,
  RouteConfigDto,
  RouteSummary,
  SavedRoute,
  ZoneResourceSummary,
} from '../types/domain';

interface ZoneOption {
  id: string;
  name: string;
}

const GOLD = 10_000;

function parsePositive(raw: string): number | null {
  const v = Number(raw);
  return raw.trim() !== '' && Number.isFinite(v) && v > 0 ? v : null;
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat">
      <div className="stat-label">{label}</div>
      <div className="stat-value">{value}</div>
    </div>
  );
}

export default function RoutesPage() {
  const [searchParams] = useSearchParams();

  // Zone + resources -------------------------------------------------------
  const [zones, setZones] = useState<ZoneOption[]>([]);
  const [zoneId, setZoneId] = useState(searchParams.get('zone') ?? '');
  const [resources, setResources] = useState<ZoneResourceSummary[]>([]);
  const [selected, setSelected] = useState<string[]>(() =>
    (searchParams.get('resources') ?? '').split(',').map((s) => s.trim()).filter(Boolean),
  );
  const [bootError, setBootError] = useState<string | null>(null);
  const [booting, setBooting] = useState(true);

  // Prices (gold per unit, keyed by resource id) + optimizer config ---------
  const [prices, setPrices] = useState<Record<string, string>>({});
  const [filling, setFilling] = useState(false);
  const [fillNote, setFillNote] = useState<string | null>(null);
  const [cfgSeed, setCfgSeed] = useState('20260909');
  const [cfgSpeed, setCfgSpeed] = useState('7');
  const [cfgGather, setCfgGather] = useState('3.5');
  const [cfgMaxClusters, setCfgMaxClusters] = useState('24');

  // Generation --------------------------------------------------------------
  const [candidates, setCandidates] = useState<RouteCandidate[] | null>(null);
  const [activeRank, setActiveRank] = useState(1);
  const [generating, setGenerating] = useState(false);
  const [genError, setGenError] = useState<string | null>(null);

  // Exclusions ---------------------------------------------------------------
  const [excluded, setExcluded] = useState<ExcludedArea[]>([]);
  const [exRadius, setExRadius] = useState('5');

  // Saved routes --------------------------------------------------------------
  const [saved, setSaved] = useState<RouteSummary[]>([]);
  const [savedError, setSavedError] = useState<string | null>(null);
  const [routeName, setRouteName] = useState('');
  const [saveError, setSaveError] = useState<string | null>(null);
  const [viewed, setViewed] = useState<SavedRoute | null>(null);
  const [compareIds, setCompareIds] = useState<string[]>([]);
  const [comparison, setComparison] = useState<SavedRoute[] | null>(null);
  const [exportCode, setExportCode] = useState<string | null>(null);

  const resourceName = useMemo(() => {
    const map: Record<string, string> = {};
    for (const r of resources) map[r.resourceId] = r.resourceName;
    return map;
  }, [resources]);

  const active = useMemo(
    () => candidates?.find((c) => c.rank === activeRank) ?? candidates?.[0] ?? null,
    [candidates, activeRank],
  );

  // Boot: zone list ------------------------------------------------------------
  useEffect(() => {
    let live = true;
    getAtlasTree()
      .then((tree) => {
        if (!live) return;
        const all: ZoneOption[] = [];
        for (const exp of tree.expansions) {
          for (const cont of exp.continents) {
            for (const z of cont.zones) all.push({ id: z.id, name: z.name });
          }
        }
        setZones(all);
        setZoneId((prev) => {
          if (prev && all.some((z) => z.id === prev)) return prev;
          return all[0]?.id ?? '';
        });
      })
      .catch((e: unknown) => live && setBootError(toMessage(e)))
      .finally(() => live && setBooting(false));
    return () => {
      live = false;
    };
  }, []);

  // Zone resources --------------------------------------------------------------
  useEffect(() => {
    if (!zoneId) {
      setResources([]);
      return;
    }
    let live = true;
    getZoneDetail(zoneId)
      .then((detail) => {
        if (!live) return;
        setResources(detail.resources);
        setSelected((prev) =>
          prev.filter((id) => detail.resources.some((r) => r.resourceId === id)),
        );
      })
      .catch((e: unknown) => live && setBootError(toMessage(e)));
    return () => {
      live = false;
    };
  }, [zoneId]);

  const refreshSaved = useCallback(() => {
    listRoutes()
      .then((rows) => {
        setSaved(rows);
        setSavedError(null);
      })
      .catch((e: unknown) => setSavedError(toMessage(e)));
  }, []);

  useEffect(() => {
    refreshSaved();
  }, [refreshSaved]);

  const toggleSelected = (id: string) => {
    setSelected((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
  };

  const onFillMarket = async () => {
    setFillNote(null);
    if (selected.length === 0) {
      setFillNote('Select at least one resource first.');
      return;
    }
    setFilling(true);
    try {
      const out = await getMarketPrices(selected);
      setPrices((prev) => {
        const next = { ...prev };
        for (const [id, copper] of Object.entries(out.prices)) {
          next[id] = String(copper / GOLD);
        }
        return next;
      });
      const filled = Object.keys(out.prices).length;
      const parts = [`Filled ${filled} price(s) from market.`];
      if (out.missing.length > 0) parts.push(`Missing: ${out.missing.join('; ')}.`);
      setFillNote(parts.join(' '));
    } catch (e: unknown) {
      setFillNote(toMessage(e));
    } finally {
      setFilling(false);
    }
  };

  const onGenerate = async () => {
    setGenError(null);
    if (!zoneId) {
      setGenError('Pick a zone first.');
      return;
    }
    if (selected.length === 0) {
      setGenError('Select at least one resource.');
      return;
    }
    const missing = selected.filter((id) => parsePositive(prices[id] ?? '') == null);
    if (missing.length > 0) {
      const names = missing.map((id) => resourceName[id] ?? id).join(', ');
      setGenError(`Enter a price (gold per unit) for: ${names}.`);
      return;
    }
    const copper: Record<string, number> = {};
    for (const id of selected) copper[id] = Math.round(Number(prices[id]) * GOLD);
    const config: RouteConfigDto = {};
    const seed = Number(cfgSeed);
    if (cfgSeed.trim() !== '' && Number.isFinite(seed)) config.seed = Math.floor(seed);
    const speed = parsePositive(cfgSpeed);
    if (speed != null) config.travelSpeedYps = speed;
    const gather = Number(cfgGather);
    if (cfgGather.trim() !== '' && Number.isFinite(gather) && gather >= 0) {
      config.gatherSeconds = gather;
    }
    const maxC = Number(cfgMaxClusters);
    if (cfgMaxClusters.trim() !== '' && Number.isFinite(maxC)) {
      config.maxClusters = Math.floor(maxC);
    }
    setGenerating(true);
    try {
      const result = await generateRoutes(zoneId, selected, config, copper, excluded);
      setCandidates(result);
      setActiveRank(result[0]?.rank ?? 1);
    } catch (e: unknown) {
      setGenError(toMessage(e));
    } finally {
      setGenerating(false);
    }
  };

  const onMapClick = (x: number, y: number) => {
    const radius = parsePositive(exRadius) ?? 5;
    setExcluded((prev) => [...prev, { x, y, radius }]);
  };

  const onSave = async () => {
    setSaveError(null);
    if (!active) {
      setSaveError('Generate candidates first.');
      return;
    }
    try {
      await saveRoute({
        name: routeName,
        zoneId: zoneId || null,
        resourceIds: selected,
        waypoints: active.waypoints,
        excluded,
        config: active.config,
        stats: active.stats,
      });
      setRouteName('');
      refreshSaved();
    } catch (e: unknown) {
      setSaveError(toMessage(e));
    }
  };

  const onView = async (id: string) => {
    setExportCode(null);
    try {
      setViewed(await getRoute(id));
    } catch (e: unknown) {
      setSavedError(toMessage(e));
    }
  };

  const onDelete = async (id: string) => {
    try {
      await deleteRoute(id);
      setCompareIds((prev) => prev.filter((x) => x !== id));
      setViewed((prev) => (prev?.id === id ? null : prev));
      setComparison((prev) => (prev?.some((r) => r.id === id) ? null : prev));
      refreshSaved();
    } catch (e: unknown) {
      setSavedError(toMessage(e));
    }
  };

  const onCompare = async () => {
    try {
      setComparison(await compareRoutes(compareIds));
    } catch (e: unknown) {
      setSavedError(toMessage(e));
    }
  };

  const onExport = async (id: string) => {
    try {
      const out = await exportRouteString(id);
      setExportCode(out.code);
    } catch (e: unknown) {
      setSavedError(toMessage(e));
    }
  };

  if (booting) {
    return (
      <div className="page">
        <header className="page-head">
          <h1>Routes</h1>
        </header>
        <LoadingState label="Loading zones…" />
      </div>
    );
  }

  return (
    <div className="page">
      <header className="page-head">
        <h1>Routes</h1>
        <p className="page-sub">
          Economic route optimizer: cluster node sightings, tour them, simulate gold per hour.
        </p>
      </header>
      {bootError && <ErrorState message={bootError} />}

      <div className="grid-2">
        <section className="card" aria-label="Route setup">
          <h2>Setup</h2>
          <div className="field">
            <label htmlFor="route-zone">Zone</label>
            <select
              id="route-zone"
              data-testid="zone-select"
              value={zoneId}
              onChange={(e) => setZoneId(e.target.value)}
            >
              {zones.map((z) => (
                <option key={z.id} value={z.id}>
                  {z.name}
                </option>
              ))}
            </select>
          </div>
          <div className="field">
            <span className="field-label">Resources + price per unit (gold)</span>
            {resources.length === 0 && <div className="hint">No resources in this zone.</div>}
            {resources.map((r) => (
              <div className="checkrow" key={r.resourceId}>
                <input
                  type="checkbox"
                  id={`res-${r.resourceId}`}
                  data-testid={`resource-${r.resourceId}`}
                  checked={selected.includes(r.resourceId)}
                  onChange={() => toggleSelected(r.resourceId)}
                />
                <label htmlFor={`res-${r.resourceId}`}>
                  {r.resourceName} ({formatInt(r.observations)} sightings)
                </label>
                <input
                  type="number"
                  min="0"
                  step="0.01"
                  placeholder="g/unit"
                  aria-label={`Price for ${r.resourceName}`}
                  data-testid={`price-${r.resourceId}`}
                  value={prices[r.resourceId] ?? ''}
                  onChange={(e) =>
                    setPrices((prev) => ({ ...prev, [r.resourceId]: e.target.value }))
                  }
                  style={{ width: '7em', marginLeft: 'auto' }}
                />
              </div>
            ))}
            <div className="btn-row">
              <button
                type="button"
                className="btn btn-secondary"
                data-testid="fill-market-btn"
                disabled={filling}
                onClick={onFillMarket}
              >
                {filling ? 'Filling…' : 'Fill prices from market'}
              </button>
            </div>
            {fillNote && (
              <div className="hint" data-testid="fill-note">
                {fillNote}
              </div>
            )}
          </div>
          <div className="field">
            <span className="field-label">Optimizer (optional overrides)</span>
            <div className="grid-2">
              <div>
                <label htmlFor="cfg-seed">Seed</label>
                <input
                  id="cfg-seed"
                  data-testid="cfg-seed"
                  type="number"
                  value={cfgSeed}
                  onChange={(e) => setCfgSeed(e.target.value)}
                />
              </div>
              <div>
                <label htmlFor="cfg-speed">Speed (yd/s)</label>
                <input
                  id="cfg-speed"
                  data-testid="cfg-speed"
                  type="number"
                  step="0.5"
                  value={cfgSpeed}
                  onChange={(e) => setCfgSpeed(e.target.value)}
                />
              </div>
              <div>
                <label htmlFor="cfg-gather">Gather (s/node)</label>
                <input
                  id="cfg-gather"
                  data-testid="cfg-gather"
                  type="number"
                  step="0.5"
                  value={cfgGather}
                  onChange={(e) => setCfgGather(e.target.value)}
                />
              </div>
              <div>
                <label htmlFor="cfg-maxclusters">Max clusters</label>
                <input
                  id="cfg-maxclusters"
                  data-testid="cfg-maxclusters"
                  type="number"
                  value={cfgMaxClusters}
                  onChange={(e) => setCfgMaxClusters(e.target.value)}
                />
              </div>
            </div>
          </div>
          <div className="btn-row">
            <button
              type="button"
              className="btn btn-primary"
              data-testid="generate-btn"
              disabled={generating}
              onClick={onGenerate}
            >
              {generating ? 'Optimizing…' : 'Generate routes'}
            </button>
          </div>
          {genError && (
            <div className="error-text" data-testid="generate-error" role="alert">
              {genError}
            </div>
          )}
        </section>

        <section className="card" aria-label="Candidates">
          <h2>Candidates</h2>
          {!candidates && (
            <EmptyState title="No candidates yet" hint="Set prices and generate routes." />
          )}
          {candidates?.map((c) => (
            <article
              key={c.rank}
              data-testid={`candidate-${c.rank}`}
              className={c.rank === active?.rank ? 'candidate active' : 'candidate'}
            >
              <div className="candidate-head">
                <strong>#{c.rank}</strong>
                <span>{formatGold(c.stats.gphExpected)}/h expected</span>
                <button
                  type="button"
                  className="btn btn-secondary"
                  data-testid={`select-candidate-${c.rank}`}
                  onClick={() => setActiveRank(c.rank)}
                >
                  Preview
                </button>
              </div>
              <div className="stat-grid">
                <Stat label="Conservative" value={`${formatGold(c.stats.gphConservative)}/h`} />
                <Stat label="Loop" value={formatDuration(c.stats.loopSeconds)} />
                <Stat label="Nodes" value={formatInt(c.stats.nodes)} />
                <Stat label="Yield/loop" value={formatQty(c.stats.qty)} />
                <Stat label="P10–P90/h" value={`${formatGold(c.stats.bandHourly.p10)}–${formatGold(c.stats.bandHourly.p90)}`} />
              </div>
              {c.stats.warnings.length > 0 && (
                <ul className="warn-list">
                  {c.stats.warnings.map((w, i) => (
                    <li key={i}>{w}</li>
                  ))}
                </ul>
              )}
            </article>
          ))}
        </section>
      </div>

      {active && (
        <section className="card" aria-label="Route preview">
          <h2>
            Preview — candidate #{active.rank} ({active.waypoints.length} stops)
          </h2>
          <RouteMap waypoints={active.waypoints} excluded={excluded} onMapClick={onMapClick} />
          <div className="field">
            <label htmlFor="ex-radius">Exclusion radius (map units, click map to add)</label>
            <input
              id="ex-radius"
              data-testid="exclusion-radius"
              type="number"
              min="1"
              step="1"
              value={exRadius}
              onChange={(e) => setExRadius(e.target.value)}
              style={{ width: '7em' }}
            />
          </div>
          {excluded.length > 0 && (
            <ul className="chip-list">
              {excluded.map((area, i) => (
                <li key={i} data-testid={`exclusion-${i}`} className="chip">
                  ({area.x.toFixed(1)}, {area.y.toFixed(1)}) r={area.radius}
                  <button
                    type="button"
                    data-testid={`remove-exclusion-${i}`}
                    aria-label={`Remove exclusion ${i + 1}`}
                    onClick={() => setExcluded((prev) => prev.filter((_, j) => j !== i))}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div className="hint">Exclusions apply when you regenerate.</div>
          <div className="field">
            <label htmlFor="route-name">Save this candidate as</label>
            <input
              id="route-name"
              data-testid="route-name"
              type="text"
              placeholder="e.g. Dorn bismuth loop"
              value={routeName}
              onChange={(e) => setRouteName(e.target.value)}
            />
          </div>
          <div className="btn-row">
            <button type="button" className="btn btn-primary" data-testid="save-btn" onClick={onSave}>
              Save route
            </button>
          </div>
          {saveError && (
            <div className="error-text" data-testid="save-error" role="alert">
              {saveError}
            </div>
          )}
        </section>
      )}

      <section className="card" aria-label="Saved routes">
        <h2>Saved routes</h2>
        {savedError && (
          <div className="error-text" data-testid="saved-error" role="alert">
            {savedError}
          </div>
        )}
        {saved.length === 0 && (
          <EmptyState title="No saved routes" hint="Save a candidate to compare it later." />
        )}
        {saved.length > 0 && (
          <div className="table-wrap">
            <table className="data" data-testid="saved-list">
              <thead>
                <tr>
                  <th>Compare</th>
                  <th>Name</th>
                  <th>GPH</th>
                  <th>Stops</th>
                  <th>Updated</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {saved.map((r) => (
                  <tr key={r.id} data-testid={`saved-${r.id}`}>
                    <td>
                      <input
                        type="checkbox"
                        aria-label={`Compare ${r.name}`}
                        data-testid={`compare-${r.id}`}
                        checked={compareIds.includes(r.id)}
                        onChange={() =>
                          setCompareIds((prev) =>
                            prev.includes(r.id)
                              ? prev.filter((x) => x !== r.id)
                              : [...prev, r.id],
                          )
                        }
                      />
                    </td>
                    <td>{r.name}</td>
                    <td>{formatGold(r.gphExpected)}/h</td>
                    <td>{r.waypointCount}</td>
                    <td>{formatAgo(r.updatedAt)}</td>
                    <td>
                      <div className="btn-row">
                        <button
                          type="button"
                          className="btn btn-secondary"
                          data-testid={`view-${r.id}`}
                          onClick={() => onView(r.id)}
                        >
                          View
                        </button>
                        <button
                          type="button"
                          className="btn btn-secondary"
                          data-testid={`delete-${r.id}`}
                          onClick={() => onDelete(r.id)}
                        >
                          Delete
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {saved.length > 0 && (
          <div className="btn-row">
            <button
              type="button"
              className="btn btn-secondary"
              data-testid="compare-btn"
              disabled={compareIds.length === 0}
              onClick={onCompare}
            >
              Compare selected ({compareIds.length})
            </button>
          </div>
        )}
      </section>

      {comparison && (
        <section className="card" aria-label="Comparison">
          <h2>Comparison</h2>
          <div className="table-wrap">
            <table className="data" data-testid="compare-table">
              <thead>
                <tr>
                  <th>Metric</th>
                  {comparison.map((r) => (
                    <th key={r.id}>{r.name}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>GPH expected</td>
                  {comparison.map((r) => (
                    <td key={r.id}>{formatGold(r.stats.gphExpected)}/h</td>
                  ))}
                </tr>
                <tr>
                  <td>GPH conservative</td>
                  {comparison.map((r) => (
                    <td key={r.id}>{formatGold(r.stats.gphConservative)}/h</td>
                  ))}
                </tr>
                <tr>
                  <td>Loop time</td>
                  {comparison.map((r) => (
                    <td key={r.id}>{formatDuration(r.stats.loopSeconds)}</td>
                  ))}
                </tr>
                <tr>
                  <td>Nodes / yield</td>
                  {comparison.map((r) => (
                    <td key={r.id}>
                      {formatInt(r.stats.nodes)} / {formatQty(r.stats.qty)}
                    </td>
                  ))}
                </tr>
                <tr>
                  <td>Value per loop</td>
                  {comparison.map((r) => (
                    <td key={r.id}>{formatGold(r.stats.effectiveValue)}</td>
                  ))}
                </tr>
                <tr>
                  <td>Band P10–P90/h</td>
                  {comparison.map((r) => (
                    <td key={r.id}>
                      {formatGold(r.stats.bandHourly.p10)}–{formatGold(r.stats.bandHourly.p90)}
                    </td>
                  ))}
                </tr>
              </tbody>
            </table>
          </div>
        </section>
      )}

      {viewed && (
        <section className="card" aria-label="Route detail">
          <h2>{viewed.name}</h2>
          <div className="stat-grid">
            <Stat label="GPH expected" value={`${formatGold(viewed.stats.gphExpected)}/h`} />
            <Stat label="GPH conservative" value={`${formatGold(viewed.stats.gphConservative)}/h`} />
            <Stat label="Loop" value={formatDuration(viewed.stats.loopSeconds)} />
            <Stat label="Stops" value={formatInt(viewed.waypoints.length)} />
          </div>
          <RouteMap waypoints={viewed.waypoints} excluded={viewed.excluded} label="Saved route map" />
          <div className="btn-row">
            <button
              type="button"
              className="btn btn-secondary"
              data-testid="export-btn"
              onClick={() => onExport(viewed.id)}
            >
              Copy in-game import code
            </button>
          </div>
          {exportCode && (
            <div className="field">
              <label htmlFor="export-code">Paste into the add-on (`/go import`)</label>
              <textarea
                id="export-code"
                data-testid="export-code"
                readOnly
                rows={3}
                value={exportCode}
                style={{ width: '100%' }}
              />
            </div>
          )}
        </section>
      )}
    </div>
  );
}

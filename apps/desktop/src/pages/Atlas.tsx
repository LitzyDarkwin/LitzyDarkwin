import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import AtlasMap from '../components/AtlasMap';
import { ErrorState, LoadingState } from '../components/States';
import { getAtlasTree, getZoneDetail, getZoneObservations, searchResources } from '../lib/atlas-api';
import { formatInt } from '../lib/format';
import { toMessage } from '../lib/tauri';
import type {
  AtlasTree,
  ClusteredObservations,
  ResourceHit,
  ZoneDetail,
} from '../types/domain';

const CATEGORIES = [
  { id: '', label: 'All professions' },
  { id: 'mining', label: 'Mining' },
  { id: 'herbalism', label: 'Herbalism' },
  { id: 'fishing', label: 'Fishing' },
  { id: 'skinning', label: 'Skinning' },
  { id: 'treasure', label: 'Treasure' },
  { id: 'mob', label: 'Mob farming' },
  { id: 'other', label: 'Other' },
];

export default function Atlas() {
  const [params] = useSearchParams();
  const [tree, setTree] = useState<AtlasTree | null>(null);
  const [treeError, setTreeError] = useState<string | null>(null);
  const [selectedZone, setSelectedZone] = useState<string | null>(params.get('zone'));
  const [detail, setDetail] = useState<ZoneDetail | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [obs, setObs] = useState<ClusteredObservations | null>(null);
  const [obsError, setObsError] = useState<string | null>(null);
  const [checked, setChecked] = useState<Record<string, boolean>>({});
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('');
  const [hits, setHits] = useState<ResourceHit[] | null>(null);

  useEffect(() => {
    getAtlasTree()
      .then(setTree)
      .catch((err) => setTreeError(toMessage(err)));
  }, []);

  // Auto-select the first zone with data when nothing is chosen.
  useEffect(() => {
    if (selectedZone || !tree) return;
    for (const exp of tree.expansions) {
      for (const cont of exp.continents) {
        const zone = cont.zones.find((z) => z.observationCount > 0) ?? cont.zones[0];
        if (zone) {
          setSelectedZone(zone.id);
          return;
        }
      }
    }
  }, [tree, selectedZone]);

  const loadZone = useCallback(async (zoneId: string, resourceIds: string[]) => {
    setDetailLoading(true);
    setDetailError(null);
    setObsError(null);
    try {
      const [d, o] = await Promise.all([
        getZoneDetail(zoneId),
        getZoneObservations(zoneId, resourceIds, 20),
      ]);
      setDetail(d);
      setObs(o);
    } catch (err) {
      setDetailError(toMessage(err));
    } finally {
      setDetailLoading(false);
    }
  }, []);

  // (Re)load detail when the zone changes; reset resource checks to all-on.
  useEffect(() => {
    if (!selectedZone) return;
    setChecked({});
    void loadZone(selectedZone, []);
  }, [selectedZone, loadZone]);

  const activeResources = useMemo(() => {
    if (!detail) return [];
    return detail.resources.filter((r) => checked[r.resourceId] !== false);
  }, [detail, checked]);

  const refreshMap = useCallback(
    (nextChecked: Record<string, boolean>) => {
      if (!selectedZone || !detail) return;
      const ids = detail.resources
        .filter((r) => nextChecked[r.resourceId] !== false)
        .map((r) => r.resourceId);
      if (ids.length === 0) {
        // Everything unchecked: empty map without a backend round-trip.
        setObs({ grid: 20, cells: [], totalPoints: 0, truncated: false });
        return;
      }
      getZoneObservations(selectedZone, ids, 20)
        .then(setObs)
        .catch((err) => setObsError(toMessage(err)));
    },
    [selectedZone, detail],
  );

  const toggleResource = (resourceId: string) => {
    const next = { ...checked, [resourceId]: checked[resourceId] === false };
    setChecked(next);
    refreshMap(next);
  };

  const runSearch = useCallback(async () => {
    try {
      setHits(await searchResources(query, category || undefined, 25));
    } catch (err) {
      setHits([]);
      setTreeError(toMessage(err));
    }
  }, [query, category]);

  useEffect(() => {
    const t = setTimeout(() => void runSearch(), 250);
    return () => clearTimeout(t);
  }, [runSearch]);

  const names = useMemo(() => {
    const map: Record<string, string> = {};
    for (const r of detail?.resources ?? []) map[r.resourceId] = r.resourceName;
    return map;
  }, [detail]);

  const routeHref = useMemo(() => {
    if (!selectedZone) return '/routes';
    const ids = activeResources.map((r) => r.resourceId).join(',');
    return `/routes?zone=${encodeURIComponent(selectedZone)}&resources=${encodeURIComponent(ids)}`;
  }, [selectedZone, activeResources]);

  return (
    <div className="page">
      <header className="page-head">
        <h1>Atlas</h1>
        <p className="page-sub">Expansions, zones, resources and node density.</p>
      </header>
      <div className="grid-2">
        <div>
          <section className="card" aria-label="Search">
            <div className="toolbar">
              <input
                type="text"
                placeholder="Search resources…"
                aria-label="Search resources"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                style={{ flex: 1 }}
              />
              <select aria-label="Profession filter" value={category} onChange={(e) => setCategory(e.target.value)}>
                {CATEGORIES.map((c) => (
                  <option key={c.id} value={c.id}>{c.label}</option>
                ))}
              </select>
            </div>
            {hits && hits.length > 0 && (
              <ul className="list" aria-label="Search results">
                {hits.slice(0, 8).map((h) => (
                  <li key={h.id}>
                    <strong>{h.name}</strong>{' '}
                    <span className="small muted">
                      {h.categoryName} · {formatInt(h.observationCount)} obs ·{' '}
                      {h.zones.slice(0, 3).map((z) => z.name).join(', ')}
                      {h.zones.length > 3 ? ` +${h.zones.length - 3}` : ''}
                    </span>
                  </li>
                ))}
              </ul>
            )}
            {hits && hits.length === 0 && (
              <p className="small muted">No resources match. Try another name or profession.</p>
            )}
          </section>
          <section className="card" aria-label="Zone browser">
            <h2>Zones</h2>
            {treeError && <ErrorState message={treeError} />}
            {!tree && !treeError && <LoadingState label="Loading atlas…" />}
            {tree?.expansions.map((exp) => (
              <div key={exp.id} style={{ marginBottom: 8 }}>
                <h3>{exp.name}</h3>
                {exp.continents.map((cont) => (
                  <div key={cont.id} style={{ marginBottom: 4 }}>
                    <div className="small muted">{cont.name}</div>
                    <ul className="list">
                      {cont.zones.map((z) => (
                        <li key={z.id}>
                          <button
                            type="button"
                            className="btn btn-secondary"
                            aria-pressed={z.id === selectedZone}
                            disabled={z.id === selectedZone}
                            onClick={() => setSelectedZone(z.id)}
                            style={{ width: '100%', textAlign: 'left' }}
                          >
                            {z.name}{' '}
                            <span className="small muted">
                              · {formatInt(z.observationCount)} obs · {z.resourceCount} res
                            </span>
                          </button>
                        </li>
                      ))}
                    </ul>
                  </div>
                ))}
              </div>
            ))}
          </section>
        </div>
        <div>
          {detailLoading && (
            <div className="card">
              <LoadingState label="Loading zone…" />
            </div>
          )}
          {detailError && (
            <div className="card">
              <ErrorState message={detailError} />
            </div>
          )}
          {!detailLoading && !detailError && detail && (
            <section className="card" aria-label="Zone detail">
              <h2>{detail.name}</h2>
              <p className="small muted">
                {detail.expansionName} · {detail.continentName}
                {detail.mapId ? ` · map ${detail.mapId}` : ''} ·{' '}
                {formatInt(detail.observationCount)} observations
              </p>
              {obsError && <ErrorState message={obsError} />}
              {obs && <AtlasMap data={obs} names={names} />}
              <h3 style={{ marginTop: 12 }}>Resources</h3>
              <ul className="list">
                {detail.resources.map((r) => (
                  <li key={r.resourceId}>
                    <label className="checkrow" style={{ cursor: 'pointer' }}>
                      <input
                        type="checkbox"
                        checked={checked[r.resourceId] !== false}
                        onChange={() => toggleResource(r.resourceId)}
                      />
                      <span>
                        <strong>{r.resourceName}</strong>{' '}
                        <span className="small muted">
                          {formatInt(r.observations)} obs · avg yield{' '}
                          {r.avgYield != null ? r.avgYield.toFixed(1) : '—'} · {r.sources.join(', ')}
                        </span>
                      </span>
                    </label>
                  </li>
                ))}
              </ul>
              <div className="btn-row">
                <Link className="btn btn-primary" to={routeHref}>
                  Generate route{activeResources.length > 0 ? ` (${activeResources.length})` : ''}
                </Link>
              </div>
              <p className="small muted">
                Data sources are attributed per resource. Seed points are starting priors —
                your synced sightings refine them.
              </p>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}

import { useMemo } from 'react';
import { colorForResource } from '../lib/colors';
import type { ClusteredObservations } from '../types/domain';

interface AtlasMapProps {
  data: ClusteredObservations;
  /** resourceId → display name for the legend/tooltips. */
  names: Record<string, string>;
}

/**
 * Density map: server-aggregated grid cells rendered as an SVG heatmap.
 * Map space is 0–100 in both axes (y grows downward, like WoW maps).
 * Presentation only — aggregation happens in Rust.
 */
export default function AtlasMap({ data, names }: AtlasMapProps) {
  const maxCount = useMemo(
    () => data.cells.reduce((m, c) => Math.max(m, c.count), 1),
    [data.cells],
  );
  const cellSize = 100 / data.grid;

  const legend = useMemo(() => {
    const totals = new Map<string, number>();
    for (const cell of data.cells) {
      for (const rc of cell.byResource) {
        totals.set(rc.resourceId, (totals.get(rc.resourceId) ?? 0) + rc.count);
      }
    }
    return [...totals.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8)
      .map(([resourceId, count]) => ({ resourceId, count }));
  }, [data.cells]);

  if (data.cells.length === 0) {
    return (
      <div className="map-wrap">
        <div className="state">
          <div className="state-title">No observations here</div>
          <div className="state-hint">
            Try widening the resource filter, or sync the add-on for fresh sightings.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div>
      <div className="map-wrap">
        <svg viewBox="0 0 100 100" role="img" aria-label="Node density map" data-testid="atlas-map">
          {data.cells.map((cell) => {
            const color = colorForResource(cell.topResource ?? 'unknown');
            const opacity = 0.22 + (cell.count / maxCount) * 0.73;
            const tip = cell.byResource
              .map((rc) => `${names[rc.resourceId] ?? rc.resourceId}: ${rc.count}`)
              .join(', ');
            return (
              <rect
                key={`${cell.gx},${cell.gy}`}
                data-testid="map-cell"
                x={cell.gx * cellSize + 0.15}
                y={cell.gy * cellSize + 0.15}
                width={cellSize - 0.3}
                height={cellSize - 0.3}
                rx={0.6}
                fill={color}
                fillOpacity={opacity}
              >
                <title>{`${cell.count} node(s) — ${tip}`}</title>
              </rect>
            );
          })}
        </svg>
      </div>
      <div className="legend">
        {legend.map((item) => (
          <span key={item.resourceId}>
            <span
              className="legend-dot"
              style={{ background: colorForResource(item.resourceId) }}
            />
            {names[item.resourceId] ?? item.resourceId} ({item.count})
          </span>
        ))}
        <span className="small muted">
          {data.totalPoints} point(s) in {data.cells.length} cell(s)
          {data.truncated ? ' — truncated at server cap' : ''}
        </span>
      </div>
    </div>
  );
}

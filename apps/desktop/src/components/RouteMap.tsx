import type { ExcludedArea, Waypoint } from '../types/domain';

interface RouteMapProps {
  waypoints: Waypoint[];
  excluded?: ExcludedArea[];
  /** When set, clicking the map reports 0–100 coordinates (exclusion editing). */
  onMapClick?: (x: number, y: number) => void;
  label?: string;
}

/**
 * Route preview: visit-order path through cluster waypoints + exclusion zones.
 * Map space is 0–100 in both axes (y grows downward, like WoW maps).
 * Presentation only — tours are optimized in Rust.
 */
export default function RouteMap({ waypoints, excluded = [], onMapClick, label }: RouteMapProps) {
  if (waypoints.length === 0) {
    return (
      <div className="map-wrap">
        <div className="state">
          <div className="state-title">No route to preview</div>
          <div className="state-hint">Generate candidates or open a saved route.</div>
        </div>
      </div>
    );
  }

  const loop = [...waypoints, waypoints[0]];
  const points = loop.map((w) => `${w.x.toFixed(2)},${w.y.toFixed(2)}`).join(' ');

  const handleClick = (e: React.MouseEvent<SVGSVGElement>) => {
    if (!onMapClick) return;
    const rect = e.currentTarget.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return;
    const x = Math.min(100, Math.max(0, ((e.clientX - rect.left) / rect.width) * 100));
    const y = Math.min(100, Math.max(0, ((e.clientY - rect.top) / rect.height) * 100));
    onMapClick(x, y);
  };

  return (
    <div className="map-wrap">
      <svg
        viewBox="0 0 100 100"
        role="img"
        aria-label={label ?? 'Route preview map'}
        data-testid="route-map"
        onClick={handleClick}
        style={onMapClick ? { cursor: 'crosshair' } : undefined}
      >
        {excluded.map((area, i) => (
          <circle
            key={`ex-${i}`}
            data-testid={`map-exclusion-${i}`}
            cx={area.x}
            cy={area.y}
            r={Math.max(area.radius, 0.5)}
            fill="rgba(220, 60, 60, 0.18)"
            stroke="rgba(220, 60, 60, 0.8)"
            strokeWidth="0.4"
            strokeDasharray="1.5 1"
          />
        ))}
        <polyline
          data-testid="route-path"
          points={points}
          fill="none"
          stroke="#e8b53a"
          strokeWidth="0.7"
        />
        {waypoints.map((w, i) => (
          <g key={`${w.clusterId}-${i}`}>
            <circle
              data-testid={`map-waypoint-${i}`}
              cx={w.x}
              cy={w.y}
              r="2.1"
              fill="#1d2530"
              stroke="#e8b53a"
              strokeWidth="0.5"
            />
            <text
              x={w.x}
              y={w.y + 1.3}
              textAnchor="middle"
              fontSize="2.6"
              fill="#e8b53a"
              pointerEvents="none"
            >
              {i + 1}
            </text>
            <title>{`Stop ${i + 1} (${w.clusterId}): ${w.x.toFixed(1)}, ${w.y.toFixed(1)}`}</title>
          </g>
        ))}
      </svg>
    </div>
  );
}

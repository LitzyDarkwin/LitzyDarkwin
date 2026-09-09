import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Waypoint } from '../types/domain';
import RouteMap from './RouteMap';

const WPS: Waypoint[] = [
  { x: 10, y: 10, clusterId: 'c0', value: 5 },
  { x: 50, y: 20, clusterId: 'c1', value: 6 },
  { x: 30, y: 70, clusterId: 'c2', value: 7 },
];

describe('RouteMap', () => {
  it('renders the closed path, numbered stops and exclusions', () => {
    render(<RouteMap waypoints={WPS} excluded={[{ x: 80, y: 80, radius: 6 }]} />);
    expect(screen.getByTestId('route-map')).toBeDefined();
    expect(screen.getByTestId('route-path').getAttribute('points')).toContain('10.00,10.00');
    expect(screen.getByTestId('map-waypoint-0')).toBeDefined();
    expect(screen.getByTestId('map-waypoint-2')).toBeDefined();
    expect(screen.getByTestId('map-exclusion-0')).toBeDefined();
  });

  it('shows an empty state without waypoints', () => {
    render(<RouteMap waypoints={[]} />);
    expect(screen.getByText('No route to preview')).toBeDefined();
  });

  it('reports 0–100 coordinates on click', () => {
    const onMapClick = vi.fn();
    render(<RouteMap waypoints={WPS} onMapClick={onMapClick} />);
    const svg = screen.getByTestId('route-map');
    svg.getBoundingClientRect = () =>
      ({ left: 10, top: 20, width: 200, height: 200 }) as DOMRect;
    fireEvent.click(svg, { clientX: 110, clientY: 120 });
    expect(onMapClick).toHaveBeenCalledTimes(1);
    const [x, y] = onMapClick.mock.calls[0] as [number, number];
    expect(x).toBeCloseTo(50, 9);
    expect(y).toBeCloseTo(50, 9);
  });
});

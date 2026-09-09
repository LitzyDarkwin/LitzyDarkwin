import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { ClusteredObservations } from '../types/domain';
import AtlasMap from './AtlasMap';

const DATA: ClusteredObservations = {
  grid: 10,
  totalPoints: 12,
  truncated: false,
  cells: [
    { gx: 1, gy: 2, cx: 15, cy: 25, count: 7, topResource: 'a', byResource: [{ resourceId: 'a', count: 7 }] },
    { gx: 5, gy: 6, cx: 55, cy: 65, count: 5, topResource: 'b', byResource: [{ resourceId: 'b', count: 5 }] },
  ],
};

describe('AtlasMap', () => {
  it('renders one rect per cell plus legend', () => {
    render(<AtlasMap data={DATA} names={{ a: 'Alpha', b: 'Beta' }} />);
    expect(screen.getByTestId('atlas-map')).toBeDefined();
    expect(screen.getAllByTestId('map-cell')).toHaveLength(2);
    expect(screen.getByText(/Alpha \(7\)/)).toBeDefined();
    expect(screen.getByText(/12 point\(s\) in 2 cell\(s\)/)).toBeDefined();
  });

  it('shows an empty state without cells', () => {
    render(<AtlasMap data={{ grid: 10, cells: [], totalPoints: 0, truncated: false }} names={{}} />);
    expect(screen.getByText('No observations here')).toBeDefined();
  });
});

import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import RoutesPage from './Routes';

function renderRoutes(initial = '/routes') {
  return render(
    <MemoryRouter initialEntries={[initial]}>
      <RoutesPage />
    </MemoryRouter>,
  );
}

async function generateWithPrices() {
  renderRoutes('/routes?zone=isle-of-dorn&resources=bismuth,mycobloom');
  // Handoff preselects zone + resources.
  const zone = (await screen.findByTestId('zone-select')) as HTMLSelectElement;
  expect(zone.value).toBe('isle-of-dorn');
  const bismuth = (await screen.findByTestId('resource-bismuth')) as HTMLInputElement;
  expect(bismuth.checked).toBe(true);
  fireEvent.change(screen.getByTestId('price-bismuth'), { target: { value: '5' } });
  fireEvent.change(screen.getByTestId('price-mycobloom'), { target: { value: '12' } });
  fireEvent.click(screen.getByTestId('generate-btn'));
  await screen.findByTestId('candidate-1');
}

describe('Routes page (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('requires prices before generating', async () => {
    renderRoutes('/routes?zone=isle-of-dorn&resources=bismuth');
    await screen.findByTestId('resource-bismuth');
    fireEvent.click(screen.getByTestId('generate-btn'));
    expect((await screen.findByTestId('generate-error')).textContent).toContain('Bismuth');
  });

  it('fills prices from the market', async () => {
    renderRoutes('/routes?zone=isle-of-dorn&resources=bismuth,mycobloom');
    await screen.findByTestId('resource-bismuth');
    fireEvent.click(screen.getByTestId('fill-market-btn'));
    const note = await screen.findByTestId('fill-note');
    expect(note.textContent).toContain('Filled 2 price(s)');
    expect((screen.getByTestId('price-bismuth') as HTMLInputElement).value).toBe('4.6912');
    expect((screen.getByTestId('price-mycobloom') as HTMLInputElement).value).toBe('10.571');
  });

  it('generates candidates and previews the selected one', async () => {
    await generateWithPrices();
    expect(screen.getByTestId('candidate-2')).toBeDefined();
    fireEvent.click(screen.getByTestId('select-candidate-2'));
    expect(screen.getByText(/Preview — candidate #2/)).toBeDefined();
    expect(screen.getByTestId('route-map')).toBeDefined();
    expect(screen.getByTestId('map-waypoint-0')).toBeDefined();
  });

  it('shows no personal block for plain candidates', async () => {
    await generateWithPrices();
    expect(screen.queryByTestId('personal-1')).toBeNull();
  });

  it('personalizes candidates with explain lines when enabled', async () => {
    renderRoutes('/routes?zone=isle-of-dorn&resources=bismuth,mycobloom');
    await screen.findByTestId('resource-bismuth');
    fireEvent.change(screen.getByTestId('price-bismuth'), { target: { value: '5' } });
    fireEvent.change(screen.getByTestId('price-mycobloom'), { target: { value: '12' } });
    fireEvent.click(screen.getByTestId('personalize-check'));
    fireEvent.click(screen.getByTestId('generate-btn'));
    const block = await screen.findByTestId('personal-1');
    expect(within(block).getByText(/Personal ×0\.85/)).toBeDefined();
    expect(within(block).getByText(/bismuth: yield/)).toBeDefined();
  });

  it('adds and removes map exclusions', async () => {
    await generateWithPrices();
    const svg = screen.getByTestId('route-map');
    svg.getBoundingClientRect = () =>
      ({ left: 0, top: 0, width: 100, height: 100 }) as DOMRect;
    fireEvent.change(screen.getByTestId('exclusion-radius'), { target: { value: '6' } });
    fireEvent.click(svg, { clientX: 25, clientY: 30 });
    expect((await screen.findByTestId('exclusion-0')).textContent).toContain('r=6');
    expect(screen.getByTestId('map-exclusion-0')).toBeDefined();
    fireEvent.click(screen.getByTestId('remove-exclusion-0'));
    expect(screen.queryByTestId('exclusion-0')).toBeNull();
  });

  it('saves, views, exports, compares and deletes routes', async () => {
    await generateWithPrices();
    fireEvent.change(screen.getByTestId('route-name'), { target: { value: 'E2E Loop' } });
    fireEvent.click(screen.getByTestId('save-btn'));
    const savedTable = await screen.findByTestId('saved-list');
    expect(within(savedTable).getByText('E2E Loop')).toBeDefined();

    // View + export the new route.
    const row = within(savedTable).getByText('E2E Loop').closest('tr') as HTMLElement;
    fireEvent.click(within(row).getByText('View'));
    expect(await screen.findByText('E2E Loop', { selector: 'h2' })).toBeDefined();
    fireEvent.click(screen.getByTestId('export-btn'));
    const code = (await screen.findByTestId('export-code')) as HTMLTextAreaElement;
    expect(code.value).toMatch(/^GO1\./);

    // Compare it against the seeded sample route.
    const boxes = within(savedTable).getAllByRole('checkbox');
    for (const box of boxes) fireEvent.click(box);
    fireEvent.click(screen.getByTestId('compare-btn'));
    const table = await screen.findByTestId('compare-table');
    expect(within(table).getByText('GPH expected')).toBeDefined();

    // Delete the new route.
    fireEvent.click(within(row).getByText('Delete'));
    await screen.findByTestId('saved-list');
    expect(within(screen.getByTestId('saved-list')).queryByText('E2E Loop')).toBeNull();
  });
});

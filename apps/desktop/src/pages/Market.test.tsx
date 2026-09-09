import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import Market from './Market';

function renderMarket() {
  return render(
    <MemoryRouter initialEntries={['/market']}>
      <Market />
    </MemoryRouter>,
  );
}

describe('Market page (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('shows dataset status and refreshes on demand', async () => {
    renderMarket();
    await screen.findByTestId('market-status');
    expect(screen.getByTestId('dataset-commodities')).toBeDefined();
    expect(screen.getByTestId('dataset-region-items')).toBeDefined();
    fireEvent.click(screen.getByTestId('refresh-btn'));
    const result = await screen.findByTestId('refresh-result');
    expect(result.textContent).toContain('5 upserted');
    expect(result.textContent).toContain('2 skipped');
  });

  it('searches items and shows the valuation breakdown', async () => {
    renderMarket();
    // Default query is "bloom" (debounced 200ms).
    const row = await screen.findByTestId('item-161504');
    expect(row.textContent).toContain('Mycobloom');
    fireEvent.click(row);
    const detail = await screen.findByTestId('valuation-detail');
    expect(detail.textContent).toContain('10.57g');
    fireEvent.change(screen.getByTestId('search-input'), { target: { value: '161505' } });
    const thin = await screen.findByTestId('item-161505');
    fireEvent.click(thin);
    await screen.findByTestId('flag-live-unknown');
  });

  it('shows an empty state without matches', async () => {
    renderMarket();
    await screen.findByTestId('market-status');
    fireEvent.change(screen.getByTestId('search-input'), { target: { value: 'zzz-nothing' } });
    await screen.findByText('No matches');
  });
});

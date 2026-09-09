import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import LiveFarm from './LiveFarm';

function renderLive() {
  return render(
    <MemoryRouter initialEntries={['/live']}>
      <LiveFarm />
    </MemoryRouter>,
  );
}

describe('LiveFarm page (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('shows sync state, totals and history', async () => {
    renderLive();
    await screen.findByTestId('sync-state');
    await screen.findByTestId('stats-card');
    const history = await screen.findByTestId('history-table');
    expect(within(history).getByTestId('session-s1')).toBeDefined();
    expect(within(history).getByTestId('session-s2')).toBeDefined();
    fireEvent.click(screen.getByTestId('sync-btn'));
    await screen.findByTestId('sync-msg');
  });

  it('filters history by zone', async () => {
    renderLive();
    await screen.findByTestId('history-table');
    fireEvent.change(screen.getByTestId('zone-filter'), { target: { value: 'ringing-deeps' } });
    const history = await screen.findByTestId('history-table');
    await within(history).findByTestId('session-s2');
    expect(within(history).queryByTestId('session-s1')).toBeNull();
  });

  it('opens detail, links a route and shows deltas', async () => {
    renderLive();
    const history = await screen.findByTestId('history-table');
    fireEvent.click(within(history).getByTestId('session-s1'));
    const detail = await screen.findByTestId('session-detail');
    expect(within(detail).getByTestId('loot-161502')).toBeDefined();
    expect(within(detail).getByTestId('loot-161504')).toBeDefined();
    fireEvent.change(screen.getByTestId('link-route-select'), {
      target: { value: 'rt-mock-1' },
    });
    const panel = await screen.findByTestId('expected-panel');
    // Δ then and Δ now agree here (mock then-prices equal live prices).
    await within(panel).findAllByText(/-83\.6%/);
    expect(within(panel).getAllByText(/-83\.6%/)).toHaveLength(2);
  });

  it('analyzes imported sessions in place', async () => {
    renderLive();
    const history = await screen.findByTestId('history-table');
    fireEvent.click(within(history).getByTestId('session-s2'));
    await screen.findByTestId('session-detail');
    fireEvent.click(await screen.findByTestId('analyze-btn'));
    await screen.findByTestId('session-detail');
    expect(screen.queryByTestId('analyze-btn')).toBeNull();
  });
});

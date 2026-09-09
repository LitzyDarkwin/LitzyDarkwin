import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import Intelligence from './Intelligence';

function renderIntel() {
  return render(
    <MemoryRouter initialEntries={['/intelligence']}>
      <Intelligence />
    </MemoryRouter>,
  );
}

describe('Intelligence page (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('shows learned metrics, zones and resources', async () => {
    renderIntel();
    expect((await screen.findByTestId('metric-sessions')).textContent).toBe('12');
    expect((await screen.findByTestId('metric-efficiency')).textContent).toBe('×0.85');
    const zones = await screen.findByTestId('zones-table');
    const zoneRow = within(zones).getByTestId('zone-isle-of-dorn');
    expect(within(zoneRow).getByText('Isle of Dorn')).toBeDefined();
    const resources = await screen.findByTestId('resources-table');
    expect(within(resources).getByTestId('resource-bismuth')).toBeDefined();
    expect(within(resources).getByTestId('resource-mycobloom')).toBeDefined();
  });

  it('rebuilds from sessions and reports counts', async () => {
    renderIntel();
    await screen.findByTestId('metric-sessions');
    fireEvent.click(screen.getByTestId('rebuild-btn'));
    const msg = await screen.findByTestId('rebuild-msg');
    expect(msg.textContent).toContain('12 sessions');
    expect(msg.textContent).toContain('2 resources');
  });
});

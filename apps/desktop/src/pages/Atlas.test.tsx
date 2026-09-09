import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import Atlas from './Atlas';

function renderAtlas(initial = '/atlas') {
  return render(
    <MemoryRouter initialEntries={[initial]}>
      <Atlas />
    </MemoryRouter>,
  );
}

describe('Atlas page (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('auto-selects the first zone and shows its map', async () => {
    renderAtlas();
    expect(await screen.findByText('Isle of Dorn', { selector: 'h2' })).toBeDefined();
    expect(await screen.findByTestId('atlas-map')).toBeDefined();
    expect(screen.getAllByTestId('map-cell').length).toBeGreaterThan(0);
  });

  it('switches zones from the browser', async () => {
    renderAtlas();
    await screen.findByText('Isle of Dorn', { selector: 'h2' });
    const browser = screen.getByLabelText('Zone browser');
    fireEvent.click(within(browser).getByText(/The Ringing Deeps/));
    expect(await screen.findByText('The Ringing Deeps', { selector: 'h2' })).toBeDefined();
  });

  it('honors a ?zone= deep link', async () => {
    renderAtlas('/atlas?zone=ringing-deeps');
    expect(await screen.findByText('The Ringing Deeps', { selector: 'h2' })).toBeDefined();
  });

  it('searches resources with profession filter', async () => {
    renderAtlas();
    await screen.findByText('Isle of Dorn', { selector: 'h2' });
    fireEvent.change(screen.getByLabelText('Search resources'), { target: { value: 'luredrop' } });
    expect(await screen.findByText('Luredrop')).toBeDefined();
  });

  it('unchecking all resources empties the map', async () => {
    renderAtlas();
    await screen.findByTestId('atlas-map');
    const boxes = screen.getAllByRole('checkbox') as HTMLInputElement[];
    expect(boxes.length).toBeGreaterThan(0);
    for (const box of boxes) fireEvent.click(box);
    expect(await screen.findByText('No observations here')).toBeDefined();
  });

  it('offers a route handoff link with zone + resources', async () => {
    renderAtlas();
    const link = (await screen.findByText(/Generate route/)) as HTMLAnchorElement;
    expect(link.getAttribute('href')).toContain('zone=isle-of-dorn');
    expect(link.getAttribute('href')).toContain('resources=');
  });
});

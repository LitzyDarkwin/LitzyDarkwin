import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import Settings from './Settings';

describe('Settings page (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('renders all four sections', async () => {
    render(<Settings />);
    expect(await screen.findByText('World of Warcraft & sync')).toBeDefined();
    expect(await screen.findByText('Market data')).toBeDefined();
    expect(await screen.findByText('Privacy & community data')).toBeDefined();
    expect(await screen.findByText('Diagnostics')).toBeDefined();
  });

  it('detects installs and syncs with a report', async () => {
    render(<Settings />);
    fireEvent.click(await screen.findByText('Detect WoW'));
    expect(await screen.findByText('Detected installs')).toBeDefined();
    fireEvent.click(await screen.findByText('Sync now'));
    expect(await screen.findByText(/Sync complete/)).toBeDefined();
    expect(await screen.findByText('Last sync report')).toBeDefined();
  });

  it('shows diagnostics warnings before first sync', async () => {
    render(<Settings />);
    expect(await screen.findByText(/never synced/)).toBeDefined();
  });

  it('saves privacy toggles', async () => {
    render(<Settings />);
    const share = (await screen.findByLabelText('Share anonymous farming observations')) as HTMLInputElement;
    expect(share.checked).toBe(false);
    fireEvent.click(share);
    fireEvent.click(await screen.findByText('Save privacy settings'));
    expect(await screen.findByText('Privacy settings saved.')).toBeDefined();
  });

  it('validates the market refresh interval', async () => {
    render(<Settings />);
    const hours = (await screen.findByLabelText('Refresh at most every (hours)')) as HTMLInputElement;
    fireEvent.change(hours, { target: { value: '999' } });
    fireEvent.click(await screen.findByText('Save market settings'));
    expect(await screen.findByText(/between 1 and 168/)).toBeDefined();
  });
});

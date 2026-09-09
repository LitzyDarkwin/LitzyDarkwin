import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import { getSettings, updateSettings } from './settings-api';

describe('settings-api (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('reads defaults', async () => {
    const s = await getSettings();
    expect(s.region).toBe('eu');
    expect(s.marketRefreshHours).toBe(12);
    expect(s.crowdShare).toBe(false);
  });

  it('persists patches across calls', async () => {
    await updateSettings({ region: 'us', crowdShare: true });
    const s = await getSettings();
    expect(s.region).toBe('us');
    expect(s.crowdShare).toBe(true);
  });

  it('surfaces backend validation errors', async () => {
    await expect(updateSettings({ region: 'xx' })).rejects.toThrow('Unknown region');
    await expect(updateSettings({ marketRefreshHours: 999 })).rejects.toThrow('1–168');
  });
});

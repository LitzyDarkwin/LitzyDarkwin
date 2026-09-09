import { beforeEach, describe, expect, it, vi } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import { detectWow, getSyncState, setWowDir, syncAddon } from './sync-api';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args?: Record<string, unknown>) => ({ cmd, args })),
}));

describe('sync-api (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('detects a preview install', async () => {
    const found = await detectWow();
    expect(found).toHaveLength(1);
    expect(found[0].addonInstalled).toBe(true);
  });

  it('rejects bogus WoW paths', async () => {
    await expect(setWowDir('C:\\Nothing\\Here')).rejects.toThrow('_retail_');
  });

  it('syncs and records freshness state', async () => {
    const before = await getSyncState();
    expect(before.freshness).toBe('unknown');
    const report = await syncAddon();
    expect(report.sessionsImported).toBe(2);
    const after = await getSyncState();
    expect(after.freshness).toBe('fresh');
    expect(after.lastSyncAt).toBe(report.syncedAt);
  });
});

describe('sync-api (tauri arg contracts)', () => {
  it('passes { path } to set_wow_dir', async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await setWowDir('C:\\WoW');
      expect(invoke).toHaveBeenCalledWith('set_wow_dir', { path: 'C:\\WoW' });
      await syncAddon();
      expect(invoke).toHaveBeenCalledWith('sync_addon', undefined);
    } finally {
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    }
  });
});

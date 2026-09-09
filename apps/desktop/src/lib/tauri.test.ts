import { describe, expect, it, vi } from 'vitest';
import { invokeSafe, isTauriRuntime, toMessage } from './tauri';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => ({ mocked: cmd })),
}));

describe('isTauriRuntime', () => {
  it('is false in jsdom without internals', () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    expect(isTauriRuntime()).toBe(false);
  });
  it('is true when internals exist', () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    expect(isTauriRuntime()).toBe(true);
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });
});

describe('invokeSafe', () => {
  it('falls back to the mock backend outside Tauri', async () => {
    const status = await invokeSafe<{ version: string }>('get_status');
    expect(status.version).toContain('mock');
  });
  it('throws a helpful error for unmocked commands', async () => {
    await expect(invokeSafe('nope_missing')).rejects.toThrow('no mock implementation');
  });
});

describe('toMessage', () => {
  it('normalizes error shapes', () => {
    expect(toMessage('plain')).toBe('plain');
    expect(toMessage(new Error('boom'))).toBe('boom');
    expect(toMessage({ message: 'obj' })).toBe('obj');
    expect(toMessage(42)).toBe('Unexpected error');
  });
});

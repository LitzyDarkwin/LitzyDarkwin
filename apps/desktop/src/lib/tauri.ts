/**
 * Single choke point for backend calls. Inside the Tauri runtime it uses
 * `invoke`; anywhere else (vite preview, browser, unit tests) it delegates to
 * the typed mock provider so the UI stays reviewable without the shell.
 * No component may import `@tauri-apps/api` directly.
 */
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { mockInvoke, hasMock } from '../mocks/backend';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauriRuntime(): boolean {
  return (
    typeof window !== 'undefined' && typeof window.__TAURI_INTERNALS__ !== 'undefined'
  );
}

/** Invoke a Tauri command, or the mock backend when no runtime is present. */
export async function invokeSafe<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) {
    return tauriInvoke<T>(cmd, args);
  }
  if (hasMock(cmd)) {
    return mockInvoke<T>(cmd, args);
  }
  throw new Error(
    `Command "${cmd}" has no mock implementation (running without the desktop backend).`,
  );
}

/** Normalize unknown throws into a displayable message. */
export function toMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  if (err && typeof err === 'object' && 'message' in err) return String(err.message);
  return 'Unexpected error';
}

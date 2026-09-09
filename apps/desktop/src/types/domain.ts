/** Shared DTO types mirroring Rust `serde` output (camelCase). Extended per phase. */

export type Freshness = 'fresh' | 'aging' | 'stale' | 'unavailable' | 'unknown';

export interface StatusPayload {
  version: string;
  schemaVersion: number;
  dbPath: string;
  syncState: string;
}

export interface BackendError {
  code: string;
  message: string;
}

/** Async data wrapper used by page states. */
export type Loadable<T> =
  | { state: 'loading' }
  | { state: 'ready'; data: T }
  | { state: 'empty' }
  | { state: 'error'; message: string };

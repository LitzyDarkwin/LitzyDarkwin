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

/* Phase 2 — settings, sync, diagnostics ------------------------------------ */

export interface Settings {
  wowDir: string | null;
  region: string;
  marketProvider: string;
  marketRefreshHours: number;
  crowdShare: boolean;
  crowdUse: boolean;
  crowdEndpoint: string;
  routeConfig: unknown;
  movementProfile: unknown;
  uiLastRoute: string | null;
}

export interface SettingsPatch {
  wowDir?: string | null;
  region?: string;
  marketProvider?: string;
  marketRefreshHours?: number;
  crowdShare?: boolean;
  crowdUse?: boolean;
  crowdEndpoint?: string;
  routeConfig?: unknown;
  movementProfile?: unknown;
  uiLastRoute?: string | null;
}

export interface AccountSv {
  account: string;
  path: string;
  sizeBytes: number;
  mtime: number;
}

export interface WowDetection {
  wowDir: string;
  retailDir: string;
  addonsDir: string;
  addonInstalled: boolean;
  addonVersion: string | null;
  addonInterface: string | null;
  accounts: AccountSv[];
}

export interface SyncFileReport {
  account: string;
  path: string;
  exportTimestamp: number | null;
  sessionsSeen: number;
  sessionsImported: number;
  sessionsUpdated: number;
  sessionsSkipped: number;
  observationsSeen: number;
  observationsImported: number;
  observationsSkipped: number;
  warnings: string[];
}

export interface SyncReport {
  syncedAt: number;
  files: SyncFileReport[];
  sessionsImported: number;
  sessionsUpdated: number;
  sessionsSkipped: number;
  observationsImported: number;
  warnings: string[];
}

export interface SyncStateInfo {
  lastSyncAt: number | null;
  lastExportAt: number | null;
  lastResult: string | null;
  freshness: Freshness;
}

export interface DiagnosticReport {
  appVersion: string;
  schemaVersion: number;
  migrationState: string;
  dbPath: string;
  dbSizeBytes: number;
  wowDir: string | null;
  wowDetected: boolean;
  addonDetected: boolean;
  addonVersion: string | null;
  lastSyncAt: number | null;
  syncFreshness: Freshness;
  marketProvider: string;
  marketFreshness: Freshness;
  lastMarketError: string | null;
  sessionCount: number;
  observationCount: number;
  routeCount: number;
  warnings: string[];
}

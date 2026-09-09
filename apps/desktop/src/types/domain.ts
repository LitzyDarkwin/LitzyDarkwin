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

/* Phase 3 — atlas ------------------------------------------------------------ */

export interface TreeZone {
  id: string;
  name: string;
  mapId: number | null;
  observationCount: number;
  resourceCount: number;
}

export interface TreeContinent {
  id: string;
  name: string;
  zones: TreeZone[];
}

export interface TreeExpansion {
  id: string;
  name: string;
  short: string;
  continents: TreeContinent[];
}

export interface AtlasTree {
  expansions: TreeExpansion[];
}

export interface ZoneRef {
  id: string;
  name: string;
}

export interface ResourceHit {
  id: string;
  name: string;
  categoryId: string;
  categoryName: string;
  itemId: number | null;
  observationCount: number;
  zones: ZoneRef[];
}

export interface ZoneResourceSummary {
  resourceId: string;
  resourceName: string;
  categoryId: string;
  itemId: number | null;
  observations: number;
  sources: string[];
  avgYield: number | null;
  newestObservation: number | null;
}

export interface ZoneDetail {
  id: string;
  name: string;
  mapId: number | null;
  continentId: string;
  continentName: string;
  expansionId: string;
  expansionName: string;
  yardsPerUnit: number;
  observationCount: number;
  resources: ZoneResourceSummary[];
}

export interface ResourceCount {
  resourceId: string;
  count: number;
}

export interface GridCell {
  gx: number;
  gy: number;
  cx: number;
  cy: number;
  count: number;
  topResource: string | null;
  byResource: ResourceCount[];
}

export interface ClusteredObservations {
  grid: number;
  cells: GridCell[];
  totalPoints: number;
  truncated: boolean;
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

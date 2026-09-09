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

/** Partial optimizer config from the UI; backend fills defaults + clamps. */
export interface RouteConfigDto {
  seed?: number | null;
  cellSize?: number | null;
  maxDiameter?: number | null;
  minSamples?: number | null;
  maxClusters?: number | null;
  candidateStarts?: number | null;
  twoOptPasses?: number | null;
  travelSpeedYps?: number | null;
  gatherSeconds?: number | null;
  loopOverheadSeconds?: number | null;
}

export interface RouteConfig {
  seed: number;
  cellSize: number;
  maxDiameter: number;
  minSamples: number;
  maxClusters: number;
  candidateStarts: number;
  twoOptPasses: number;
  travelSpeedYps: number;
  gatherSeconds: number;
  loopOverheadSeconds: number;
}

export interface ExcludedArea {
  x: number;
  y: number;
  radius: number;
}

export interface Waypoint {
  x: number;
  y: number;
  clusterId: string;
  value: number;
}

export interface Band {
  p10: number;
  median: number;
  p90: number;
}

export interface RouteStats {
  loopYards: number;
  loopSeconds: number;
  nodes: number;
  qty: number;
  grossValue: number;
  effectiveValue: number;
  gphExpected: number;
  gphConservative: number;
  bandHourly: Band;
  resolvedPrices: Record<string, number>;
  prunedClusters: number;
  droppedPoints: number;
  warnings: string[];
}

export interface RouteCandidate {
  rank: number;
  waypoints: Waypoint[];
  stats: RouteStats;
  /** Effective config (post-defaults, post-clamp) so saves reproduce exactly. */
  config: RouteConfig;
}

export interface SavedRoute {
  id: string;
  name: string;
  zoneId: string | null;
  resourceIds: string[];
  waypoints: Waypoint[];
  excluded: ExcludedArea[];
  config: RouteConfig;
  stats: RouteStats;
  createdAt: number;
  updatedAt: number;
}

export interface RouteSummary {
  id: string;
  name: string;
  zoneId: string | null;
  zoneName: string | null;
  gphExpected: number;
  waypointCount: number;
  updatedAt: number;
}

export interface SaveRouteInput {
  name: string;
  zoneId: string | null;
  resourceIds: string[];
  waypoints: Waypoint[];
  excluded: ExcludedArea[];
  config: RouteConfig;
  stats: RouteStats;
}

export interface RouteExport {
  code: string;
  mapId: number;
  waypointCount: number;
}

export interface IngestReport {
  dataset: string;
  rowsTotal: number;
  rowsUpserted: number;
  rowsSkipped: number;
  snapshotUpdatedAt: number | null;
}

export interface RefreshSummary {
  region: string;
  commodities: IngestReport | null;
  regionItems: IngestReport | null;
  errors: string[];
}

export interface DatasetStatus {
  dataset: string;
  updatedAt: number | null;
  fetchedAt: number | null;
  freshness: Freshness;
  failures: number;
  lastError: string | null;
}

export interface MarketStatus {
  provider: string;
  region: string;
  itemCount: number;
  datasets: DatasetStatus[];
  freshness: Freshness;
}

export interface ItemValuation {
  itemId: number;
  marketValue: number;
  conservativeValue: number;
  effectiveValue: number;
  confidence: number;
  freshness: Freshness;
  flags: string[];
  explain: string[];
}

export interface MarketItemRow {
  itemId: number;
  itemName: string;
  marketValue: number;
  effectiveValue: number;
  confidence: number;
  freshness: Freshness;
}

export interface MarketPrices {
  prices: Record<string, number>;
  missing: string[];
}

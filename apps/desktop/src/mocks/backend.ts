/**
 * Typed mock backend used when the UI runs without the Tauri runtime
 * (vite preview, browser review, unit tests). Each mock mirrors the real
 * command's DTO shape. Extended per phase alongside the real commands.
 *
 * Stateful where the UI needs it (settings persist across calls within a
 * page lifetime); `resetMockBackend()` restores defaults (tests call it).
 */

import type {
  DiagnosticReport,
  Settings,
  SyncReport,
  SyncStateInfo,
  WowDetection,
} from '../types/domain';
import { MOCK_TREE, mockObservations, mockSearch, mockZoneDetail } from './atlas-data';

type Handler = (args?: Record<string, unknown>) => unknown;

const MOCK_VERSION = '0.1.0-mock';
const NOW = () => Math.floor(Date.now() / 1000);

interface MockState {
  settings: Settings;
  lastSyncAt: number | null;
  lastExportAt: number | null;
  lastResult: string | null;
}

function defaultState(): MockState {
  return {
    settings: {
      wowDir: 'C:\\Program Files (x86)\\World of Warcraft (preview)',
      region: 'eu',
      marketProvider: 'tsm',
      marketRefreshHours: 12,
      crowdShare: false,
      crowdUse: false,
      crowdEndpoint: '',
      routeConfig: {},
      movementProfile: {},
      uiLastRoute: null,
    },
    lastSyncAt: null,
    lastExportAt: null,
    lastResult: null,
  };
}

let mockState: MockState = defaultState();

export function resetMockBackend(): void {
  mockState = defaultState();
}

function mockDetection(wowDir: string): WowDetection {
  return {
    wowDir,
    retailDir: `${wowDir}\\_retail_`,
    addonsDir: `${wowDir}\\_retail_\\Interface\\AddOns`,
    addonInstalled: true,
    addonVersion: '0.1.0',
    addonInterface: '120100',
    accounts: [
      {
        account: 'ACCT1',
        path: `${wowDir}\\_retail_\\WTF\\Account\\ACCT1\\SavedVariables\\GatherOptimizer.lua`,
        sizeBytes: 12340,
        mtime: NOW() - 3600,
      },
    ],
  };
}

const handlers: Record<string, Handler> = {
  get_status: () => ({
    version: MOCK_VERSION,
    schemaVersion: 6,
    dbPath: '(browser preview — no local database)',
    syncState: mockState.lastResult ?? 'not_synced',
  }),

  detect_wow: () => [mockDetection('C:\\Program Files (x86)\\World of Warcraft')],

  set_wow_dir: (args) => {
    const path = String(args?.path ?? '');
    if (!path.toLowerCase().includes('warcraft')) {
      throw new Error('No `_retail_` folder found there — is this the WoW install?');
    }
    mockState.settings.wowDir = path;
    return mockDetection(path);
  },

  sync_addon: () => {
    if (!mockState.settings.wowDir) {
      throw new Error('WoW directory is not configured.');
    }
    const now = NOW();
    mockState.lastSyncAt = now;
    mockState.lastExportAt = now - 600;
    mockState.lastResult = 'ok';
    const report: SyncReport = {
      syncedAt: now,
      files: [
        {
          account: 'ACCT1',
          path: '(preview)',
          exportTimestamp: mockState.lastExportAt,
          sessionsSeen: 2,
          sessionsImported: 2,
          sessionsUpdated: 0,
          sessionsSkipped: 0,
          observationsSeen: 12,
          observationsImported: 12,
          observationsSkipped: 0,
          warnings: [],
        },
      ],
      sessionsImported: 2,
      sessionsUpdated: 0,
      sessionsSkipped: 0,
      observationsImported: 12,
      warnings: [],
    };
    return report;
  },

  get_sync_state: () => {
    const state: SyncStateInfo = {
      lastSyncAt: mockState.lastSyncAt,
      lastExportAt: mockState.lastExportAt,
      lastResult: mockState.lastResult,
      freshness: mockState.lastSyncAt ? 'fresh' : 'unknown',
    };
    return state;
  },

  get_settings: () => ({ ...mockState.settings }),

  update_settings: (args) => {
    const patch = (args?.patch ?? {}) as Partial<Settings>;
    if (patch.region !== undefined && !['eu', 'us', 'kr', 'tw', 'cn'].includes(patch.region)) {
      throw new Error(`Unknown region \`${patch.region}\``);
    }
    if (
      patch.marketRefreshHours !== undefined &&
      (patch.marketRefreshHours < 1 || patch.marketRefreshHours > 168)
    ) {
      throw new Error('Market refresh interval must be 1–168 hours');
    }
    mockState.settings = { ...mockState.settings, ...patch };
    return { ...mockState.settings };
  },

  get_diagnostics: () => {
    const report: DiagnosticReport = {
      appVersion: MOCK_VERSION,
      schemaVersion: 6,
      migrationState: 'v6 (latest)',
      dbPath: '(browser preview — no local database)',
      dbSizeBytes: 0,
      wowDir: mockState.settings.wowDir,
      wowDetected: mockState.settings.wowDir !== null,
      addonDetected: true,
      addonVersion: '0.1.0',
      lastSyncAt: mockState.lastSyncAt,
      syncFreshness: mockState.lastSyncAt ? 'fresh' : 'unknown',
      marketProvider: mockState.settings.marketProvider,
      marketFreshness: 'unknown',
      lastMarketError: null,
      sessionCount: mockState.lastSyncAt ? 2 : 0,
      observationCount: mockState.lastSyncAt ? 12 : 0,
      routeCount: 0,
      warnings: mockState.lastSyncAt ? [] : ['never synced with the addon (preview)'],
    };
    return report;
  },

  get_atlas_tree: () => MOCK_TREE,

  search_resources: (args) =>
    mockSearch(String(args?.query ?? ''), (args?.category as string | null) ?? null),

  get_zone_detail: (args) => mockZoneDetail(String(args?.zoneId ?? '')),

  get_zone_observations: (args) =>
    mockObservations(
      String(args?.zoneId ?? ''),
      ((args?.resourceIds as string[] | undefined) ?? []) as string[],
      Number(args?.grid ?? 20),
    ),
};

export function hasMock(cmd: string): boolean {
  return cmd in handlers;
}

export async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const handler = handlers[cmd];
  if (!handler) throw new Error(`No mock for command "${cmd}"`);
  // Microtask turn so loading states behave like the real async bridge.
  await Promise.resolve();
  return handler(args) as T;
}

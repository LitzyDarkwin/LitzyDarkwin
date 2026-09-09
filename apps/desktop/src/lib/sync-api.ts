import { invokeSafe } from './tauri';
import type { SyncReport, SyncStateInfo, WowDetection } from '../types/domain';

export function detectWow(): Promise<WowDetection[]> {
  return invokeSafe<WowDetection[]>('detect_wow');
}

export function setWowDir(path: string): Promise<WowDetection> {
  return invokeSafe<WowDetection>('set_wow_dir', { path });
}

export function syncAddon(): Promise<SyncReport> {
  return invokeSafe<SyncReport>('sync_addon');
}

export function getSyncState(): Promise<SyncStateInfo> {
  return invokeSafe<SyncStateInfo>('get_sync_state');
}

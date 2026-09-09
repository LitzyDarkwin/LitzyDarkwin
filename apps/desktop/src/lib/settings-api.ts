import { invokeSafe } from './tauri';
import type { Settings, SettingsPatch } from '../types/domain';

export function getSettings(): Promise<Settings> {
  return invokeSafe<Settings>('get_settings');
}

export function updateSettings(patch: SettingsPatch): Promise<Settings> {
  return invokeSafe<Settings>('update_settings', { patch });
}

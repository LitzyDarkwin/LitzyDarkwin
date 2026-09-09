import { invokeSafe } from './tauri';
import type { PersonalProfile, RebuildReport } from '../types/domain';

export function getPersonalProfile(): Promise<PersonalProfile> {
  return invokeSafe<PersonalProfile>('get_personal_profile', {});
}

export function rebuildPersonal(): Promise<RebuildReport> {
  return invokeSafe<RebuildReport>('rebuild_personal', {});
}

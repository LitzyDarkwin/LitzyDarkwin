import { invokeSafe } from './tauri';
import type { StatusPayload } from '../types/domain';

export function getStatus(): Promise<StatusPayload> {
  return invokeSafe<StatusPayload>('get_status');
}

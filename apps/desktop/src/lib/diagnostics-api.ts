import { invokeSafe } from './tauri';
import type { DiagnosticReport } from '../types/domain';

export function getDiagnostics(): Promise<DiagnosticReport> {
  return invokeSafe<DiagnosticReport>('get_diagnostics');
}

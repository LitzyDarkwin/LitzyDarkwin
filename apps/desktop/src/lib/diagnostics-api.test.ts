import { beforeEach, describe, expect, it } from 'vitest';
import { resetMockBackend } from '../mocks/backend';
import { getDiagnostics } from './diagnostics-api';

describe('diagnostics-api (mock backend)', () => {
  beforeEach(() => resetMockBackend());

  it('returns a well-shaped report', async () => {
    const d = await getDiagnostics();
    expect(d.schemaVersion).toBe(6);
    expect(d.migrationState).toContain('v6');
    expect(d.syncFreshness).toBe('unknown');
    expect(d.warnings.length).toBeGreaterThan(0);
    expect(d.wowDir).toContain('Warcraft');
  });
});

/**
 * Typed mock backend used when the UI runs without the Tauri runtime
 * (vite preview, browser review, unit tests). Each mock mirrors the real
 * command's DTO shape. Extended per phase alongside the real commands.
 */

type Handler = (args?: Record<string, unknown>) => unknown;

const MOCK_VERSION = '0.1.0-mock';

const handlers: Record<string, Handler> = {
  get_status: () => ({
    version: MOCK_VERSION,
    schemaVersion: 6,
    dbPath: '(browser preview — no local database)',
    syncState: 'not_synced',
  }),
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

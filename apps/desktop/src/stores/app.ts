import { create } from 'zustand';
import { getStatus } from '../lib/status-api';
import { isTauriRuntime, toMessage } from '../lib/tauri';
import type { StatusPayload } from '../types/domain';

interface AppState {
  status: StatusPayload | null;
  statusError: string | null;
  loading: boolean;
  isMock: boolean;
  refreshStatus: () => Promise<void>;
}

export const useAppStore = create<AppState>((set) => ({
  status: null,
  statusError: null,
  loading: false,
  isMock: !isTauriRuntime(),
  refreshStatus: async () => {
    set({ loading: true, statusError: null });
    try {
      const status = await getStatus();
      set({ status, loading: false, isMock: !isTauriRuntime() });
    } catch (err) {
      set({ statusError: toMessage(err), loading: false });
    }
  },
}));

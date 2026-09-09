import { useCallback, useEffect, useState } from 'react';
import DiagnosticsSection from '../components/settings/DiagnosticsSection';
import MarketSection from '../components/settings/MarketSection';
import PrivacySection from '../components/settings/PrivacySection';
import WowSection from '../components/settings/WowSection';
import { ErrorState, LoadingState } from '../components/States';
import { getDiagnostics } from '../lib/diagnostics-api';
import { getSettings } from '../lib/settings-api';
import { getSyncState } from '../lib/sync-api';
import { toMessage } from '../lib/tauri';
import type { DiagnosticReport, Settings as SettingsData, SyncStateInfo } from '../types/domain';

export default function Settings() {
  const [settings, setSettings] = useState<SettingsData | null>(null);
  const [syncState, setSyncState] = useState<SyncStateInfo | null>(null);
  const [diagnostics, setDiagnostics] = useState<DiagnosticReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [s, sync, diag] = await Promise.all([getSettings(), getSyncState(), getDiagnostics()]);
      setSettings(s);
      setSyncState(sync);
      setDiagnostics(diag);
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className="page">
      <header className="page-head">
        <h1>Settings</h1>
        <p className="page-sub">WoW location, sync, market, privacy and diagnostics.</p>
      </header>
      {loading && (
        <div className="card">
          <LoadingState label="Loading settings…" />
        </div>
      )}
      {error && (
        <div className="card">
          <ErrorState message={error} onRetry={() => void refresh()} />
        </div>
      )}
      {!loading && !error && settings && syncState && (
        <div className="grid-2">
          <WowSection settings={settings} syncState={syncState} onChanged={() => void refresh()} />
          <MarketSection settings={settings} onChanged={() => void refresh()} />
          <PrivacySection settings={settings} onChanged={() => void refresh()} />
          <DiagnosticsSection
            report={diagnostics}
            loading={false}
            error={null}
            onRefresh={() => void refresh()}
          />
        </div>
      )}
    </div>
  );
}

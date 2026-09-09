import { useState } from 'react';
import { detectWow, setWowDir, syncAddon } from '../../lib/sync-api';
import { toMessage } from '../../lib/tauri';
import { formatAgo, formatTime } from '../../lib/format';
import type { Settings, SyncReport, SyncStateInfo, WowDetection } from '../../types/domain';
import { FreshnessBadge } from '../Badges';

interface WowSectionProps {
  settings: Settings;
  syncState: SyncStateInfo;
  onChanged: () => void;
}

export default function WowSection({ settings, syncState, onChanged }: WowSectionProps) {
  const [candidates, setCandidates] = useState<WowDetection[] | null>(null);
  const [manualPath, setManualPath] = useState('');
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [lastReport, setLastReport] = useState<SyncReport | null>(null);

  async function run(label: string, fn: () => Promise<void>) {
    setBusy(label);
    setError(null);
    setNotice(null);
    try {
      await fn();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setBusy(null);
    }
  }

  const handleDetect = () =>
    run('detect', async () => {
      const found = await detectWow();
      setCandidates(found);
      if (found.length === 0) {
        setNotice('No WoW install found in default locations — enter the path manually.');
      }
    });

  const handleUse = (path: string) =>
    run('use', async () => {
      await setWowDir(path);
      setNotice('WoW directory saved.');
      onChanged();
    });

  const handleManual = () =>
    run('manual', async () => {
      if (!manualPath.trim()) {
        setError('Enter a path first.');
        return;
      }
      await setWowDir(manualPath.trim());
      setNotice('WoW directory saved.');
      onChanged();
    });

  const handleSync = () =>
    run('sync', async () => {
      const report = await syncAddon();
      setLastReport(report);
      setNotice(
        `Sync complete: ${report.sessionsImported} session(s), ${report.observationsImported} observation(s).`,
      );
      onChanged();
    });

  return (
    <section className="card" aria-labelledby="wow-heading">
      <h2 id="wow-heading">World of Warcraft &amp; sync</h2>
      <dl className="kv">
        <dt>WoW directory</dt>
        <dd className="mono">{settings.wowDir ?? 'not configured'}</dd>
        <dt>Last sync</dt>
        <dd>
          {syncState.lastSyncAt ? formatAgo(syncState.lastSyncAt) : 'never'}{' '}
          <FreshnessBadge value={syncState.freshness} />
        </dd>
        <dt>Add-on data as of</dt>
        <dd>{formatTime(syncState.lastExportAt)}</dd>
      </dl>
      <p className="small muted">
        Sync reads the add-on&rsquo;s SavedVariables file. WoW only writes that file on logout or
        <span className="mono"> /reload</span> — the desktop never pretends to be live.
      </p>

      <div className="btn-row">
        <button type="button" className="btn btn-secondary" disabled={busy !== null} onClick={handleDetect}>
          {busy === 'detect' ? 'Detecting…' : 'Detect WoW'}
        </button>
        <button
          type="button"
          className="btn btn-primary"
          disabled={busy !== null || !settings.wowDir}
          onClick={handleSync}
        >
          {busy === 'sync' ? 'Syncing…' : 'Sync now'}
        </button>
      </div>

      {candidates && candidates.length > 0 && (
        <div style={{ marginTop: 12 }}>
          <h3>Detected installs</h3>
          <ul className="list">
            {candidates.map((c) => (
              <li key={c.wowDir}>
                <span className="mono">{c.wowDir}</span>{' '}
                <span className="small muted">
                  {c.addonInstalled
                    ? `addon v${c.addonVersion ?? '?'} · ${c.accounts.length} account(s)`
                    : 'addon NOT installed'}
                </span>{' '}
                <button
                  type="button"
                  className="btn btn-secondary"
                  disabled={busy !== null}
                  onClick={() => void handleUse(c.wowDir)}
                >
                  Use
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="field" style={{ marginTop: 12 }}>
        <label htmlFor="wow-manual">Set path manually</label>
        <input
          id="wow-manual"
          type="text"
          placeholder="C:\Program Files (x86)\World of Warcraft"
          value={manualPath}
          onChange={(e) => setManualPath(e.target.value)}
        />
        <div className="btn-row">
          <button
            type="button"
            className="btn btn-secondary"
            disabled={busy !== null}
            onClick={() => void handleManual()}
          >
            {busy === 'manual' ? 'Saving…' : 'Save path'}
          </button>
        </div>
      </div>

      {lastReport && (
        <div style={{ marginTop: 12 }}>
          <h3>Last sync report</h3>
          <dl className="kv">
            <dt>Sessions imported</dt>
            <dd>{lastReport.sessionsImported}</dd>
            <dt>Sessions updated</dt>
            <dd>{lastReport.sessionsUpdated}</dd>
            <dt>Sessions skipped</dt>
            <dd>{lastReport.sessionsSkipped}</dd>
            <dt>Observations imported</dt>
            <dd>{lastReport.observationsImported}</dd>
          </dl>
          {lastReport.warnings.length > 0 && (
            <ul className="explain">
              {lastReport.warnings.map((w, i) => (
                <li key={i} className="flat">⚠ {w}</li>
              ))}
            </ul>
          )}
        </div>
      )}

      {notice && <p className="small" role="status">{notice}</p>}
      {error && (
        <p className="small" role="alert" style={{ color: 'var(--bad)' }}>
          {error}
        </p>
      )}
    </section>
  );
}

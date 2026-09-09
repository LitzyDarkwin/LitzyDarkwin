import { useState } from 'react';
import { updateSettings } from '../../lib/settings-api';
import { toMessage } from '../../lib/tauri';
import type { Settings } from '../../types/domain';

interface MarketSectionProps {
  settings: Settings;
  onChanged: () => void;
}

export default function MarketSection({ settings, onChanged }: MarketSectionProps) {
  const [region, setRegion] = useState(settings.region);
  const [hours, setHours] = useState(String(settings.marketRefreshHours));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const handleSave = async () => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const refreshHours = Number(hours);
      if (!Number.isInteger(refreshHours) || refreshHours < 1 || refreshHours > 168) {
        setError('Refresh interval must be a whole number of hours between 1 and 168.');
        return;
      }
      await updateSettings({ region, marketRefreshHours: refreshHours });
      setNotice('Market settings saved.');
      onChanged();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card" aria-labelledby="market-heading">
      <h2 id="market-heading">Market data</h2>
      <div className="field">
        <label htmlFor="market-region">Region</label>
        <select id="market-region" value={region} onChange={(e) => setRegion(e.target.value)}>
          <option value="eu">EU (supported)</option>
          <option value="us">US (reserved)</option>
          <option value="kr">KR (reserved)</option>
          <option value="tw">TW (reserved)</option>
          <option value="cn">CN (reserved)</option>
        </select>
        <div className="hint">Only EU pricing is wired up initially; other regions validate but have no data yet.</div>
      </div>
      <div className="field">
        <label htmlFor="market-hours">Refresh at most every (hours)</label>
        <input
          id="market-hours"
          type="number"
          min={1}
          max={168}
          value={hours}
          onChange={(e) => setHours(e.target.value)}
        />
        <div className="hint">Provider: TSM public data (no key needed). Cached data is never deleted by a failed refresh.</div>
      </div>
      <div className="btn-row">
        <button type="button" className="btn btn-secondary" disabled={busy} onClick={() => void handleSave()}>
          {busy ? 'Saving…' : 'Save market settings'}
        </button>
      </div>
      {notice && <p className="small" role="status">{notice}</p>}
      {error && (
        <p className="small" role="alert" style={{ color: 'var(--bad)' }}>
          {error}
        </p>
      )}
    </section>
  );
}

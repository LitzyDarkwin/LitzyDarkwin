import { useState } from 'react';
import { updateSettings } from '../../lib/settings-api';
import { toMessage } from '../../lib/tauri';
import type { Settings } from '../../types/domain';

interface PrivacySectionProps {
  settings: Settings;
  onChanged: () => void;
}

export default function PrivacySection({ settings, onChanged }: PrivacySectionProps) {
  const [share, setShare] = useState(settings.crowdShare);
  const [use, setUse] = useState(settings.crowdUse);
  const [endpoint, setEndpoint] = useState(settings.crowdEndpoint);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const handleSave = async () => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await updateSettings({ crowdShare: share, crowdUse: use, crowdEndpoint: endpoint });
      setNotice('Privacy settings saved.');
      onChanged();
    } catch (err) {
      setError(toMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card" aria-labelledby="privacy-heading">
      <h2 id="privacy-heading">Privacy &amp; community data</h2>
      <p className="small muted">
        Everything works offline with your own data. Community features are strictly opt-in and
        only ever exchange coarse, anonymous farming signals — never credentials, chat, or
        precise movement trails.
      </p>
      <div className="field checkrow">
        <input
          id="crowd-share"
          type="checkbox"
          checked={share}
          onChange={(e) => setShare(e.target.checked)}
        />
        <div>
          <label htmlFor="crowd-share">Share anonymous farming observations</label>
          <div className="hint">Upload coarse zone/resource success signals. Off by default.</div>
        </div>
      </div>
      <div className="field checkrow">
        <input id="crowd-use" type="checkbox" checked={use} onChange={(e) => setUse(e.target.checked)} />
        <div>
          <label htmlFor="crowd-use">Use community observations</label>
          <div className="hint">Blend community spawn priors into your estimates. Off by default.</div>
        </div>
      </div>
      <div className="field">
        <label htmlFor="crowd-endpoint">Community endpoint (optional)</label>
        <input
          id="crowd-endpoint"
          type="text"
          placeholder="https://… (leave empty unless you run a community server)"
          value={endpoint}
          onChange={(e) => setEndpoint(e.target.value)}
        />
      </div>
      <div className="btn-row">
        <button type="button" className="btn btn-secondary" disabled={busy} onClick={() => void handleSave()}>
          {busy ? 'Saving…' : 'Save privacy settings'}
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

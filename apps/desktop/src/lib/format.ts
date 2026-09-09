/** Presentation formatting. Money arrives as integer copper. */

export function formatGold(copper: number | null | undefined): string {
  if (copper == null || !Number.isFinite(copper)) return '—';
  const gold = copper / 10_000;
  if (gold >= 1000) return `${Math.round(gold).toLocaleString('en-US')}g`;
  if (gold >= 100) return `${gold.toFixed(1)}g`;
  return `${gold.toFixed(2)}g`;
}

export function formatInt(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return '—';
  return Math.round(n).toLocaleString('en-US');
}

export function formatQty(q: number | null | undefined): string {
  if (q == null || !Number.isFinite(q)) return '—';
  return q >= 100 ? q.toFixed(0) : q >= 10 ? q.toFixed(1) : q.toFixed(2);
}

export function formatDuration(totalSeconds: number | null | undefined): string {
  if (totalSeconds == null || !Number.isFinite(totalSeconds) || totalSeconds < 0) return '—';
  const s = Math.floor(totalSeconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}h ${String(m).padStart(2, '0')}m`;
  if (m > 0) return `${m}m ${String(sec).padStart(2, '0')}s`;
  return `${sec}s`;
}

export function formatTime(unixSecs: number | null | undefined): string {
  if (unixSecs == null || !unixSecs) return 'never';
  return new Date(unixSecs * 1000).toLocaleString();
}

export function formatAgo(unixSecs: number | null | undefined, nowMs = Date.now()): string {
  if (unixSecs == null || !unixSecs) return 'never';
  const diff = Math.max(0, Math.floor(nowMs / 1000) - unixSecs);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export function formatPct(ratio: number | null | undefined): string {
  if (ratio == null || !Number.isFinite(ratio)) return '—';
  return `${Math.round(ratio * 100)}%`;
}

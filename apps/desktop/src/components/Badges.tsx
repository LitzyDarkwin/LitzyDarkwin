import type { Freshness } from '../types/domain';

const FRESHNESS_LABEL: Record<Freshness, string> = {
  fresh: 'Fresh',
  aging: 'Aging',
  stale: 'Stale',
  unavailable: 'Unavailable',
  unknown: 'Unknown',
};

export function FreshnessBadge({ value, detail }: { value: Freshness; detail?: string }) {
  return (
    <span className={`badge badge-${value}`} title={detail ?? FRESHNESS_LABEL[value]}>
      {FRESHNESS_LABEL[value]}
      {detail ? ` · ${detail}` : ''}
    </span>
  );
}

export function ConfidenceBadge({ level }: { level: 'High' | 'Medium' | 'Low' }) {
  return <span className={`badge badge-conf-${level.toLowerCase()}`}>{level} confidence</span>;
}

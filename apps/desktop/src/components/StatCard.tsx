interface StatCardProps {
  label: string;
  value: string;
  sub?: string;
}

/** Compact analytical stat. Presentation only — values preformatted by callers. */
export default function StatCard({ label, value, sub }: StatCardProps) {
  return (
    <div className="stat">
      <div className="stat-label">{label}</div>
      <div className="stat-value">{value}</div>
      {sub && <div className="stat-sub">{sub}</div>}
    </div>
  );
}

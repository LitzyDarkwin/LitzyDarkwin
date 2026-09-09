import { useEffect } from 'react';
import { NavLink } from 'react-router-dom';
import { useAppStore } from '../stores/app';

const NAV = [
  { to: '/', label: 'Dashboard', end: true },
  { to: '/atlas', label: 'Atlas' },
  { to: '/routes', label: 'Routes' },
  { to: '/market', label: 'Market' },
  { to: '/live', label: 'Live Farm' },
  { to: '/intelligence', label: 'Intelligence' },
  { to: '/settings', label: 'Settings' },
];

export default function Layout({ children }: { children: React.ReactNode }) {
  const { status, refreshStatus, isMock } = useAppStore();

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  return (
    <div className="shell">
      <aside className="sidebar" aria-label="Primary">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">◆</span>
          <span className="brand-name">wowgoldfarm</span>
        </div>
        <nav className="nav">
          {NAV.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) => `nav-link${isActive ? ' nav-link-active' : ''}`}
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-foot">
          <div className="foot-row">
            <span className="foot-label">Backend</span>
            <span className={isMock ? 'foot-mock' : 'foot-live'}>
              {isMock ? 'Preview mock' : 'Desktop'}
            </span>
          </div>
          {status && <div className="foot-row foot-dim">v{status.version}</div>}
        </div>
      </aside>
      <main className="main">
        {isMock && (
          <div className="mock-banner" role="note">
            Browser preview mode — showing sample data. Run the desktop app for live data.
          </div>
        )}
        {children}
      </main>
    </div>
  );
}

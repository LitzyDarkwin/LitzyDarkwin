import { HashRouter, Navigate, Route, Routes } from 'react-router-dom';
import Layout from './components/Layout';
import Atlas from './pages/Atlas';
import Dashboard from './pages/Dashboard';
import Intelligence from './pages/Intelligence';
import LiveFarm from './pages/LiveFarm';
import Market from './pages/Market';
import RoutesPage from './pages/Routes';
import Settings from './pages/Settings';

/** Route table. Pages own presentation only; data flows via lib/*-api. */
export default function App() {
  return (
    <HashRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/atlas" element={<Atlas />} />
          <Route path="/routes" element={<RoutesPage />} />
          <Route path="/market" element={<Market />} />
          <Route path="/live" element={<LiveFarm />} />
          <Route path="/intelligence" element={<Intelligence />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Layout>
    </HashRouter>
  );
}

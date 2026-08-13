import React from 'react';
import ReactDOM from 'react-dom/client';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ThemeProvider } from './app/ThemeProvider';
import { AppShell } from './app/AppShell';
import { LandingPage } from './features/landing/LandingPage';
import { DocsLayout } from './features/docs/DocsLayout';
import './styles/index.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ThemeProvider>
      <HashRouter>
        <AppShell>
          <Routes>
            <Route path="/" element={<LandingPage />} />
            <Route path="/docs" element={<DocsLayout />} />
            <Route path="/docs/:slug" element={<DocsLayout />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </AppShell>
      </HashRouter>
    </ThemeProvider>
  </React.StrictMode>
);

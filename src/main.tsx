import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import SettingsApp from "./components/settings/SettingsApp";
import AppErrorBoundary from "./components/errors/AppErrorBoundary";
import E2ECrashProbe from "./e2eCrashProbe";
import "./index.css";
import "./scroll-performance.css";

// Test-only forced-throw hook for the #143 boundary e2e check. `VITE_E2E_CRASH_HOOK`
// is a compile-time constant: unset, this ternary folds to `null` and the
// import above is tree-shaken out of the production bundle.
const crashProbe = import.meta.env.VITE_E2E_CRASH_HOOK ? <E2ECrashProbe /> : null;

// The Settings window (S3 / #129) is a separate Tauri WebviewWindow that
// loads this same bundle at `index.html#/settings` (see
// `src/lib/settingsWindow.ts`). Branching on the hash here, once, at boot
// is simpler than pulling in a router for a single extra top-level view.
const isSettingsWindow = window.location.hash.startsWith("#/settings");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppErrorBoundary>
      {crashProbe}
      {isSettingsWindow ? <SettingsApp /> : <App />}
    </AppErrorBoundary>
  </React.StrictMode>,
);

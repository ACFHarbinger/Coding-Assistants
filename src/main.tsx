import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import SettingsApp from "./components/settings/SettingsApp";
import AppErrorBoundary from "./components/errors/AppErrorBoundary";
import "./index.css";
import "./scroll-performance.css";

// The Settings window (S3 / #129) is a separate Tauri WebviewWindow that
// loads this same bundle at `index.html#/settings` (see
// `src/lib/settingsWindow.ts`). Branching on the hash here, once, at boot
// is simpler than pulling in a router for a single extra top-level view.
const isSettingsWindow = window.location.hash.startsWith("#/settings");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppErrorBoundary>
      {isSettingsWindow ? <SettingsApp /> : <App />}
    </AppErrorBoundary>
  </React.StrictMode>,
);

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { HashRouter, Route, Routes } from "react-router-dom";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/600.css";
import { AppShell } from "./app/AppShell";
import { ThemeProvider } from "./app/ThemeProvider";
import { DocsLayout } from "./features/docs/DocsLayout";
import { LandingPage } from "./features/landing/LandingPage";
import { NotFoundPage } from "./features/not-found/NotFoundPage";
import "./styles/index.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root");
}

createRoot(root).render(
  <StrictMode>
    <ThemeProvider>
      <HashRouter>
        <AppShell>
          <Routes>
            <Route path="/" element={<LandingPage />} />
            <Route path="/docs" element={<DocsLayout />} />
            <Route path="/docs/:slug" element={<DocsLayout />} />
            <Route path="/docs/*" element={<DocsLayout />} />
            <Route path="*" element={<NotFoundPage />} />
          </Routes>
        </AppShell>
      </HashRouter>
    </ThemeProvider>
  </StrictMode>,
);

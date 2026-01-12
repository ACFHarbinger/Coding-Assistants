---
trigger: model_decision
description: When creating or updating TypeScript/React files in src/.
---

You are an expert in React 19 + Vite with TypeScript for this Tauri app.

Project Shape
- Frontend code lives in `src/`.
- Entry point: `src/main.tsx`; root component: `src/App.tsx`.
- Styling is in `src/index.css` and component-level CSS as needed; no CSS framework by default.
- Use ESM imports and browser-safe APIs.

Tauri Integration
- Use `invoke` from `@tauri-apps/api/core` for backend commands.
- Use `@tauri-apps/plugin-dialog` or `@tauri-apps/plugin-opener` when needed.
- Keep IPC payloads JSON-serializable and aligned with Rust `serde` structs.

Component Guidelines
- Prefer function components with typed props; avoid `React.FC` unless it adds value.
- Keep state local; lift only when shared.
- Use hooks with stable dependencies and cleanup effects.
- Avoid blocking the UI thread; offload heavy work to Rust commands.

TypeScript Standards
- Keep strict typing; avoid `any`.
- Prefer `unknown` for untrusted errors and validate before use.
- Keep shared types in `src/` and match Rust field names.

Testing
- No test harness is configured; if tests are requested, prefer Vitest + React Testing Library.

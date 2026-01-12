---
trigger: model_decision
description: When creating, modifying, or debugging the Tauri/React GUI layer.
---

You are a Tauri 2 + React frontend engineer for this repository.

Core Directives
1) Frontend/Backend split:
   - Frontend: `src/` (Vite + React).
   - Backend: `src-tauri/` (Rust).
2) IPC: Frontend uses `invoke` from `@tauri-apps/api/core`; backend exposes `#[tauri::command]`.
3) Keep the UI thread responsive; heavy work belongs in Rust async tasks.

Component Architecture
- Keep components under `src/` and wire them in `src/App.tsx`.
- Use `src/index.css` for global styles; avoid adding a new CSS framework unless requested.
- Favor small, composable components with clear props.

Data Flow
- Rust structs used across IPC must derive `Serialize`/`Deserialize`.
- Match JSON field names and shapes between Rust and TypeScript.

Common Workflows
- New command: add `#[tauri::command]` in `src-tauri/src/lib.rs` (or module), register in `src-tauri/src/main.rs`, call with `invoke`.
- New view: create a component in `src/` and add it to `src/App.tsx`.

Debugging Checklist
- `npm run dev` for the web UI.
- `npm run tauri dev` for the full app.
- Check the browser console and Rust logs for command errors.
- Verify command names and payloads match between frontend and backend.

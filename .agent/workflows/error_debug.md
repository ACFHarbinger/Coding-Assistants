---
description: When debugging runtime issues in this Tauri + React app.
---

You are debugging a Tauri + React application.

Workflow
1) Identify whether the failure is in `src/` (React/Vite) or `src-tauri/` (Rust).
2) Check `invoke` names and payload shapes against `#[tauri::command]` handlers.
3) Look for UI thread blocking or long-running sync work in React.
4) Inspect Rust logs for command failures, especially `std::process::Command` errors.

Checks
- Use `npm run dev` for frontend issues.
- Use `npm run tauri dev` for full app issues.

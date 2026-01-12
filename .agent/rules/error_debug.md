---
trigger: model_decision
description: When debugging code errors.
---

You are an expert debugging agent for this Tauri + React repository. Use systematic root-cause analysis and verify fixes end-to-end.

Debugging Flow
- Reproduce the issue and capture exact errors.
- Identify whether the failure is in `src/` (React/Vite) or `src-tauri/` (Rust/Tauri).
- Inspect IPC calls: command name, payload shape, and Rust handler.

Repo-Specific Checks
- For frontend issues: run `npm run dev` and check the browser console.
- For app issues: run `npm run tauri dev` and check Rust logs.
- For Rust compile errors: use `cargo check` in `src-tauri/` if requested.

Fix Verification
- Re-run the original steps.
- Confirm no new errors in both frontend and backend logs.

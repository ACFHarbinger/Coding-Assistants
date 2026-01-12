---
description: When working on TypeScript/React files in src/.
---

You are a React 19 + TypeScript engineer for this Tauri app.

Workflow
1) Locate the component in `src/` and understand props/state flows.
2) Make typed changes; avoid `any` and keep error handling explicit.
3) If IPC is required, add/adjust `invoke` calls and keep payloads JSON-safe.
4) Update any shared types to mirror Rust structs used across IPC.

Checks
- Verify hooks dependencies and effect cleanup.
- Keep UI thread light; avoid blocking operations.

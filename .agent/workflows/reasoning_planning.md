---
description: When planning work for this Tauri + React repository.
---

You are planning changes for a Tauri + React codebase.

Workflow
1) Decide which changes belong in `src/` vs `src-tauri/`.
2) Identify any IPC contract changes and plan both sides.
3) Sequence work: backend commands/types first, then frontend `invoke` calls, then UI.
4) Note any verification commands if requested.

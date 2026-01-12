---
description: When refactoring code in this Tauri + React repository.
---

You are refactoring with minimal behavior change.

Workflow
1) Identify impacted UI components (`src/`) and backend commands (`src-tauri/`).
2) Keep IPC contracts stable unless explicitly requested to change them.
3) Make small, incremental edits and keep types aligned across the boundary.

Checks
- Update matching `invoke` calls if a command signature changes.
- Avoid adding new frameworks or tools unless asked.

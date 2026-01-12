---
trigger: model_decision
description: When developing the plan for a new task.
---

You are a careful planner for this Tauri + React repository.

Planning Checklist
- Identify whether work is in `src/` (frontend) or `src-tauri/` (backend).
- Note any IPC contracts that will need to change (`invoke` names or payloads).
- Confirm tooling constraints: Vite for frontend, Cargo for Rust.
- Prefer minimal changes that fit existing structure and dependencies.

Execution Order
1) Update data models or commands in Rust if needed.
2) Update frontend types and `invoke` usage.
3) Add or adjust UI components.
4) Verify the change with the most relevant run command if requested.

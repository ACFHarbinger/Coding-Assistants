---
description: When creating or modifying UI components in the Tauri + React frontend.
---

You are a Tauri 2 + React frontend engineer for this repository.

Workflow
1) Identify the UI surface in `src/` and where it is wired in `src/App.tsx`.
2) Keep components small and typed; avoid heavy computation in React.
3) If backend data is needed, plan a `#[tauri::command]` and call it with `invoke`.
4) Keep styles in `src/index.css` or component-scoped CSS.

Checks
- Ensure `invoke` names and payload shapes match Rust commands.
- Keep UI responsive; offload heavy work to Rust.
- Run `npm run dev` if asked to verify.

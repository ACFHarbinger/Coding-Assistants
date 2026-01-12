---
description: When building or releasing this Tauri + React app.
---

You are managing builds for a Tauri 2 + React app.

Workflow
1) Frontend build: `npm run build`.
2) Tauri app build: `npm run tauri build`.
3) Verify any platform-specific requirements if requested.

Checks
- Ensure dependencies are installed (`npm install`).
- Use Rust toolchain if building the Tauri bundle.

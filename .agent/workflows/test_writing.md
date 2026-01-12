---
description: When writing tests for this Tauri + React repository.
---

You are writing tests for the frontend or backend.

Workflow
1) Choose scope: React unit tests or Rust unit tests.
2) Favor behavior tests and error cases.
3) Keep tests isolated and deterministic.

Guidance
- Frontend: if tests are requested, prefer Vitest + React Testing Library.
- Backend: use `cargo test` in `src-tauri/`.

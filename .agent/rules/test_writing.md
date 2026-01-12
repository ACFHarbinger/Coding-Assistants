---
trigger: model_decision
description: When writing tests for the codebase.
---

You are an expert test writer for this Tauri + React repository.

Testing Principles
- Choose the smallest scope that proves behavior.
- Keep tests deterministic and isolated.
- Favor behavior over implementation details.

Repo-Specific Guidance
- Frontend: If tests are requested, prefer Vitest + React Testing Library.
- Backend: Use `cargo test` with unit tests in `src-tauri/src/`.
- IPC: Add tests for serialization shapes when changing command payloads.

Test Design
- Cover happy path and key error cases.
- Use clear, descriptive test names.
- Avoid snapshot-heavy UI tests unless requested.

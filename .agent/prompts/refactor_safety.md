# Prompt: Safe Refactor

Use when asked to refactor existing code (e.g. `AgentSystem`, `LlmClient`, the TCP server, or React components) without changing behavior.

---

1. Confirm test coverage exists for the current behavior before refactoring; add characterization tests first if it doesn't (see `.agent/rules/test_writing.md`).
2. Make the smallest change that achieves the refactor's stated goal — resist opportunistic rewrites of adjacent code.
3. Re-run `cargo test` (backend) or `npm test` (frontend) after each meaningful step, not just at the end.
4. For `src-tauri/src/agents.rs` or `llm_client.rs` changes specifically, verify the IPC payload shapes invoked from `src/` still match (see `.agent/rules/rust.md` and `AGENTS.md` IPC contract rules).
5. Call out any behavior change you couldn't avoid, however small, rather than letting it hide inside a "pure" refactor.

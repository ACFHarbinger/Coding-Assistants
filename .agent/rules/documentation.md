# Documentation Rules

- Every public function/struct in `src-tauri/src/` and exported component/hook in `src/` gets a doc comment (`///` in Rust, JSDoc in TS) explaining *why*, not a restatement of the signature.
- Update `docs/ARCHITECTURE.md` when module boundaries change (new Tauri command, new agent role, new IPC surface between `src/` and `src-tauri/`).
- Record significant, hard-to-reverse decisions (LLM provider adoption, IPC protocol changes, storage format choices) as a new ADR under `docs/adr/` — don't bury the rationale in a PR description that will get lost.
- Keep `docs/moon/CHANGELOG.md` updated for anything that changes the public surface (a new provider, a new agent capability, a new Android companion feature).
- README and `docs/index.md` should always reflect what actually exists in the repo today, not aspirational features — mark unfinished systems as `> **TODO:**` rather than describing them as done.

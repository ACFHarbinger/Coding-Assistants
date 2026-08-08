# Workflow: Documentation Update

1. Identify what changed: module boundary → `docs/ARCHITECTURE.md`; a hard-to-reverse decision → new `docs/adr/NNNN-*.md`; shipped work → `docs/moon/CHANGELOG.md`; planned work → the relevant `docs/moon/roadmaps/*.md`.
2. Keep `AGENTS.md` in sync if the change affects IPC contracts, Tauri command signatures, or the review severity protocol.
3. Verify code examples in docs actually compile/run against the current `src/`/`src-tauri/` code — don't let docs drift into aspirational pseudo-code without marking it `> **TODO:**`.
4. Run `mkdocs build --config-file docs/mkdocs.yml --strict` locally if you have the toolchain, to catch broken internal links before CI's `docs.yml` does.

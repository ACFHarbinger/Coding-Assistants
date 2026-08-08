# Documentation Standards

- **Docstrings/doc-comments**: TSDoc for TypeScript (`src/`), `///` doc-comments for Rust (`src-tauri/`), KDoc for Kotlin (`android/`). Every public symbol gets one.
- **Markdown docs** live under `docs/`; each page starts with a one-paragraph summary before any headings.
- **Diagrams**: prefer the C4 model via [Structurizr DSL](structurizr/workspace.dsl) over ad hoc images — it stays diffable and versioned.
- **Code examples** in docs must be runnable against the current codebase; stale examples are worse than no examples.
- **ADRs** record decisions, not designs-in-progress — write one only once a decision is made.
- **Inclusive language**: docs are checked in CI (see the markdown link-checker in `.pre-commit-config.yaml`); avoid ableist/exclusionary phrasing.

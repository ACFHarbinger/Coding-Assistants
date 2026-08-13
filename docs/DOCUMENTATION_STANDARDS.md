# Documentation Standards

- **Docstrings/doc-comments**: TSDoc for TypeScript (`src/`), `///` doc-comments for Rust (`src-tauri/`), KDoc for Kotlin (`android/`). Every public symbol gets one.
- **Markdown docs** live under `docs/`; each page starts with a one-paragraph summary before any headings.
- **Diagrams**: prefer the C4 model via [Structurizr DSL](structurizr/workspace.dsl) over ad hoc images — it stays diffable and versioned.
- **Code examples** in docs must be runnable against the current codebase; stale examples are worse than no examples.
- **ADRs** record decisions, not designs-in-progress — write one only once a decision is made.
- **Inclusive language**: docs are checked in CI (see the markdown link-checker in `.pre-commit-config.yaml`); avoid ableist/exclusionary phrasing.
- **Website source and build**: `docs/` Markdown is canonical. The public website is
  built from its explicitly curated corpus by `docs/website/scripts/build-content.ts`;
  do not edit generated files under `docs/website/src/content/`.
- **Website verification**: before changing published documentation, run `npm ci`,
  `npm test`, and `npm run build` from `docs/website`. The Pages workflow runs the
  same commands and deploys only `docs/website/dist` from `main`.
- **Pages cutover and rollback**: verify the deployed URL, a HashRouter refresh,
  search, theme selection, and a Mermaid page after each production deployment. If
  production verification fails, restore the last known-good `main` revision and
  redeploy it; retain the MkDocs-era sources until this React deployment is accepted.

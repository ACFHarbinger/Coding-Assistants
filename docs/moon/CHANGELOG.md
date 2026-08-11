# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **2026-08-11 memory/communication hub slice (M1–M5, C1–C4 partial):**
  - `ca-hub`: promote/compact/delete, purge-stale, age-out short-term; wake
    pending **dedup**; wake resolve; standing `WakePolicy` (human-gate defaults);
    message status updates; Markdown export includes handoffs.
  - `ca` CLI: `memory promote|delete|compact|purge-stale|age-out`,
    `msg status`, `wake resolve|policy`.
  - Tauri `hub_*` IPC + React **Shared Hub** panel (Memory/Inbox/Wakes) with
    Orchestrate|Hub tabs; same `$CA_HOME` / `~/.coding-assistants` store as CLI.
  - Roadmaps/changelog/crates README updated; unit tests for promote/compact,
    wake dedup, policy, and retention.
- Added the first executable M6 acceptance flow covering a durable handoff,
  provenance-linked memory, cross-agent inbox retrieval, wake resolution, and
  Markdown export; a real multi-agent repository run remains.
- **M3 done:** `HubStore::export_markdown_git` runs `git add` + `git commit`
  on the Markdown export when its directory is inside a Git work tree; outside
  a repo, with a failed `git add`, or with nothing to commit, it returns a
  `GitExportOutcome { committed: false, detail }` instead of erroring. Wired
  through `ca export-markdown --commit [--message ...]`, the Tauri command
  `hub_export_markdown_git`, and a desktop "Export MD + Commit" button. Covered
  by `m3_export_markdown_git_commits_inside_a_work_tree` (spins up a real
  temporary Git repo).
- PMF VS10 pivot recorded in the agent coordination bus; baseline frontend and
  Rust workspace checks passed before this implementation began.

- Hub spine crates (`crates/ca-hub`, `crates/ca-cli` binary `ca`): SQLite
  agents/memories/messages/wakes, private journals, wake JSON side-channel,
  Markdown export, CLI commands for init/memory/msg/wake/journal/export;
  unit test covering M1/C1–C3 smoke path.

- Replaced language-oriented roadmap files with capability roadmaps for memory,
  communication, UI, dashboards, platform, and infrastructure. Added a Mermaid
  Gantt index, made private agent journals part of the first memory milestone,
  retained LAN and Firebase prototyping, promoted A2A to the next major
  milestone, and removed obsolete deployment scaffolding. Deleted the duplicate
  `docs/ROADMAP.md` and the superseded per-language roadmap files.

- Reoriented the roadmap around the owner-confirmed product identity: a
  personal, local-first collaboration hub. Added the priority
  the initial shared memory and coordination roadmap for
  SQLite/Markdown hybrid memory, durable handoffs, CLI access, wake signals,
  configurable policies, and external-agent adapters. Folded the former root
  feature checklist into the moon roadmaps, kept `docs/ROADMAP.md` as a pointer
  stub at that stage, demoted TUI/3D/GraphQL-first/early actors to
  Someday/Maybe, and
  recorded provider, security, testing, and infrastructure-hygiene follow-up.

- Roadmap implementation, batch 2 (the former daemon-extraction spike):
  completed the daemon-extraction spike as [ADR 0003](../adr/0003-daemon-extraction-spike.md).
  Measured the actual `tauri::AppHandle` coupling across the backend
  (`file_tools.rs`: none; `agents.rs`/`llm_client.rs`: event emission only;
  `tcp_server.rs`: event emission + listening) and decided against
  extracting a separate daemon crate yet — recommends decoupling event
  emission into an internal broadcast channel first (tracked as new item
  `P1`), deferring the physical crate split until the API boundary is clear.
- Roadmap implementation, batch 1 (rate limiting, async file I/O, and shell
  audit): per-provider token-bucket rate limiting on
  outbound LLM calls (`governor`, burst 3 / 1 per second) in
  `llm_client.rs`; converted `FileTools` and the remaining Tauri-command
  file I/O to `tokio::fs` so it no longer blocks async worker threads;
  audited the tool layer for raw shell execution (none found — confirmed
  compliant with `AGENTS.md`'s Security Notes). Also removed `lib.rs`'s
  dead `start_remote_server` command (superseded by `TcpServer`, and it
  hardcoded a machine-specific absolute path that doesn't exist in this
  repo).

- Synced repo scaffolding/tooling from the Tauri-App-Template layout (excluding
  its backend/frontend/middleware/notebooks directories, which don't apply to
  this repo's existing `src/`/`src-tauri/`/`android/` layout): `.agent/`
  cross-agent delegation docs, `.devcontainer/`, `.forgejo/`/`.gitea/`/`.gitlab/`
  CI mirrors, expanded `.github/` automation, `git/` repo-process tooling
  (hooks, backlog sync, label taxonomy), the original `infra/` scaffolding,
  `tools/*/justfile` + root `justfile`, `settings/` editor configs, and
  `docs/` additions (ADRs, `docs/moon/`, Structurizr C4 model, `docs/website/`).
- Moved root-level docs (`ARCHITECTURE.md`, `DEPENDENCIES.md`, `DEVELOPMENT.md`,
  `TESTING.md`, `TROUBLESHOOTING.md`, `TUTORIAL.md`, `ROADMAP.md`,
  `SECURITY.md`, `CHANGELOG.md`) into `docs/`.
- Moved `codecov.yaml`/`CONTRIBUTING.md` from `.github/` into the new `git/`
  directory.
- Expanded the former per-area roadmaps with target-architecture work items
  synthesized from
  `docs/moon/research/Multi-Agent AI App Architecture.md` and
  `docs/moon/reports/AI Coding Tools Feature Report.md`: a headless Tokio
  actor-model daemon, a GraphQL-over-WebSockets API, MCP + A2A protocol
  support, rate limiting + affine-typed budget guardrails, two-tier
  persistent memory, human-in-the-loop security gates, a 2D telemetry
  dashboard, 3D force-graph visualization, and a new Ratatui TUI
  (now reorganized under the capability roadmaps). Added a capability-order
  index and Mermaid Gantt to `docs/moon/ROADMAP.md`.

## [0.1.0] — 2026-07-30

### Added

- Repository created from scratch.

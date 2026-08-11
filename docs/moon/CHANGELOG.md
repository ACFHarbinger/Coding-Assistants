# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Centralized Tauri invocation behind a runtime guard. Browser/Vite mode now
  reports a clear desktop-runtime requirement instead of throwing an undefined
  bridge error, and Tauri event listeners are skipped outside the desktop app.

### Added

- Reframed Orchestrate as a session team chat: **Execute Task** is now
  **Write Message**, **Launch Sequence** is **Send Message**, agent events are
  displayed as sender-attributed messages, and configured spawned agents have
  explicit **Add to team** controls. Enrolling a detected existing process now
  immediately adds its participant and a join message to the chat.

- Added the audit integrity MVP to `ca-hub` and `ca-cli`: recursive filesystem
  observation via `ca audit watch`, durable pending change records, owner
  approve/quarantine actions, and SHA-256 chain verification via `ca audit
  verify`. User-space observation records the watcher context and documents
  that originating external-writer PID attribution requires a privileged
  adapter.

- Added per-role existing-process endpoint configuration. When an
  OpenAI-compatible endpoint is supplied, orchestration sends requests to the
  already-running model service and does not spawn or terminate a child
  process; blank endpoints preserve the existing provider-managed behavior.
- Added a **Detect running agents** control to Orchestrate. It discovers local
  Grok, Claude, Codex/ChatGPT, and Gemini/Antigravity command processes and
  lets the user add selected identities to the configured team without taking
  ownership of or terminating those processes.
- Tightened process discovery to match executable basenames only, excluding
  desktop helpers, Chromium/Node utility services, and agent runtime helpers.
- Refined Gemini detection to recognize `agy` and the legacy `gemini` CLI while
  excluding the `antigravity` IDE executable.
- Improved maximized-window scrolling by removing permanent compositor-layer
  promotion from full-page panels and allowing offscreen sections to be skipped.
- Added a large-window performance profile that reduces full-surface gradients,
  card/button shadows, and header backdrop filtering while preserving layout and
  colors.
- Renamed the Shared Hub Budget tab to Usage and added per-agent used/available
  budget charts.
- Made startup resource discovery read-only so the default workspace is not
  created until the user explicitly initializes or runs a task.

- **Dashboard telemetry slice:** added persisted `agent_metrics` counters for
  provider calls, output lines/chars, estimated tokens used, and cached tokens;
  added Shared Hub Dashboard cards with per-agent budget progress. Exact
  provider token/cache/cost/latency reporting remains follow-up work.
- Dashboard now includes a collaboration overview sourced from existing task,
  message, and wake records, including pending-wake counts and recent tasks.

- **C6 done:** `agent_budgets` table + `HubStore::set_agent_budget` /
  `record_budget_usage` / `resume_agent` / `pause_for_budget`. Crossing a
  budget's `limit_units` flips `paused` (caller-defined units — call count,
  USD, tokens, whatever the caller tracks); `pause_for_budget` writes a
  Markdown handoff summary (objective/completed/missing) under
  `markdown/handoffs/`, sends a durable `Handoff` message to the delegate
  (default `"human"`), and `request_wake` then rejects the paused agent until
  a human calls `resume_agent`. Wired through `ca budget
  set|status|spend|pause|resume` and Tauri `hub_set_agent_budget` /
  `hub_get_budget` / `hub_record_budget_usage` / `hub_resume_agent` /
  `hub_pause_for_budget` (desktop UI wiring still open). Covered by
  `c6_budget_exhaustion_pauses_writes_handoff_and_blocks_wakes`.

- **2026-08-11 memory/communication hub slice (M1–M5, C1–C5):**
  - `ca-hub`: promote/compact/delete, purge-stale, age-out short-term; wake
    pending **dedup**; wake resolve; standing `WakePolicy` (human-gate defaults);
    message status updates; Markdown export includes handoffs;
    **`export_markdown_git`** (M3); **`tasks`** with sequential stages,
    bounded-parallel groups (`parallel_group` + `max_parallel`), and
    per-stage **retries** (`max_retries` / `retry_task`) (C5).
  - `ca` CLI: `memory promote|delete|compact|purge-stale|age-out`,
    `msg status`, `wake resolve|policy`, `export-markdown --commit`,
    `task create|advance|complete|retry|list|get|cancel`.
  - C6 budget controls: `ca budget set|status|spend|pause|resume`; exhausted
    agents are blocked from new wakes and produce durable Markdown handoffs.
  - Tauri `AgentSystem` now checks configured budgets before provider calls and
    records one call unit after successful completions, invoking the handoff
    boundary when a role exhausts its budget.
  - Active-run cancellation now records a durable shutdown handoff and
    delegation message before the Tauri task exits.
  - Shared Hub now includes a Usage tab for configuring limits, recording
    usage, inspecting paused agents, and resuming them.
  - C4 task-level `require_human_approval` is persisted and exposed through
    CLI/Tauri workflow creation, with coverage for ungated task wakes.
  - Added atomic pre-call budget reservation via `ca budget consume` and
    Tauri `hub_consume_budget`; over-limit provider calls are rejected before
    they start.
  - Tauri `hub_*` IPC + React **Shared Hub** panel; Orchestrate UI split into
    `ConfigPanel`/`ActivityPanel`/`RemotePanel`/`ApprovalPanel`.
  - Shared Hub **Policy** tab added for managing standing `WakePolicy` (human gate defaults);
    Wakes panel resolves pending wakes as delivered.
  - **C7 done:** Implemented A2A-compatible discovery and horizontal delegation. `AgentCard` schema and storage were added to `ca-hub`. `ca agent register-card` was added to `ca-cli`. The Tauri API exposes `hub_upsert_agent_card` and the TCP server now handles `GetAgentCards` payloads, enabling local workflows to interoperate with A2A peers.
  - **U3 done:** Implemented `update_memory` in `ca-hub` store and added inline editing
    along with color-coded scope indicators to the Shared Hub Memory tab.
  - **U2 done:** Added Task Browser tab to Shared Hub, allowing users to view task history,
    metadata, and message/handoff transcripts.
  - **U5 done:** Added DashboardScreen to Android app for viewing events and approving/rejecting wakes via TCP.
  - **U6 done:** Implemented Project Creation Wizard via a `bootstrap_workspace` Tauri command
    and a button in the ConfigPanel to initialize `.agent/` skeletons for new workspaces.
  - **C4 done:** Implemented per-task delegation policies via `require_human_approval` on
    `TaskRecord`, enabling configurability for automatic wakes during task dispatch, accessible
    through both the `ca-cli` (`--require-approval`) and the Tauri API (`CreateTaskArgs`).
  - **C6 done:** Exposed shutdown hooks via `ca shutdown` in the CLI and `hub_record_shutdown`
    in the Tauri API. This completes the budget exhaustion and shutdown delegation milestone,
    allowing external adapters to properly persist handoff states upon cancellation or limit reach.
  - Install: `just install-ca` / `~/.local/bin/ca` documented in `crates/README.md`.
  - Unit tests: promote/compact, wake dedup/policy, M3 git export, M6 handoff
    acceptance, and C5 sequential plus bounded-parallel/retry workflows.
  - **U1 done:** Refactored `App.tsx` into decoupled components (`ConfigPanel`,
    `ActivityPanel`, `RemotePanel`, `ApprovalPanel`) and overhauled the UI with
    a stunning glassmorphism design and micro-animations.
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

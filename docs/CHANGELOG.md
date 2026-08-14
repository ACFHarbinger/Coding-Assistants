# Changelog

[![Version](https://img.shields.io/badge/Version-0.1.0-orange)](../package.json)
[![License](https://img.shields.io/badge/License-AGPL--3.0-blue)](../LICENSE)

All notable changes to this project are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

- **Memory graph links + heuristic link-suggestion matcher (M7, #159)**: memories can now be linked to each other directly (`memory_links`, a directed edge with freeform `relation` and mandatory `created_by` provenance), not just to their originating source event — `link/unlink/links/related/topic` CLI/IPC commands, plus a dependency-free tag+token similarity matcher (`suggest-links`/`apply-suggestions`) that scores candidate connections and, under a new `off`/`suggest`/`auto` `LinkSuggestionMode` setting, can auto-draw high-confidence edges (attributed to the system, never an agent's own author field). See `docs/moon/CHANGELOG.md` for the full detail, including a real threshold recalibration from a smoke test. No frontend/UI wiring yet.
- **Codex bridge auto-registers a manually-started session (C14.8 follow-up)**: `deliver_codex_task` (`crates/hub/src/bridge/codex.rs`) already delivered into a Codex thread discovered via the on-disk fallback (`latest_codex_thread_id`) when no Hub registration existed, but never persisted that discovery — "Managed harness readiness" stayed empty and every later delivery re-scanned the whole `~/.codex/sessions` tree from scratch. The first successful delivery now calls `register_harness_session` to record it as **observed** (never managed — the Hub didn't spawn that process and must not claim ownership of it), so it becomes visible in the UI and later lookups hit the registration directly. `register_harness_session` unconditionally resets mode/writer/pid on conflict, so this is gated on there being no existing registration at all, to avoid silently downgrading an already-managed session. Regression test exercises the real fallback+auto-register path end-to-end (not just the inner `_from()` helper) by overriding `$HOME` to a temp `.codex/sessions/` tree.
- **Chat & Memory attachments**: paste an image from the clipboard directly into the composer textarea, or click the new **📎 Attach** button next to Send Message, to attach one or more files to a message. Attachments are stored as plain files under `<hub_home>/attachments/` and indexed in a new `attachments` table (`crates/hub/src/store/attachments.rs`; `hub_save_attachment`/`hub_get_attachment` Tauri commands, base64 over the IPC boundary), referenced from the message body via an `[attachment:<id>:<filename>]` token — the same embedded-reference pattern `[Memory #<id>]` already uses, so no schema change to `messages` or any existing `send_message`/`send_tagged_message` call site. The Chat & Memory stream renders images inline (click to download) and other files as a small download chip (`messager/attachments.tsx`). All harness dispatch is a plain CLI subprocess prompt rather than a multimodal API call, so task/wake delivery to a live work-session harness substitutes each token with the attachment's absolute on-disk path in the dispatched body only (`resolveDispatchBody`) — the stored/displayed message keeps the friendly token — letting each harness's own file-read/vision tool open it directly.
- **Tauri backend organization:** `src-tauri/src` is grouped into `agent`,
  `client`, `harness`, `hub`, `server`, `core`, and `main` modules without
  changing the public IPC command contract.
- **Hub command organization:** the Tauri Hub boundary is split into focused
  store, memory, messaging, workflow, quota, and test modules. All public IPC
  command names and payload contracts remain stable.
- **Crate naming:** workspace packages are `hub` and `cli`; the installed
  binary remains `ca`.
- **Hub-Native Multi-Agent Orchestration (U11–U12, C10–C12)**:
  - **Create & Load Team Chat (U11)**: Added `Create & Open` and `Load & Open` entry points in Orchestrate view to manage durable work sessions and switch focus to Chat & Memory.
  - **Recipient Addressing & Intent Tags (U12, C10, C11)**: Added Recipient Mode controls (`All Team`, `Subset`, `Single Agent`) and Intent Tag toggles (`⚡ [TASK]`, `🔔 [WAKE]`), enforcing task-refuse vs wake-enroll semantics with durable per-recipient `SendOutcome` records.
  - **Harness lifecycle and capture (C12)**: Implemented explicit-argument
    wake/start adapters for Grok (`grok`), OpenAI Codex (`codex`), Anthropic
    Claude Code (`claude`), and Google Antigravity CLI (`agy`), plus on-disk
    transcript capture. Task delivery uses only a provider-supported active
    bridge and otherwise remains durably queued.
  - **4-Harness Session Capture (C12)**: Added on-disk session transcript reverse-engineering for all four harness identities (`harness_grok.rs`, `harness_codex.rs`, `harness_claude.rs`, `harness_gemini.rs`), with SHA-256 content deduplication and active work session refresh polling.
  - **CLI Harness Capture & Tagged Dispatch (`cli`)**: Added `ca harness capture` for headless transcript capture and `ca msg tag --dispatch` for CLI-native tagged message injection.
- **Website Documentation Portal Sync (V1-DOCS-SYNC)**: Regenerated `docs/website/src/data/docs.json` for full capability roadmap, architecture, and changelog alignment.
- **Claude Channel: selective interruption and desktop connect/spawn (C14.3 follow-up, #150)**: only wake and task-tagged Hub messages are now pushed into a live Channel-connected session as an interruption; plain chat and handoffs stay durably queued and are surfaced through a new `check_inbox` MCP tool Claude can call on its own initiative, formatting and acking whatever quiet traffic is waiting (`hub::bridge::claude_channel::{poll_channel_events, poll_quiet_channel_events}`, `crates/claude`). The Shared Hub → Channels tab now shows a live connected/not-connected status per configured workspace (`claude_channel_is_connected` / `hub::is_channel_session_live`, a process-table check for the bridge process) and a **Connect** button that opens a real terminal running `claude --dangerously-load-development-channels server:coding-assistants-channel` when none is connected (`claude_channel_connect` / `hub::launch_claude_channel_session`) — Claude Code's Channel research preview has no headless daemon mode, so this always spawns a real terminal emulator rather than a detached background process, unlike the Codex/Gemini managed-session adapters. Live acceptance against a real `claude --channels` session is now verified (ping/wake/task round-trip).
- **Chat read receipts**: a durable `read_markers` table tracks how far each team member has read into a chat scope (channel, work session, or DM pairing). `HubStore::mark_read`/`list_read_markers`, matching `hub_mark_read`/`hub_list_read_markers` Tauri commands, and a `ca msg read`/`ca msg readers` CLI pair. The desktop Chat & Memory view marks the human's own view automatically and shows a "✓✓ Read by ..." marker under each message once another team member's marker has caught up to it; Claude's `reply` tool auto-marks itself read for the session it just replied in.
- **Chat & Memory sort order toggle**: a header button next to the search box lets the human dev flip the active channel/DM/work-session stream between descending (newest first, default) and ascending (oldest first) date-time order (`MessagerPanel.tsx`'s `sortOrder` state, rendered in `messager/ChatHeader.tsx`). Messages are sorted by an actual parsed `created_at` comparison (`messager/utils.ts`'s `sortByCreatedAt`, stable on ties) rather than assumed pre-existing array order — the Hub's own message queries return rows `ORDER BY created_at DESC` (newest first), so a naive "reverse if descending" toggle silently inverted both orderings from what their labels claimed. Auto-scroll, the "jump to latest" affordance, and the near-edge scroll tracking all follow whichever end of the list is currently newest (`isNearNewestEdge`/`newestEdgeScrollTop`). Clicking the toggle itself now lands on the literal first message of the newly chosen order (`jumpToStartRef`) — oldest-first genuinely opens at the oldest message, not always the newest one re-pinned to the opposite edge, which is what made the two orderings look identical before. Purely a client-side view toggle — no backend or storage change.
- **Provider bridge audit (C14.6–C14.8, #154, #155, #156)**: diagnosed why Hub-delivered messages weren't reaching live sessions for Grok, Gemini/Antigravity (`agy`), and Codex, using the same scrutiny applied while building the Claude Channel. Grok's leader-mode ACP delivery path is implemented correctly but needs a running `--leader` process (#154, documentation/UX gap, not a code bug). Gemini/agy's managed-worker delivery passes the prompt as `--prompt <text>`, but `agy --help` documents `--prompt` as a bare `--print` alias — the real prompt is silently dropped, producing off-topic replies (#155, real bug, root-caused, not yet fixed). Codex delivery likely fails silently for a live session the user started by hand because nothing auto-registers it with the Hub, and even a resolved thread is only ever turned through a disposable headless `app-server` client, never the visible TUI (#156). Task breakdowns for each provider's own agent are in `.agent/cache/AGENT_BUS.md` and the linked issues; no fixes were made to any of the three bridges in this round.
- **Grok leader channel (C14.6 / #154)**: Hub inject still uses documented ACP (`grok agent --leader stdio`). Shared Hub → Channels and Orchestrate can start `grok agent leader` and open `grok --leader` when `~/.grok/leader.sock` is missing. Standalone TUIs stay capture-only. A Hub task ping reached a live Grok session.
- **Source size refactor, Grok's frontend slice (I8 / #158, #152)**: `ConfigPanel.tsx` and `MessagerPanel.tsx` split under 500 LoC; Channels UI is `hub/ChannelsTab.tsx`. Fabricated `managed-<pid>` ids are rejected.
- **Source size refactor, Claude's slice (I8 / #158)**: `crates/hub/src/bridge/claude_channel.rs` (1,069 LoC) split into `hub::bridge::channels::claude::{workspaces,events,reply,permissions,terminal}` (largest file 394 LoC); `crates/claude/src/main.rs` (613 LoC) split into a thin `main.rs` entry point plus `main/{cli,protocol,server}.rs` (largest 307 LoC, using `#[path]` module attributes since a binary crate root can't resolve submodules into a same-named directory implicitly); `src/components/settings/SettingsApp.tsx` (812 LoC) split by tab into `settings/tabs/{shared,GeneralTab,WorkspaceTab,MemoryTab,OrchestrationTab}.tsx` (largest 242 LoC), `SettingsApp.tsx` itself now 457. Every touched file is ≤500 LoC. Public API (`hub::{poll_channel_events, ...}` crate-root re-exports), the MCP protocol surface, CLI subcommands, and Settings UI/state behavior are all unchanged — only module boundaries moved. Added a few module-boundary tests (`terminal_exec_prefix_*`, `handle_request_initialize_declares_both_channel_capabilities`, `handle_request_records_a_permission_request_exactly_once`).

### Fixed

- **Blank white main window:** `harness/types.ts` was accidentally overwritten with Orchestrate config types, so Vite failed to load harness exports and React never mounted. Restored the harness session/delivery types.
- **Orchestrate persistence and process discovery:** the selected workspace
  root and persisted team roster survive restart. The discovery action now
  toggles between **Detect running agents** and purple **Hide detected
  agents**, making its visibility state clear and avoiding any implication
  that a discovered terminal process is automatically controllable.
- **Tagged harness-delivery failure reporting (C12)**: A failed Tauri harness
  injection no longer aborts the entire recipient batch and masquerades as a
  generic send failure. Chat & Memory now retains the durable post and lists
  every rejected or unavailable target in the owner-facing delivery alert.
- **Session-message IPC serialization (C10–C12)**: Tauri now accepts the
  camelCase nested payload used by Chat & Memory for tagged and ordinary
  work-session sends (`isTask`, `isWake`, and `sessionId`), preventing tagged
  sends from failing before delivery with a missing `is_task` argument.
- **Standalone Settings window reliability (regression on S3 / #129, #153)**:
  the Settings window now reliably opens (explicit focus on first creation),
  closes (`core:window:allow-close` was missing from the app's capability,
  so the in-window Close button's `close()` call was silently rejected by
  Tauri's permission system while the OS window-manager `X` bypassed it), and
  reopens repeatedly within the same app run without requiring a restart
  (Settings is no longer hidden-and-kept-alive like the tray-resident main
  window). Also replaced a panicking `.unwrap()` in the shared window-close
  handler that could take down the whole app.
- **Claude's chat replies disappearing on every new message (#150 follow-up)**:
  `record_channel_reply` gave every reply in the same session the identical,
  non-unique subject `channel:session:<id>:reply`. The desktop Chat & Memory
  view's per-post dedup (meant to collapse team fan-out *copies* of one
  broadcast post, not distinct sends) treated every reply as the same post
  and kept only the most recent, so each earlier reply from Claude vanished
  the instant a new one arrived. Each reply's subject is now uuid-suffixed,
  matching the pattern `send_session_message` already uses by default.
- **Any agent's chat capture overwriting another agent's, in the same session (#150 follow-up, same bug class as above)**: `record_harness_capture` — the C12 poller that pulls each harness's own on-disk transcript into the Hub for Grok/Chat-Codex/Claude/Gemini alike — gave every captured chunk in a session the identical, non-unique subject `channel:session:<id>:capture` regardless of which agent authored it. The same per-post dedup collapse applied across agents, not just within one: a fresh capture from any agent made the desktop chat appear to silently overwrite whatever the *previous* capture (from any other agent) had shown. Each capture's subject is now uuid-suffixed the same way.

---

## [0.1.0] - 2026-01-31 (Current)

### Added

- Shared Hub implementation for local-first memory and communication: durable
  memory promotion/deletion/compaction, inbox polling, deduplicated wake
  requests, Tauri commands, and desktop Shared Hub navigation.

- Shared Hub lifecycle controls: short-term age-out and stale-memory purge,
  message/wake status resolution, and persisted wake human-gate/auto-wake
  policy exposed through the `ca` CLI and Tauri commands.

- M6 acceptance coverage for cross-agent handoffs: provenance is now exposed
  on `MemoryRecord`, source-aware writes are supported, and an integration
  test verifies handoff retrieval, wake deduplication/resolution, and Markdown
  export together.

- Shared Hub Wakes panel now exposes persisted wake policy controls and a
  pending-wake delivery action, completing the desktop side of C4's standing
  policy boundary.

- C5 workflow orchestration now supports bounded parallel stages, queued agent
  release, retry limits, failed terminal state, CLI/Tauri commands, and Shared
  Hub task controls.

- C6 now provides per-agent budget tracking, exhaustion pause, durable Markdown
  delegation handoffs, wake blocking, and human-controlled resume through the
  CLI/Tauri Hub boundary. Automatic provider spend reporting remains open.

- Tauri agent execution now enforces configured budgets before provider calls
  and records one call unit after successful completions, triggering the
  durable exhaustion handoff when a role reaches its limit.

- Cancelled Tauri runs now write a reviewable shutdown handoff under the shared
  Markdown handoff directory and emit a durable delegation message.

- Shared Hub now exposes a Budget tab for configuring per-agent limits,
  recording caller-defined usage, viewing pause state, and resuming agents.

- Added an atomic pre-provider budget reservation command so external adapters
  can reject over-limit calls before invocation, matching Tauri enforcement.

- C4 task-level delegation approval is now persisted and exposed through CLI
  and Tauri workflow creation, with a regression test for ungated task wakes.

- Fixed browser-mode startup errors by guarding Tauri invocation/event APIs and
  reporting when a command requires the desktop runtime.

- **AGPL-3.0 license** for open-source distribution (`1b580e3`)
- **Local model serving** via Ollama and file-based memory persistence (`8a098ec`)
- **Android companion app updates** with improved UI and stability (`2c3d4ff`)
- **Cargo workspace** with root-level `Cargo.toml` and updated `.gitignore` (`dc5ec99`)
- **Android remote control app** with TCP/IP connectivity to the desktop app (`0bd5e73`)
  - Kotlin + Jetpack Compose + Material 3
  - WiFi-based connection to desktop TCP server on port 5555
  - Model browsing, task submission, and real-time event monitoring
- **Dynamic role management** -- add and remove an arbitrary number of agent roles (`75be40a`)
- **Inter-agent communication** via `[[ASK_AGENT:RoleName]]` markers with authorization modal (`bf42181`)
- **User-in-the-loop interaction** via `[[ASK_USER]]` markers with input modal (`b19a8ee`)
- **Task cancellation on app close** -- agents are cancelled when the window is closed (`ad9bd8e`)
- **MCP server support** -- configure Model Context Protocol servers (sequential-thinking, filesystem, memory) (`9c013e7`)
- **Markdown report generation** -- agents produce a project memory file at end of task (`d8fa18b`)
- **Agent activity viewer** -- real-time event log with colored badges per agent role (`e5e29df`)
- **Agent resource system** -- `.agent/` directory with prompts, rules, and workflows (`af3e681`, `c1d16f7`, `678a3f3`, `f71afc9`)
  - Prompts: system instructions per role
  - Rules: constraints and guidelines
  - Workflows: step-by-step procedures
  - Resource preview modal
- **Governance documentation** -- AGENTS.md and supporting project docs (`4e7eb54`)
- **Reviewer role** and workspace directory browser button (`688f6eb`)
- **Provider and model dropdowns** -- dynamic UI for selecting LLM providers and their models (`df94f84`)

### Changed

- **Complete rewrite** from previous Python-based architecture to Tauri 2 + Rust + React 19 + TypeScript (`db5066d`)
  - Frontend: React 19 with glass-morphism dark theme
  - Backend: Rust with Tokio async runtime
  - IPC: Tauri command/event system replacing previous approach
  - Build: Vite 7 for frontend, Cargo for backend

### Fixed

- **App styling** -- resolved CSS issues after the Tauri rewrite (`6bee41d`)
- **Model naming** -- corrected model name display and selection (`50721ff`)

---

## [0.0.1] - 2025-12-13 (Pre-Tauri)

Initial Python-based prototype with GUI.

### Added

- **Base code** with initial agent orchestration logic (`dd366d3`)
- **File system tools** with tests (`e264110`)
- **Argument parser and GUI** for desktop interaction (`bd583bb`)
- **Multi-model role selection** -- reviewer role with ability to assign different models per role (`c8ffecb`)
- **GUI tests** (`eb27d36`)

---

## Version History Summary

| Version | Date       | Milestone                                         |
| ------- | ---------- | ------------------------------------------------- |
| 0.1.0   | 2026-01-31 | Tauri 2 rewrite, multi-agent orchestration, Android app, MCP, remote control |
| 0.0.1   | 2025-12-13 | Initial prototype with Python GUI                 |

---

## Versioning

This project uses [Semantic Versioning](https://semver.org/):

- **MAJOR**: Incompatible changes to the agent system, IPC contract, or configuration format
- **MINOR**: New features (providers, agent capabilities, UI components) in a backwards-compatible manner
- **PATCH**: Bug fixes and documentation updates

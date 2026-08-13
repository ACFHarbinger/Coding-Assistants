# Changelog

[![Version](https://img.shields.io/badge/Version-0.1.0-orange)](package.json)
[![License](https://img.shields.io/badge/License-AGPL--3.0-blue)](LICENSE)

All notable changes to this project are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

- **Hub-Native Multi-Agent Orchestration (U11–U12, C10–C12)**:
  - **Create & Load Team Chat (U11)**: Added `Create & Open` and `Load & Open` entry points in Orchestrate view to manage durable work sessions and switch focus to Chat & Memory.
  - **Recipient Addressing & Intent Tags (U12, C10, C11)**: Added Recipient Mode controls (`All Team`, `Subset`, `Single Agent`) and Intent Tag toggles (`⚡ [TASK]`, `🔔 [WAKE]`), enforcing task-refuse vs wake-enroll semantics with durable per-recipient `SendOutcome` records.
  - **Bidirectional Harness Adapters (C12)**: Implemented explicit argv start and inject adapters for Grok (`grok --cwd`), OpenAI Codex (`codex exec --cwd`), Anthropic Claude Code (`claude -p`), and Google Antigravity CLI (`agy --cwd`).
  - **4-Harness Session Capture (C12)**: Added on-disk session transcript reverse-engineering for all four harness identities (`harness_grok.rs`, `harness_codex.rs`, `harness_claude.rs`, `harness_gemini.rs`), with SHA-256 content deduplication and active work session refresh polling.
  - **CLI Harness Capture & Tagged Dispatch (`ca-cli`)**: Added `ca harness capture` for headless transcript capture and `ca msg tag --dispatch` for CLI-native tagged message injection.
- **Website Documentation Portal Sync (V1-DOCS-SYNC)**: Regenerated `docs/website/src/data/docs.json` for full capability roadmap, architecture, and changelog alignment.

### Fixed

- **Tagged harness-delivery failure reporting (C12)**: A failed Tauri harness
  injection no longer aborts the entire recipient batch and masquerades as a
  generic send failure. Chat & Memory now retains the durable post and lists
  every rejected or unavailable target in the owner-facing delivery alert.

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

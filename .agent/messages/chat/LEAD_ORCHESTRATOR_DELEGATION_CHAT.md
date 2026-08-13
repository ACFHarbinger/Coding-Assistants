# Lead Orchestration Delegation — Gemini → Chat/Codex

> **Date:** 2026-08-12
> **From:** Gemini 3.6 (Lead Orchestrator)
> **To:** Chat/Codex
> **Target:** `Coding-Assistants` (`/home/pkhunter/Repositories/Repos/Coding-Assistants`)
> **Project:** GitHub Project #21 (Messager-like Chat & Agentic Memory Hub)

---

## Assigned Task: CA-102 — Channel & Message Query Extensions

### Objective
Extend `hub` store (`crates/hub/src/store.rs`) and Tauri IPC commands (`src-tauri/src/hub_cmds.rs`) to support channel-filtered message listing, message tags, and memory cross-referencing.

### Key Requirements
1. Support filtering messages by channel subject prefix (e.g., `channel:general`, `channel:team-coordination`, `channel:agent-memory`).
2. Add helper method `HubStore::list_channel_messages(channel, limit)` and corresponding Tauri IPC command `hub_list_channel_messages`.
3. Support embedding and parsing `[Memory #id]` tags in message bodies.

### Coordination Protocol
- Log status and task claims on `.agent/cache/AGENT_BUS.md`.
- Ensure all tests pass with `cargo test --workspace`.

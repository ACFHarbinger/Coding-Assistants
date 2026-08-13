# Lead Orchestration Delegation — Gemini → Grok

> **Date:** 2026-08-12
> **From:** Gemini 3.6 (Lead Orchestrator)
> **To:** Grok (Build / xAI)
> **Target:** `Coding-Assistants` (`/home/pkhunter/Repositories/Repos/Coding-Assistants`)
> **Project:** GitHub Project #21 (Messager-like Chat & Agentic Memory Hub)

---

## Assigned Task: CA-104 — Process Heartbeat & Telemetry Bridge

### Objective
Wire process detector (`detect_agent_processes` Tauri command) into the Messager-like channel sidebar and agent roster to display real-time active agent process status (`ONLINE` / `IDLE` / `OFFLINE`) for Grok, Claude, Codex, and Gemini.

### Key Requirements
1. Use exact executable basename detection (excluding helpers/utilities).
2. Surface heartbeat indicator pills (green dot = running, amber = idle/background, gray = offline) in the UI sidebar.
3. Ensure zero false positives and zero UI thread blocking.

### Coordination Protocol
- Log status and task claims on `.agent/cache/AGENT_BUS.md`.
- Keep changes isolated to process telemetry and UI status hooks.

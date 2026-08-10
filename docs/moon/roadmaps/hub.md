# Cross-Agent Shared Memory & Coordination Roadmap

> **Top-priority product track** (owner Q&A 2026-08-10).  
> CA is a **collaboration hub** for external coding agents (Claude Code, Codex,
> Gemini/Antigravity, Grok Build, OpenCode, Ollama, llama.cpp) and a human
> developer — not only an in-process multi-role pipeline.  
> This track sits **above** the Core Orchestration Daemon track in
> [`../ROADMAP.md`](../ROADMAP.md). It **subsumes / re-frames** single-agent
> memory items RP1–RP4 in [`rust.md`](rust.md) into a multi-agent, multi-scope
> design.

Status markers: ✅ Done · 🚧 In Progress · 📋 Pending · 💤 Someday/Maybe

## Product constraints (owner-locked)

| Constraint | Value |
| --- | --- |
| Audience (v1) | Solo power developer (personal tool) |
| Collab mode first | Async mailbox, then parallel |
| Memory model | Hybrid: local DB + git-tracked markdown |
| Scopes | Both global (cross-repo) and per-workspace |
| Tiers | Recent rich + long-term compressed |
| Wake vs durable | Distinct mechanisms (ephemeral wake ≠ durable store) |
| Human gate | Configurable per session/task |
| Inter-agent auth | Standing policies preferred |
| Path conventions | `.agent/reports/` + messages are temporary |
| 30-day success | Joint quality on a real repo (e.g. Project-Mobile-Fortress) ≥ any single teammate |

---

## Track: Durable Store (SQLite + Markdown)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RH1 | SQLite schema for messages, decisions, transcripts, memory entries, agent identity/attribution | M | 📋 Pending |
| RH2 | Dual scope: `workspace_id` + optional `global` scope for cross-repo patterns | M | 📋 Pending |
| RH3 | Multi-tier retention: recent unfiltered window; periodic compaction into compressed long-term memories | L | 📋 Pending |
| RH4 | Git-tracked markdown tier for high-priority near-term items and high-importance decisions (export/sync from DB) | M | 📋 Pending |
| RH5 | Automatic capture hooks: decisions, errors, preferences, conversation segments; file changes via Git | M | 📋 Pending |
| RH6 | Human review UI/CLI for edit, augment, and delete/stale memories | M | 📋 Pending |
| RH7 | Optional automatic write-conflict detection when two agents touch the same path (user setting) | M | 📋 Pending |

---

## Track: Shared CLI Helper

Any of the four assistants’ tool-calling loops can invoke a tiny helper without
depending on the Tauri GUI process.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RH10 | `ca` (or `coding-assistants`) CLI: `read` / `write` / `poll` / `search` against the durable store | M | 📋 Pending |
| RH11 | Agent identity header on every write (who, tool, session, timestamp, workspace) | S | 📋 Pending |
| RH12 | Transcript append with configurable TTL (durable for a limited time by default) | S | 📋 Pending |
| RH13 | Optional MCP tools wrapping the same operations (later; not blocking RH10) | M | 📋 Pending · later |

---

## Track: Wake Signal (Ephemeral)

Decoupled from the durable store so storage remains correct even if no agent is
running.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RH20 | Wake channel via file-watch and/or local Unix socket notify | M | 📋 Pending |
| RH21 | Per-session/task setting: require human gate before wake, or allow agent-to-agent wake | M | 📋 Pending |
| RH22 | Standing policies (e.g. “Claude may always ask Grok about Rust”) as alternative to per-message modals | M | 📋 Pending |
| RH23 | On budget exhaustion (see `rust.md` RB3): pause, write persistent markdown summary (objective, done, missing), delegate, shutdown until human wake | M | 📋 Pending |

---

## Track: Collaboration Topologies (phased)

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RH30 | Declarative role wiring (sequential / bounded parallel discussion with clear task boundaries) | M | 📋 Pending |
| RH31 | Agents continue work while human is away (within policy/budget) | M | 📋 Pending |
| RH32 | Full open parallel execution from session start | L | 📋 Pending · after RH30 |
| RH33 | Dynamic A2A-style discovery/delegation | XL | 💤 Someday/Maybe |

---

## Track: External Agent Adapters (v1 must-integrate)

Harness choice (direct CLI vs OpenCode router vs HTTP) is agent-owned; track
outcomes, not a single vendor lock-in.

| # | Item | Effort | Status |
| --- | --- | --- | --- |
| RH40 | Adapter interface: start / message / cancel / status / optional session-resume | L | 📋 Pending |
| RH41 | Claude Code adapter (prefer headless/stream-json when available) | M | 📋 Pending |
| RH42 | Codex CLI adapter | M | 📋 Pending |
| RH43 | Gemini / Antigravity adapter | M | 📋 Pending |
| RH44 | Grok Build adapter | M | 📋 Pending |
| RH45 | OpenCode adapter | M | 📋 Pending |
| RH46 | Ollama adapter | S | 📋 Pending |
| RH47 | llama.cpp adapter | M | 📋 Pending |
| RH48 | Session resume vs context-injection: implement hybrid (durable transcript always; resume IDs when provider supports) | L | 📋 Pending · design OPEN |

---

## Relationship to RP1–RP4

| Old item | Disposition |
| --- | --- |
| RP1 briefing | Becomes markdown/DB **workspace briefing** under RH3–RH4 |
| RP2 deep store | Replaced by SQLite **RH1–RH2** (not only JSON file) |
| RP3 decay | Maps to multi-tier **RH3** |
| RP4 dedup | Maps into compaction pipeline **RH3** |

Mark RP1–RP4 in [`rust.md`](rust.md) as **superseded by hub track** once RH1 lands.

---

## Acceptance cadence (owner)

Not every row needs a metric, but **at least every ~5 items** should ship with
an acceptance test or measurable exit criterion (owner Q&A Chat R.29).

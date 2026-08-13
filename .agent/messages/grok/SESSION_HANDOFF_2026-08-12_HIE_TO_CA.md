# Grok session handoff — HIE work → Coding-Assistants wake

> **Written:** 2026-08-12T21:55:02Z
> **Author:** Grok (Build / xAI)
> **From workspace:** Image-Toolkit + `submodules/HIE`
> **Next workspace:** `/home/pkhunter/Repositories/Repos/Coding-Assistants`
> **Why this file exists:** Harbinger is clearing the Image-Toolkit session and opening a new session on Coding-Assistants. This is the continuity packet for the next Grok instance.

---

## 0. First actions on wake (Coding-Assistants)

1. **Address the user as Harbinger / ACFHarbinger** (never “human owner”).
2. Read this file, then:
   - `AGENTS.md` / `CLAUDE.md` / `GEMINI.md` in the CA repo root
   - `docs/moon/ROADMAP.md` + capability files under `docs/moon/roadmaps/`
   - `~/.coding-assistants/journals/grok/journal.md` (private continuity)
   - Peer journals if needed: `~/.coding-assistants/journals/{chat,claude,gemini}/`
3. **Inspect the worktree before editing:** `git status`, `git log -15`, concurrent agent dirt.
4. Prefer **small, evidence-backed** changes; update changelog + roadmap for substantive work; commit with `Co-authored-by: Grok <grok@x.ai>` when appropriate.
5. Multi-agent bus (if active): `.agent/cache/AGENT_BUS.md` and related cache files — claim before overlapping work.

---

## 1. Who Harbinger is and how collaboration works

| Rule | Detail |
| --- | --- |
| Name | **Harbinger** / **ACFHarbinger** |
| Authority | Owner decisions bind; agents recommend and implement |
| Evidence | Screenshots, live repro, tests > confident static reasoning |
| Multi-agent | Independent reports first → shared synthesis → owner GO |
| Write confinement | Stage only your files; re-read before edit; never clobber peer dirty trees |
| Docs | Substantive work → `docs/moon/CHANGELOG.md` + relevant roadmap |
| GitHub | Prefer `gh` yourself; if mutation fails, supply exact Markdown for paste |
| Secrets | Never hardcode credentials; journals/memory helpers only under `~/.coding-assistants/code/` |

### Durable product lesson (from CA multi-agent experiment 2026-08-10)

**Memory-first hub** beats orchestration theatre. Product center of gravity:

1. Durable hybrid memory (SQLite + git-tracked Markdown for high-priority notes)
2. Discoverable single communication bus + presence
3. Explicit human gates / standing policies
4. Provenance / audit

A2A is the **next major milestone after the hub spine**, not before. GraphQL / actors / 3D / TUI are later or research.

---

## 2. Workflow learned this session (Image-Toolkit / HIE) — keep for all repos

### 2.1 Onboarding pattern

When a submodule/repo leaves an `ONBOARDING.md` under `.agent/cache/<agent>/`:

1. Read it fully before coding.
2. Read peer journals under `~/.coding-assistants/journals/`.
3. Write/refresh **your** journal under `~/.coding-assistants/journals/grok/`.
4. Post presence + claim on `AGENT_BUS.md` (or CA equivalent).
5. **Ask clarifying questions** if ownership/priority is ambiguous — Harbinger answers concretely.

### 2.2 Concurrent multi-agent work

- **Never stage another agent’s dirty files** (this session: Claude’s `uv.lock` / jobs / package-flatten WIP).
- If you accidentally reverse a peer’s intentional restructure, **put it back** and say so on the bus.
- Package-layout ownership: Claude was flattening `hie_middleware` → `middleware/src/*` and planned `hie_gui` → `gui/src/*`. Grok’s IPC used **flat imports** (`from document import`, `from pipeline import`). Same lesson for CA: if a peer owns a restructure, adapt to their layout.

### 2.3 Submodule UI ownership pattern (portable)

When a feature lives in a submodule:

- **Source of truth** = submodule UI packages.
- **Parent** = thin re-exports only.
- ASP/CSG already do this (`asp_gui`, `csg_gui` via `git/scripts/_submodule_bootstrap.py`).
- HIE now does this for Hybrid Editor (see §3).

### 2.4 Definition of done

Tests green → changelog → roadmap note → commit (and issue comments). Push only when asked or when the standing task says push.

---

## 3. What Grok actually shipped this session (context for CA continuity)

**Workspace:** Image-Toolkit + HIE submodule (not Coding-Assistants code).

| Deliverable | Location / commits |
| --- | --- |
| Private journal filled from past sessions | `~/.coding-assistants/journals/grok/journal.md` (+ README) |
| Hybrid Editor UI ownership in HIE | PySide6 `HieTab`/`HieEditorTab`; React `frontend/src/embed/react/` |
| Parent thin re-exports | IT `gui/src/tabs/editor/`, `frontend` + `hie-frontend` file dep |
| Pipeline/IPC | `PipelineSession` in GUI; IPC methods `list_capabilities`, `preview_policy`, `accept_proposal`, `submit_restoration` |
| Docs | HIE changelog/roadmap/Track 04; IT S377 + master roadmap HIE note |
| GitHub | Comments on HIE #8, #7; parent #363 |

**Key commits (may advance after this handoff):**

- HIE: `ae052e8` (feature), `1020d93` (docs)
- Image-Toolkit: `a672ba46` (re-exports), `09e90011` (docs)

**Tests at land:** middleware 103 passed / 23 skipped; gui 5 passed.

**Not pushed** in that session unless Harbinger requested later.

---

## 4. Coding-Assistants repo — readiness brief

### 4.1 Paths

| Path | Role |
| --- | --- |
| `/home/pkhunter/Repositories/Repos/Coding-Assistants` | Product repo (Tauri + React + Rust + Android) |
| `crates/hub`, `crates/cli` | Hub spine CLI / SQLite store (Plan Alpha) |
| `src/` | React 19 UI |
| `src-tauri/` | Tauri 2 backend (orchestration, TCP, tools) |
| `android/` | Remote companion (approvals / monitoring) |
| `docs/moon/ROADMAP.md` | Canonical priority index |
| `docs/moon/roadmaps/{memory,communication,platform,ui,dashboard,infrastructure}.md` | Capability plans |
| `.agent/cache/` | Multi-agent buses, merge experiments, presence |
| `.agent/messages/grok/` | **This handoff directory** |
| `.agent/messages/shared/` | Shared subagent delegation notes (moved from flat messages) |
| `~/.coding-assistants/` | Runtime hub data: `hub.db`, journals, wakes, `code/journal_crypto.py` |

### 4.2 Product identity (DECIDED 2026-08-10)

Coding-Assistants is a **local-first collaboration hub** for Harbinger + external agents (Claude Code, Codex, Gemini/Antigravity, Grok Build, OpenCode, Ollama, llama.cpp). The sequential multi-role in-app pipeline was an **experiment only**.

**V1 success:** joint work on a real repo (e.g. Project-Mobile-Fortress) quality ≥ best single teammate.

**Priority order (roadmap v2):**

1. Memory + private journals
2. Communication / async coordination
3. Platform reliability, providers, tools, security
4. Desktop UI + 2D dashboard
5. A2A interoperability
6. Daemon/multi-client when justified
7. Android monitoring/approvals
8. Someday: TUI, 3D, GraphQL, actors

### 4.3 Observed git state when this handoff was written

```
branch: main, ahead of origin by ~11 commits (local unpushed work may exist)
recent tips include: usage quota plots, private reply reveal, codex bridge/inbox adapters,
  team/private messaging, orchestrate hub routing, audit ledger, hub UI/usage charts
dirty (do not stomp without checking):
  D  .agent/messages/*_subagent_delegation.md   # moved under messages/shared/
  D  tools/codex-harness-adapter                # may live under scripts/codex-harness-adapter
  ?? .agent/messages/gemini/ , shared/ , scripts/codex-harness-adapter
```

**On wake:** re-run `git status` / `git log`; concurrent agents may have advanced main.

### 4.4 Architecture reminders (CA-specific)

- UI in `src/`; system I/O, LLM, networking in `src-tauri/`.
- IPC: every `invoke` needs a matching `#[tauri::command]`; keep serde types aligned.
- **Read-only discovery must not create workspace state** (past bug: `get_agent_resources` mkdir).
- Process discovery: match **executable basenames**, not full command lines; distinguish legacy `gemini` / `agy` from Antigravity proper.
- Private hub messages: messages addressed **to Harbinger** must be readable in UI; privacy defaults need an explicit self-exception.
- Global `AppState` / single Mutex: concurrent agent tasks can clobber cancel/input channels — fix when touching orchestration.
- Audit ledger (`ca audit`): head-read + insert in one SQLite transaction to avoid forked chains.

### 4.5 Useful commands (verify against justfile/README on wake)

```bash
cd /home/pkhunter/Repositories/Repos/Coding-Assistants
# Hub CLI (after workspace build)
cargo test -p hub
cargo run -p cli -- --help
# Desktop
npm install   # if needed
# Tauri dev — check package.json / justfile for current recipe
just --list   # if justfile present
cargo check --workspace
```

### 4.6 Shared runtime hub (outside the git repo)

| Path | Purpose |
| --- | --- |
| `~/.coding-assistants/hub.db` | SQLite hub (agents, memories, messages, wakes, audit) |
| `~/.coding-assistants/journals/grok/` | **Your** journal — append after meaningful sessions |
| `~/.coding-assistants/code/` | **Only** place for journal/memory helper executables |
| `~/.coding-assistants/wake/` | Wake request side-channel files |
| `~/.coding-assistants/mcp.json` | MCP config (historically racy if multi-writer) |

Journal encryption (optional): Fernet blocks in `<!--ENC-->…<!--/ENC-->`; helper pattern in Claude’s README; code only under `code/`.

---

## 5. Suggested next work on Coding-Assistants (if Harbinger has no override)

These are **suggestions**, not claims. Confirm on bus / with Harbinger before starting:

1. **Hub spine hardening** — `hub` / `cli` completeness vs README; smoke every subcommand; no dead modules.
2. **Memory gate** — two agents retrieve a prior handoff on a real repo task; shared vs private separation.
3. **Orchestrate / messaging** — team vs private delivery, wake policies, human gates; fix remaining privacy/display edge cases.
4. **Codex bridge** — inbox adapters and harness path moved under `scripts/`; finish discovery/publish path cleanly.
5. **Usage / dashboard** — provider quota plots landed recently; keep 2D telemetry ahead of 3D.
6. **Audit UX** — surface pending journal audit events when opening journals.

Do **not** start GraphQL/A2A-as-default/3D cathedrals ahead of memory + communication gates.

---

## 6. Cross-repo context Harbinger may still care about

| Repo | Note |
| --- | --- |
| Image-Toolkit | HIE submodule pointer advanced; Hybrid Editor re-exports only |
| HIE | Host IPC + UI ownership; Claude package flatten may still be dirty in HIE worktree |
| Project-Mobile-Fortress | CA V1 joint-task target; Godot 4 + C++ sim Slice-0 existed mid-August |
| Journals | Grok journal now has history from Jul–Aug 2026 + this HIE session |

---

## 7. Meta: how to write the next handoff

When leaving Coding-Assistants (or any long session):

1. Append a dated entry to `~/.coding-assistants/journals/grok/journal.md`.
2. Drop a short file under `.agent/messages/grok/` with: identity, commits, open claims, dirty peers, next steps.
3. Post a bus note if multi-agent work was concurrent.

This file’s name pattern: `SESSION_HANDOFF_YYYY-MM-DD_<from>_TO_<to>.md`.

---

## 8. One-line identity

**Grok’s job here:** implement and measure; preserve peer boundaries; memory-first collaboration hub; evidence over guesswork; Harbinger has final call.

— End of handoff —

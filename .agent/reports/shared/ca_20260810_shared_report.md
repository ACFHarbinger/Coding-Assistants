# Coding-Assistants: Shared Multi-Agent Report

**Date opened:** 2026-08-10  
**Owner/editor:** ACFHarbinger (and collaborating agents)  
**Repository under review:** `Coding-Assistants`  
**Document authority:** Shared synthesis edited by all five parties (Owner, Chat/Codex, Gemini, Claude, Grok)  
**Status:** Structure merged + owner Q&A applied (2026-08-10) — product contract largely DECIDED; owner may still fill prose §1  
**Purpose:** Reconcile independent CA analyses, record product identity and architecture decisions, and provide binding input to the post-brainstorm roadmap set.

**Provenance note (paths):**
- Independent reports: `.agent/reports/{chat,claude,gemini,grok}/`
- Owner/admin synthesis: `.agent/reports/admin/` (Chat scaffolds first; Gemini → Claude → Grok review in that order)
- **This shared report (canonical):** `.agent/reports/shared/ca_20260810_shared_report.md`
- Superseded template stubs: `coding_assistants_shared_report_20260810.md`, `shared_team_report.md` (pointer only)
- Coordination experiment log: `.agent/cache/shared_report_merge_coordination.md` (+ `AGENT_BUS.md` for T1 outline)
- Active roadmaps: `docs/moon/` and `docs/ROADMAP.md` (to be updated only after this brainstorm cycle)

---

## How to Use This Report

This document is the **joint** synthesis after independent reports and owner Q&A. It is not a substitute for each agent's own report under `.agent/reports/{agent}/`.

Use these labels consistently:

- **DECIDED** — owner has selected the policy or outcome.
- **PROVISIONAL** — current direction, subject to evidence or prototype results.
- **OPEN** — a decision is still required.
- **REJECTED** — considered and explicitly declined, with a reason.
- **OBSERVED** — direct fact from code, docs, or owner observation.
- **AGENT CLAIM** — conclusion from an agent report not yet independently accepted.

### Contribution rules

1. Add every material edit to the changelog at the end of this file.
2. Do not rewrite another contributor's changelog entry.
3. Cite the report, roadmap, ADR, code path, or owner observation supporting a claim.
4. Record disagreement; do not manufacture consensus.
5. The owner has final authority over product priorities and roadmap locks.
6. **Do not update `docs/moon/*` or `docs/ROADMAP.md` from this file until owner signals brainstorm complete.**

### Concurrent editing protocol

Four agent programs (`claude`, `codex`/`chat`, `gemini`/`agy`, `grok`) plus the owner edit concurrently:

1. **Re-read the file immediately before every edit session.**
2. **Edit append-only or inside blocks you own.** Never rewrite a peer's block; add a labeled response block instead.
3. **Shared tables:** only *add* rows; never reword an existing row without a `(disputed by <contributor>: …)` note.
4. If a previous edit is missing or mangled, restore it and note the restoration in the changelog.
5. The owner's edits always win; agents repair around them.

### Filling order (recommended)

1. §2 Provenance — link independent reports once they exist.  
2. §3 Product identity contract — resolve what CA *is*.  
3. §4 Architecture decisions — daemon, API, memory, agents.  
4. §5 Keep / Change / Archive / Reject.  
5. §6 Roadmap structure for post-brainstorm authors.  
6. §1 Executive summary — write last.

---

## 1. Executive Summary

### 1.1 Overall assessment

**Status:** `[OWNER TODO]`

Write 3–8 paragraphs covering:

- what Coding-Assistants is today (code + docs);
- what it is *for* (owner product identity);
- whether the current sequential multi-role pipeline matches that purpose;
- largest technical and product obstacles;
- assets worth keeping;
- what must be true before the next major roadmap lock.

### 1.2 Product identity (one paragraph)

**Status:** `DECIDED` (owner 2026-08-10)

> Coding-Assistants is a **local-first collaboration hub** for a human operator and multiple external AI coding agents (Claude Code, Codex, Gemini/Antigravity, Grok Build, OpenCode, Ollama, llama.cpp), providing shared context, durable hybrid memory, inter-agent messaging (async first), and desktop orchestration UI. The self-contained multi-role LLM pipeline was an initial experiment only. CA is intended to become the owner's daily driver surface for those agents.

### 1.3 Immediate priorities / Agreed roadmap priorities

**Status:** `[OWNER TODO]`

| Priority | Action / Capability | Owner | Dependencies | Exit criteria | Blocks |
| --- | --- | --- | --- | --- | --- |
| P0 | `[OWNER TODO]` | | | | |
| P1 | `[OWNER TODO]` | | | | |
| P2 | `[OWNER TODO]` | | | | |

*(Table widened 2026-08-10 merge: Chat template §5 columns Owner / Dependencies / Exit criteria.)*

**Observed scale gap (Claude evidence, 2026-08-10):** backend core is ~1,350 LOC (`lib.rs` 277, `agents.rs` 423, `llm_client.rs` 344, `tcp_server.rs` 272, `file_tools.rs` 32) while moon roadmaps describe actor daemon + GraphQL + A2A + 3D UI — ambition/code ratio should inform §1.1 when filled.

---

## 2. Review Inputs and Provenance

### 2.1 Independent status reports

| Agent | Path | Status |
| --- | --- | --- |
| Chat / Codex | `.agent/reports/chat/` | `[pending]` |
| Claude | `.agent/reports/claude/` | `[pending]` |
| Gemini | `.agent/reports/gemini/` | `[pending]` |
| Grok | `.agent/reports/grok/` | `[pending — after Q&A]` |
| Owner / Admin | `.agent/reports/admin/` | `[Chat scaffolds first]` |

### 2.2 Primary project evidence

| Artifact | Path | Notes |
| --- | --- | --- |
| App roadmap (short-term) | `docs/ROADMAP.md` | Feature checklist, v0.1.0 alpha |
| Moon index | `docs/moon/ROADMAP.md` | Scaffolding + target architecture pointer |
| Backend roadmap | `docs/moon/roadmaps/rust.md` | Daemon, GraphQL, MCP/A2A, budgets, memory |
| Frontend roadmap | `docs/moon/roadmaps/typescript.md` | GraphQL client, 2D/3D UI |
| TUI roadmap | `docs/moon/roadmaps/tui.md` | Ratatui (not scaffolded) |
| Android roadmap | `docs/moon/roadmaps/kotlin.md` | TCP → GraphQL migration |
| Architecture research | `docs/moon/research/Multi-Agent AI App Architecture.md` | Aspirational blueprint |
| Feature research | `docs/moon/reports/AI Coding Tools Feature Report.md` | Competitor feature synthesis |
| ADR 0002 | `docs/adr/0002-polyglot-module-layout.md` | Keep Tauri layout |
| ADR 0003 | `docs/adr/0003-daemon-extraction-spike.md` | Defer crate split; RD7 first |
| Backend core | `src-tauri/src/{agents,llm_client,tcp_server,file_tools,lib}.rs` | ~1.35k LOC |
| Frontend | `src/App.tsx` | ~900 LOC monolith |
| Android | `android/app/...` | ~1k LOC Kotlin |

### 2.3 Cross-agent consensus (fill after independent reports)

**Status:** `[pending all four reports]`

| Theme | Agreement? | Notes |
| --- | --- | --- |
| Product identity = multi-agent collab hub | OPEN | |
| Dual roadmaps need merge/reconcile | OPEN | |
| RD7 event bus before daemon split | OPEN | |
| GraphQL vs simpler local IPC | OPEN | |
| 3D visualization priority | OPEN | |
| Scaffolding infra scope | OPEN | |

### 2.4 Current-state findings by area

**Status:** scaffold only (merged from Chat template §3) — fill with evidence after independent reports

| Area | Observed strengths | Observed weaknesses | Evidence |
| --- | --- | --- | --- |
| Backend / orchestration | Sequential multi-role slice works; governor rate limit; KillOnDrop; ADR 0003 | Sequential only; CLI scrape; global AppState races; MCP write-only | `agents.rs`, `llm_client.rs`, `lib.rs`, ADR 0003 |
| Frontend | Working glass UI; invoke/listen streaming | Monolith `App.tsx` ~900 LOC; no tests | `src/App.tsx` |
| Android | TCP remote client present | No auth; protocol tied to Tauri bus | `android/`, `tcp_server.rs` |
| Documentation / roadmaps | Rich moon research + ADRs | Dual roadmaps (`docs/ROADMAP.md` vs `docs/moon/`); scaffolding marked ✅ while product thin | `docs/moon/`, `docs/ROADMAP.md` |
| Security / infrastructure | Honest SECURITY.md; no shell invocation for providers | Path traversal; `read_file_absolute`; TCP `0.0.0.0` no auth; CSP null; heavy unused infra | `file_tools.rs`, `lib.rs`, `tcp_server.rs`, `infra/` |

### 2.5 Agent perspectives (freeform)

**Status:** slots created from Gemini template §2–5 — each agent fills their own block; do not rewrite peers

#### Owner

`[OWNER TODO]`

#### Gemini (Antigravity)

**Perspective (Post Q&A):** The owner has provided incredibly clear direction. The primary focus is a *local collaboration hub* for a solo power developer, prioritizing **shared memory** (SQLite + Markdown), **cross-agent messaging**, and **2D observability** over immediate daemon extraction, GraphQL, or 3D visual gimmicks. We should aggressively demote the TUI, 3D graphics, and Android client to focus entirely on robust memory tiers, sequential/boundary-defined task execution, and robust tool sandboxing via OS APIs (with configurable human gates). A crucial pattern emerged for budgeting: if funds run low, agents must summarize progress to persistent Markdown, delegate, and halt instead of hard-failing.

#### Chat (Codex)

`[CHAT TODO]`

#### Claude (Code)

`[CLAUDE TODO — optional freeform; structured arch/security notes also live in §4.5 / §7 / §8 from merge]`

#### Grok (Build)

**AGENT CLAIM → largely confirmed by owner DECIDED (2026-08-10):** Collaboration hub for external agents + human. Near-term spine: **hub memory/messaging first** ([`docs/moon/roadmaps/hub.md`](../../../docs/moon/roadmaps/hub.md)), RD7 bus, wire HTTP providers, security backlog; park GraphQL/A2A/3D/TUI. Milestone lean: Plan Alpha (Memory Hub First). Full report: `.agent/reports/grok/ca_20260810_status_report.md`.

---

## 3. Product Contract

| # | Question | Status | Decision / notes |
| --- | --- | --- | --- |
| PC1 | Primary user(s) | DECIDED | Solo power developer (human), coordinating with external AI agents. |
| PC2 | Primary agents integrated | DECIDED | Claude Code, Codex/Chat, Gemini/Antigravity, Grok Build, OpenCode, Ollama, llama.cpp. |
| PC3 | Sync vs async collaboration model | DECIDED | Async mailbox/sequential boundaries first. True parallel execution is a future goal. |
| PC4 | Shared memory model | DECIDED | Hybrid: SQLite for deep long-term storage, git-tracked Markdown for high-priority insights/decisions. Both global and per-repo scopes. |
| PC5 | Orchestration depth | DECIDED | Declarative wiring of agents (node-editor style) first, with probabilistic A2A delegation as a future option. |
| PC6 | UI priority order | DECIDED | Desktop GUI is primary. Android (monitoring only) and TUI are demoted/secondary. 3D viz is research-only (standard 2D DAG preferred). |
| PC7 | Network threat model | DECIDED | Keep LAN TCP for now; strict auth/tunnels on roadmap later. Sandbox strictly configurable per task (relaxed default). |
| PC8 | Cost controls required at v1 | DECIDED | Telemetry + soft warning default. Optional hard kill. If budget runs out, pause, write persistent markdown summary, and await user. |
| PC9 | Open-source packaging scope | DECIDED | Dual AGPL-3.0 + Commercial. Keep docker/terraform/ansible; other infra marked for trim (roadmap T6b — not necessarily deleted in-tree yet). |
| PC10| Human gate requirement | DECIDED | Configurable per task (e.g. large refactors require gate, small tasks don't). |

---

## 4. Architecture Decisions

### TL;DR snapshot (from Gemini template §6 — fill as decisions land)

| Topic | Status |
| --- | --- |
| Core Orchestration Daemon | DEMOTED (Event bus first, daemon extraction later) |
| GraphQL / WebSockets API | DEMOTED (Later, simple IPC / Unix sockets acceptable) |
| TUI & 3D GUI | DEMOTED (TUI secondary, 3D research-only, prioritize 2D DAG) |
| MCP & A2A protocols | PROVISIONAL (MCP later, focus on OS APIs/CLI directly first) |
| Resource management (affine budgets) | DECIDED (Runtime soft limits + pause & summarize protocol) |
| Cross-Agent Shared Memory & Coordination | TOP PRIORITY (New priority track) |

### 4.1 Process topology

**Status:** `PROVISIONAL` (ADR 0003)

| Option | Description | Lean |
| --- | --- | --- |
| A | Stay in-process Tauri; event bus only (RD7) | ADR 0003 short-term |
| B | Headless daemon + thin clients (GUI/TUI/Android) | moon long-term |
| C | Hybrid: local bus first, extract daemon when 2nd client lands | Grok lean (non-binding) |
| D | Hybrid split: durable state in git/SQLite; daemon/hub only for live wake-signal | **Owner-aligned lean (DECIDED direction)** — hub memory first; daemon later |

**PROVISIONAL sequencing:** D/C near-term (durable hub + RD7 bus) → B when multi-client needs force extract.

### 4.2 Client API surface

**Status:** OPEN

| Option | Pros | Cons |
| --- | --- | --- |
| GraphQL + WS | Flexible queries/subs | Heavy for local-only v1 |
| Typed JSON-RPC / custom WS | Simple, fast | Less tooling |
| Keep Tauri IPC + side-channel for TUI/Android | Minimal churn | Two APIs forever if not unified |
| gRPC / Connect | Strong typing, streaming | Less browser-friendly |

### 4.3 Agent integration model

**Status:** OPEN

| Option | Description |
| --- | --- |
| Marker protocol only (`[[ASK_*]]`) | Current; works for in-process roles |
| CLI headless SDK (stream-json) | Claude/Codex-style structured events |
| MCP host + client | Tools/resources/prompts standard |
| Session bridge to external agent CLIs | Wake/resume real Claude/Grok/Codex sessions |
| Hybrid | Markers for in-app roles; bridges for external tools |

### 4.4 Memory

**Status:** OPEN

| Option | Description |
| --- | --- |
| File-only (`.agent/`, `project_memory.md`) | Current; git-friendly |
| Two-tier (briefing + deep store) as in moon RP* | Research plan |
| SQLite + FTS/vector | Queryable sessions |
| Hybrid file + DB | Briefing in git; deep store local |

### 4.5 Security baseline for v1

**Status:** OPEN

Must address at least: path traversal in `FileTools`, scope of `read_file_absolute`, TCP auth/bind policy, CSP for production.

**Additional OBSERVED gaps (Claude review, 2026-08-10 — code-verified):**

1. **Global task state race:** `AppState.agents` / `cancellation_token` / `user_input_tx` are single `Mutex<Option<T>>` — concurrent `run_agent_task` calls can clobber cancel/input channels (`lib.rs`).
2. **Shared MCP config path race:** `mcp_config` written to fixed `~/.coding-assistants/mcp.json` for all runs (`agents.rs`) — concurrent tasks race on the file.

---

## 5. Keep, Change, Archive, Reject

### 5.1 Keep

| Item | Why | Source |
| --- | --- | --- |
| `[fill]` | | |

### 5.2 Change

| Item | Why | Candidate avenues | Source |
| --- | --- | --- | --- |
| `[fill]` | | | |

### 5.3 Archive / deprioritize

| Item | Why | Source |
| --- | --- | --- |
| `[fill]` | | |

### 5.4 Reject

| Item | Why | Source |
| --- | --- | --- |
| `[fill]` | | |

---

## 6. Final Roadmap Structure (for post-brainstorm authors)

### 6.1 Recommended document set

**Status:** OPEN

Proposed options:

1. **Single active index** (`docs/moon/ROADMAP.md`) + per-area files; archive or merge `docs/ROADMAP.md` into it.  
2. **Capability maps** (memory, messaging, orchestration, UI, security) instead of language silos.  
3. **Keep dual docs** with explicit "product features" vs "platform architecture" split and cross-links.

### 6.2 Sequencing principles

`[OWNER TODO after Q&A]`

### 6.3 First three concrete engineering increments

| # | Increment | Depends on | Success criterion |
| --- | --- | --- | --- |
| 1 | `[OPEN]` | | |
| 2 | `[OPEN]` | | |
| 3 | `[OPEN]` | | |

---

## 7. Conflict Register

| Conflict | Position A | Position B | Owner resolution | Recorded in |
| --- | --- | --- | --- | --- |
| Product = role pipeline vs collab hub | Current code/docs lean pipeline | Owner statement (session) leans collab hub | `[OWNER TODO]` | §3 |
| GraphQL now vs later | moon RA1 early | ADR 0003 / RD7 first | `[OWNER TODO]` | §4.2 |
| 3D viz priority | moon T3D* | Likely polish after core hub | `[OWNER TODO]` | §5 |
| Infra scaffolding (k8s/helm/etc.) | Keep full template | Slim to what CA actually deploys | `[OWNER TODO]` | §5 |
| Affine budgets compile-time | moon RB4 | Runtime budgets + telemetry enough for v1 | `[OWNER TODO]` | §4 / §5 |
| A2A protocol | moon RM4 XL | Local messaging bus first | `[OWNER TODO]` | §4.3 |
| Dual roadmap docs | `docs/ROADMAP.md` feature list | `docs/moon/` platform tracks | `[OWNER TODO — Claude: moon canonical, top-level stub/retire?]` | §6.1 |
| Coord channel thrash (this experiment) | Three cache files opened | Converge on one | **PROVISIONAL:** `shared_report_merge_coordination.md` owner-facing; `AGENT_BUS.md` holds T1 | `.agent/cache/` |

---

## 8. Risks and Constraints

| Risk | Impact | Mitigation options |
| --- | --- | --- |
| Roadmap ambition exceeds product identity | Build wrong system | Lock PC* before large RA/RD work |
| Concurrent multi-agent file edits | Lost work | Message bus + report protocol (this session is the pilot) |
| CLI-only LLM integration brittleness | Silent failures | Structured SDKs / HTTP providers |
| No automated tests | Regressions during refactor | Vitest + cargo test smoke before RD7 |
| Unauthenticated LAN TCP | Remote task injection | Token auth or localhost-only default |
| Dead / unused Cargo deps (`async-openai`, `reqwest`, `dotenv`, `walkdir` declared but unused in `src-tauri/src/`) | Confusion; abandoned HTTP-provider start | Wire up or remove; document intent (Claude 2026-08-10) |
| Single global AppState for cancel/input | Silent cross-task clobber | Per-task IDs + channels (Claude 2026-08-10) |

---

## 9. Final Owner Decisions

### 9.1 Accepted consensus

`[OWNER TODO]`

### 9.2 Accepted minority recommendations

`[OWNER TODO]`

### 9.3 Rejected recommendations

`[OWNER TODO]`

### 9.4 Instructions to roadmap authors

`[OWNER TODO — after brainstorm complete]`

---

## 10. Completion Checklist

- [ ] All four independent agent reports present  
- [ ] Admin report scaffolded by Chat and reviewed Gemini → Claude → Grok  
- [ ] Owner Q&A answers recorded  
- [ ] Product identity locked (PC1–PC9 or subset)  
- [ ] Architecture choices for next 1–2 increments locked  
- [ ] Keep/Change/Archive/Reject tables filled  
- [ ] Roadmaps updated in a dedicated commit (post-brainstorm only)  
- [ ] Final structure pass: each agent marks agree/disagree below  

### Final structure pass (agents)

| Agent | Agree with final structure? | Notes | Date |
| --- | --- | --- | --- |
| Chat | | | |
| Gemini | | | |
| Claude | | | |
| Grok | **Yes (structure only)** | Merged Chat/Gemini slots + Claude evidence into this file; decisions still OPEN for owner | 2026-08-10 |
| Owner | | | |

---

## 11. Collaborative Changelog

### Changelog rules

- Append only; never edit others' entries.
- Date + contributor + bullet list of sections touched.

### Detailed contribution notes

#### Grok — 2026-08-10 (initial scaffold)

- Created initial shared report template adapted from Image-Toolkit admin report patterns, specialized for Coding-Assistants product/architecture decisions.
- Did not fill owner decision cells; left OPEN markers.
- Independent Grok status report deferred until after owner Q&A (per session instructions).
- Did not mutate `docs/moon/*` or `docs/ROADMAP.md` yet.

#### Chat/Codex — 2026-08-10 (source template; pre-merge)

- Authored compact scaffold `coding_assistants_shared_report_20260810.md` (exec summary, current-state table, priorities, final review).
- Proposed coordination channel `.agent/cache/shared_report_merge_coordination.md` and canonical path = this file.

#### Gemini — 2026-08-10 (source template; pre-merge)

- Authored `shared_team_report.md` with per-agent Perspective sections and consensus TL;DR bullets.

#### Claude — 2026-08-10 (merge outline + evidence)

- Posted T1 keep/merge/drop map on `.agent/cache/AGENT_BUS.md`.
- Architecture/security/evidence review on coordination channel: AppState races, MCP path race, dual roadmaps, unused Cargo deps, LOC scale gap.
- ACK canonical path and channel.

#### Grok — 2026-08-10 (structural merge pass)

#### Grok — 2026-08-10 (owner Q&A applied)

- Recorded DECIDED product contract rows PC1–PC13 from owner answers.
- Wrote independent report `.agent/reports/grok/ca_20260810_status_report.md`.
- Updated moon roadmaps: new `hub.md`, demoted TUI/3D, folded old ROADMAP, security/provider backlog.
- Recommended milestone Plan Alpha (Memory Hub First) with embedded P0 security bugs.


- Executed agreed merge into this file: §1.3 columns; §2.4 current-state; §2.5 perspectives; §4 TL;DR; option D process topology; §4.5 Claude gaps; §7 dual-roadmap + coord thrash rows; §8 dead-deps + AppState risks.
- Stubbed non-canonical shared templates with pointers (no hard delete).
- Dual-posted progress on coordination channel + AGENT_BUS.
- Still no roadmap file mutations; owner decisions remain OPEN.

#### Gemini — 2026-08-10 (Post-Q&A Product Contract Update)

- Updated §3 Product Contract (PC1-PC9) to DECIDED based on owner's Q&A, and added PC10.
- Filled §2.5 Gemini freeform perspective slot reflecting the owner's answers.
- Updated §4 TL;DR snapshot with decisions regarding daemon demotion, 3D/TUI demotion, and Memory promotion.

---

## Appendix A — Owner scratch pad

```
[OWNER TODO: freeform notes]
```

## Appendix B — Evidence pointers

| Artifact | Location |
| --- | --- |
| This template | `.agent/reports/shared/ca_20260810_shared_report.md` |
| Admin report | `.agent/reports/admin/` (Chat first) |

---

## Chat/Codex contribution after owner Q&A — 2026-08-10

**Status:** AGENT CLAIM informed by owner answers; owner decisions remain the
final authority.

The product center is a personal, local-first collaboration hub for the owner,
external agents, and human collaborators. The initial self-contained
multi-role LLM chain is retained as an experiment, not the defining product.
The immediate priority is hybrid shared memory and asynchronous coordination:
SQLite for durable global/workspace state, Git-tracked Markdown for important
decisions and handoffs, and a separate ephemeral wake mechanism.

Recommended sequence:

1. Durable memory, message/handoff records, identities, and a small CLI helper.
2. Internal event bus and per-task state, replacing global cancellation/input
   state.
3. Explicit wired workflows with configurable human gates and standing agent
   permissions.
4. Provider/session adapters for OpenCode, Claude, Codex, Gemini, Grok,
   Ollama, and llama.cpp.
5. Multi-client protocol and daemon extraction only when the second real client
   requires it.

GraphQL, actor frameworks, A2A, Ratatui, and 3D visualization remain future or
research tracks. A Unix-domain-socket plus typed JSON-RPC/event protocol is a
more proportionate first multi-client experiment than GraphQL. The first
success benchmark should be a meaningful cross-repository task completed by
the owner and at least two agents, with durable handoff retrieval and review of
transcript, memory, approvals, and Git changes.

### Owner-confirmed product inputs

| Decision area | Owner direction |
| --- | --- |
| Audience | Solo power developer first; trusted collaborators and organization later |
| Collaboration | Explicit wired roles and bounded tasks first; asynchronous before parallel |
| Memory | Shared/global plus repository scopes; SQLite and Markdown hybrid |
| Wake-ups | Allowed while away; human gate configurable per task/session |
| Tools | Execution allowed; approval and sandbox strictness configurable |
| Cost | Telemetry/soft warning by default; optional hard limit |
| Android | Monitoring first, then approval/messaging |
| TUI/3D | Nice-to-have/research only |
| Infrastructure | Keep Docker, Terraform, Ansible; delete or archive the rest |
| Roadmaps | Separate product/platform/research/infrastructure documents with links |

### Chat report

Full report: `.agent/reports/chat/coding_assistants_architecture_review_20260810.md`

### Changelog

| Date | Contributor | Change | Decision changed? |
| --- | --- | --- | --- |
| 2026-08-10 | Chat/Codex | Added owner-Q&A synthesis, recommended sequencing, and product contract. | No — owner-confirmed inputs recorded as proposed binding direction. |
| Independent reports | `.agent/reports/{chat,claude,gemini,grok}/` |

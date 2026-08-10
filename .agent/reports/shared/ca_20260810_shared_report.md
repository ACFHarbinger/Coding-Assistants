# Coding-Assistants: Shared Multi-Agent Report

**Date opened:** 2026-08-10  
**Owner/editor:** ACFHarbinger (and collaborating agents)  
**Repository under review:** `Coding-Assistants`  
**Document authority:** Shared synthesis edited by all five parties (Owner, Chat/Codex, Gemini, Claude, Grok)  
**Status:** Owner admin report complete; capability roadmaps live (Chat); Grok alignment pass 2026-08-10 — owner may still fill §1.1 prose and final structure vote  
**Purpose:** Reconcile independent CA analyses, record product identity and architecture decisions, and provide binding input to the post-brainstorm roadmap set.

**Provenance note (paths):**
- Independent reports: `.agent/reports/{chat,claude,gemini,grok}/`
- Owner/admin synthesis: `.agent/reports/admin/` (Chat scaffolds first; Gemini → Claude → Grok review in that order)
- **This shared report (canonical):** `.agent/reports/shared/ca_20260810_shared_report.md`
- Superseded template stubs: `coding_assistants_shared_report_20260810.md`, `shared_team_report.md` (pointer only)
- Coordination experiment log: `.agent/cache/shared_report_merge_coordination.md` (+ `AGENT_BUS.md` for T1 outline)
- Active roadmaps: `docs/moon/ROADMAP.md` + capability files under `docs/moon/roadmaps/` (language silos removed; `docs/ROADMAP.md` deleted)
- Owner admin report: `.agent/reports/admin/coding_assistants_status_report_20260810.md` (filled; §3 report reviews done)

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
6. Roadmap authoring is **in progress / live** under `docs/moon/` per owner admin report; agents must not thrash structure without citing admin §8/§11.

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

**Status:** `DECIDED` (owner admin §1.3 + moon v2 capability index)

| Priority | Action / Capability | Owner | Dependencies | Exit criteria | Blocks |
| --- | --- | --- | --- | --- | --- |
| P0 | Hybrid memory (SQLite short/episodic/semantic + git Markdown + private journals) | Hub team | — | M1–M6 gates in `roadmaps/memory.md` | Collab quality |
| P0 | Async mailboxes, identities, CLI helper, wake signals, gates/policies | Hub team | Memory store | C1–C4 in `roadmaps/communication.md` | Agent coordination |
| P1 | Event bus + per-task state; provider adapters; runtime budgets; path/TCP security | Platform | P0 spine usable | P1–P6,P10 in `roadmaps/platform.md` | Reliability |
| P1 | Desktop hub UI + 2D dashboard | Frontend | Memory/inbox APIs | U1–U4, D1–D3 | Usability |
| P2 | A2A interoperability | Later | Local coord solid | C7 | Interop |
| P3 | Android monitor/approve; TUI/3D research | Later | Desktop stable | U5 / D6 | Mobile polish |

**Observed scale gap (still true):** backend core ~1,350 LOC; ambition must stay gated by memory/coordination gates first.

---

## 2. Review Inputs and Provenance

### 2.1 Independent status reports

| Agent | Path | Status |
| --- | --- | --- |
| Chat / Codex | `.agent/reports/chat/coding_assistants_architecture_review_20260810.md` | Present — owner: excellent |
| Claude | `.agent/reports/claude/ca_20260810_claude_report.md` | Present — owner: detailed |
| Gemini | `.agent/reports/gemini/gemini_status_report_20260810.md` | Present — owner: clear avenues |
| Grok | `.agent/reports/grok/ca_20260810_status_report.md` | Present — owner: best Q&A synthesis |
| Owner / Admin | `.agent/reports/admin/coding_assistants_status_report_20260810.md` | **Filled** (report reviews + decisions) |

### 2.2 Primary project evidence

| Artifact | Path | Notes |
| --- | --- | --- |
| Moon index + Gantt | `docs/moon/ROADMAP.md` | **Canonical** capability index (v2.0, Chat) |
| Memory roadmap | `docs/moon/roadmaps/memory.md` | P0 hybrid SQLite/Markdown/journals |
| Communication roadmap | `docs/moon/roadmaps/communication.md` | P0 mailboxes, CLI, wake, A2A later |
| Platform roadmap | `docs/moon/roadmaps/platform.md` | Event bus, providers, security, budgets |
| UI roadmap | `docs/moon/roadmaps/ui.md` | Desktop first; Android after; TUI someday |
| Dashboard roadmap | `docs/moon/roadmaps/dashboard.md` | 2D telemetry; 3D research |
| Infrastructure roadmap | `docs/moon/roadmaps/infrastructure.md` | docker/terraform/ansible/firebase |
| Owner admin report | `.agent/reports/admin/coding_assistants_status_report_20260810.md` | Binding product decisions |
| Architecture research | `docs/moon/research/` | Aspirational; not near-term commits |
| ADR 0003 | `docs/adr/0003-daemon-extraction-spike.md` | Event bus first — ACCEPTED |
| Backend core | `src-tauri/src/*.rs` | ~1.35k LOC sequential pipeline |
| Frontend | `src/App.tsx` | ~900 LOC monolith |
| Infra tree | `infra/{docker,terraform,ansible,firebase}` | Heavy scaffolding already pruned |

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

**Grok (post admin + capability roadmaps):** Owner admin report and Chat’s capability roadmaps **agree** with Plan Alpha. Binding spine: `memory.md` + `communication.md` first; `platform.md` event bus + per-task state next; A2A is next major *after* local coord (not archived). Language roadmaps correctly removed. Full report: `.agent/reports/grok/ca_20260810_status_report.md`. Implementation started: `crates/ca-hub` + `ca` CLI.

---

## 3. Product Contract

| # | Question | Status | Decision / notes |
| --- | --- | --- | --- |
| PC1 | Primary user(s) | DECIDED | Solo power developer (human), coordinating with external AI agents. |
| PC2 | Primary agents integrated | DECIDED | Claude Code, Codex/Chat, Gemini/Antigravity, Grok Build, OpenCode, Ollama, llama.cpp. |
| PC3 | Sync vs async collaboration model | DECIDED | Async mailbox/sequential boundaries first. True parallel execution is a future goal. |
| PC4 | Shared memory model | DECIDED | Hybrid SQLite + git Markdown; tiers: **short-term** (raw recent), **episodic**, **semantic**; global + per-repo; **private journals** per agent (optional encrypt; shared never encrypted). |
| PC5 | Orchestration depth | DECIDED | Declarative wiring of agents (node-editor style) first, with probabilistic A2A delegation as a future option. |
| PC6 | UI priority order | DECIDED | Desktop GUI is primary. Android (monitoring only) and TUI are demoted/secondary. 3D viz is research-only (standard 2D DAG preferred). |
| PC7 | Network threat model | DECIDED | Keep LAN TCP for now; strict auth/tunnels on roadmap later. Sandbox strictly configurable per task (relaxed default). |
| PC8 | Cost controls required at v1 | DECIDED | Telemetry + soft warning default. Optional hard kill. If budget runs out, pause, write persistent markdown summary, and await user. |
| PC9 | Open-source packaging scope | DECIDED | Dual AGPL-3.0 + Commercial. Keep docker/terraform/ansible/**firebase**; heavy infra **removed from tree** (Chat pass). Package names remain `tauri-app` until owner renames. |
| PC10| Human gate requirement | DECIDED | Configurable per task (e.g. large refactors require gate, small tasks don't). |
| PC11| Memory tiers | DECIDED | Short-term / episodic / semantic + private journals (admin §5). |
| PC12| V1 success bar | DECIDED | Collaborative quality on PMF (and multi-repo) ≥ single human contributor; issues in 2 sprints + holistic review. |
| PC13| Roadmap shape | DECIDED | Capability files + moon index Gantt; language silos removed. |
| PC14| Runtime budgets | DECIDED | Soft warn default; optional hard kill; pause → Markdown summary → delegate → shutdown. Affine typing postponed. |


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

**Status:** `DECIDED` direction (owner admin §6.3)

| Option | Pros | Cons |
| --- | --- | --- |
| GraphQL + WS | Flexible queries/subs | Heavy for local-only v1 |
| Typed JSON-RPC / custom WS | Simple, fast | Less tooling |
| Keep Tauri IPC + side-channel for TUI/Android | Minimal churn | Two APIs forever if not unified |
| gRPC / Connect | Strong typing, streaming | Less browser-friendly |

### 4.3 Agent integration model

**Status:** `DECIDED` direction (declarative wiring + external adapters; markers temporary)

| Option | Description |
| --- | --- |
| Marker protocol only (`[[ASK_*]]`) | Current; works for in-process roles |
| CLI headless SDK (stream-json) | Claude/Codex-style structured events |
| MCP host + client | Tools/resources/prompts standard |
| Session bridge to external agent CLIs | Wake/resume real Claude/Grok/Codex sessions |
| Hybrid | Markers for in-app roles; bridges for external tools |

### 4.4 Memory

**Status:** `DECIDED` (owner admin §5 / §6.4 + `roadmaps/memory.md`)

| Layer | Role |
| --- | --- |
| Short-term | Raw recent transcripts/logs for task re-entry |
| Episodic | Important events/lessons |
| Semantic | Architecture, deps, features |
| Git Markdown | High-priority handoffs and decisions |
| Private journals | Per-agent; optional encrypt; not merged into shared |
| Scopes | Global + workspace |


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
| Terraform, Docker, Ansible | Required for basic devops and potential cloud deployments. | Owner Admin Report §9.1 |
| Crate/Package names (`ca`) | Current names have proper IDE icons and are liked by the owner. | Owner Q&A R.31 |
| SQLite | Required for durable, long-term, compressed memory tier. | Owner Admin Report §9.1 |

### 5.2 Change

| Item | Why | Candidate avenues | Source |
| --- | --- | --- | --- |
| License | Prevent unauthorized commercialization while remaining free. | Dual AGPL-3.0 + Commercial | Owner Admin Report §9.2 |
| Roadmap Structure | Decouple roadmap from tech stack (e.g. rust.md). | Feature-specific tracks (`memory.md`, `ui.md`) | Owner Admin Report §8.1 |
| CLI wrappers | Too brittle for structured tool parsing. | Native HTTP/SDK integrations | Shared consensus |

### 5.3 Archive / deprioritize

| Item | Why | Source |
| --- | --- | --- |
| 3D Visualization | Demoted to research-only; V1 needs standard 2D observability. | Owner Admin Report §9.3 |
| Ratatui TUI | Nice-to-have secondary interface, not required for V1 hub. | Owner Admin Report §9.3 |
| Android Client | Prioritizing desktop GUI stability first. | Owner Admin Report §6.6 |

### 5.4 Reject

| Item | Why | Source |
| --- | --- | --- |
| Heavy Cloud Infra (K8s, Helm) | Unnecessary for a local-first hub; overcomplicates deployment. | Owner Admin Report §9.4 |
| Single local API protocol | Not strictly required; Unix sockets are fine for now. | Owner Admin Report §9.4 |
| GraphQL and Actor Frameworks | Too complex for V1; frozen/parked for a later date. | Owner Admin Report §9.4 |

---

## 6. Final Roadmap Structure (for post-brainstorm authors)

### 6.1 Recommended document set

**Status:** DECIDED (Owner)

1. **Single active index**: `docs/moon/ROADMAP.md` remains the high-level index with gantt charts. Old `docs/ROADMAP.md` is removed.
2. **Capability maps**: Language silos (`rust.md`, `typescript.md`) are replaced with feature tracks (`memory.md`, `communication.md`, `ui.md`, `infrastructure.md`, `platform.md`, `dashboard.md`).

### 6.2 Sequencing principles

**Status:** DECIDED (Owner)

- Prioritize the "Persistent Shared Memory & Cross-Agent Coordination" track above all else.
- Asynchronous mailboxes and defined sequential boundaries must be built before true parallel execution.
- Must include mandatory acceptance tests and exit criteria gated every ~5 roadmap entries.

### 6.3 First three concrete engineering increments

| # | Increment | Depends on | Success criterion |
| --- | --- | --- | --- |
| 1 | **Hybrid Memory & Handoff** (SQLite schemas + Git-tracked Markdown summaries) | None | Two agents successfully retrieve each other's durable context across a repository boundary. |
| 2 | **Event Bus & Async Mailboxes** (Implement ADR 0003) | Increment 1 | Global `AppState` races resolved; agents can pause and await user safely without clobbering channels. |
| 3 | **Provider Adapters & Direct SDKs** | Increment 2 | External agent LLM calls are executed via native HTTP or structured SDKs, eliminating CLI stdout scraping. |

---

## 7. Conflict Register

| Conflict | Position A | Position B | Owner resolution | Recorded in |
| --- | --- | --- | --- | --- |
| Product = role pipeline vs collab hub | Pipeline experiment | Collaboration hub | **DECIDED hub** (admin §11) | §3 |
| GraphQL now vs later | GraphQL-first | ADR 0003 / UDS later | **DECIDED later** (admin §7) | §4.2 |
| 3D viz priority | Force-graph near-term | 2D first | **DECIDED 2D; 3D research** | §5 / dashboard.md |
| Infra scaffolding | Full template | Lean local | **DECIDED prune** (done in tree; keep docker/tf/ansible/firebase) | infrastructure.md |
| Affine budgets | Compile-time affine | Runtime soft/hard + pause | **DECIDED runtime; affine postponed** | platform.md P10 |
| A2A protocol | Immediate | Local mailbox first | **DECIDED local first; A2A next major milestone (C7)** | communication.md |
| Dual roadmap docs | docs/ROADMAP.md | docs/moon | **DECIDED moon only; root ROADMAP removed** | moon/ROADMAP.md |
| Language vs capability roadmaps | rust/ts/kotlin silos | memory/ui/… | **DECIDED capability** (Chat restructure) | §6 |
| Coord channel thrash | Multiple cache files | Single bus | **RESOLVED** for experiment; product needs real hub | .agent/cache/ |
| Package naming | Rename to ca | Keep tauri-app icons | **PROVISIONAL keep tauri-app** until explicit rename task | §9.1 wording vs Q&A |

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

- Product is a **collaboration hub** for external agents + human (admin §11.1).
- Hybrid **SQLite + Markdown** memory with multi-tier + private journals.
- **Plan Alpha** / memory-first sequencing (all agents + owner).
- Capability roadmaps under `docs/moon/roadmaps/` with Gantt index.

### 9.2 Accepted minority recommendations

- ADR 0003 event bus before daemon extract.
- Unix domain sockets / JSON before GraphQL.
- Runtime budgets with pause/summary/shutdown (not affine compile-time first).
- Dual AGPL-3.0 + Commercial license (implementation pending legal pass).

### 9.3 Rejected recommendations

- Self-contained multi-role LLM app as product identity.
- Near-term GraphQL, actors, TUI-as-primary, 3D force-graph.
- Heavy k8s/helm/serverless/etc. scaffolding in active tree.

### 9.4 Instructions to roadmap authors

- Additive tags for demotions; archive speculative ideas under `docs/moon/archive/`.
- Agents write only under `.agent/reports/{name}` or `shared`.
- Wire unused HTTP Cargo deps into provider work (P4), do not drop.
- Acceptance criteria every ~5 items; memory + coordination gates before A2A.
- **Build order:** memory store + CLI → mailboxes/wake → event bus/per-task state → UI/dashboard → providers polish.

---

## 10. Completion Checklist

- [x] All four independent agent reports present  
- [x] Admin report filled by owner (incl. §3 report reviews)  
- [x] Owner Q&A answers recorded  
- [x] Product identity locked  
- [x] Architecture choices for next increments locked (admin §6 + moon v2)  
- [x] Keep/Change/Archive/Reject tables filled  
- [x] Capability roadmaps updated (`docs/moon/`)  
- [ ] Final structure pass: each agent marks agree/disagree below (after owner shared-report pass)  
- [x] Implementation M1/C1/C2 started (`crates/ca-hub` + `ca` CLI; tests green)  

### Final structure pass (agents)

| Agent | Agree with final structure? | Notes | Date |
| --- | --- | --- | --- |
| Chat | | | |
| Gemini | **Yes** | Filled out §5 Keep/Change/Reject, §6 Roadmap Structure/Increments based on Admin Report. | 2026-08-10 |
| Claude | | | |
| Grok | **Yes** | Aligned with finished admin report + Chat capability roadmaps; ready for owner prose §1.1 and final vote | 2026-08-10 |
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

---

## Chat/Codex final verification and shared-report contribution — 2026-08-10

### Verification result

I read the completed owner/admin report, all current capability roadmaps, the
roadmap index, archive guidance, and the canonical shared report. The owner’s
product direction is coherent and ready for implementation:

- local-first collaboration hub for the owner, human collaborators, and
  external agents;
- SQLite plus Markdown shared memory with global, workspace, episodic,
  semantic, and recent/raw tiers;
- private per-agent journals separated from shared memory;
- asynchronous mailbox and declarative task boundaries before parallel work;
- configurable tool, sandbox, human-gate, wake, and budget policies;
- LAN support retained for now;
- desktop first, Android monitoring/approval later;
- A2A as the next major milestone after the local hub foundation;
- TUI, 3D visualization, GraphQL-first APIs, and early actor frameworks
  deferred without being deleted;
- Firebase retained for cloud/device synchronization prototypes;
- retained infrastructure flattened under `infra/ansible`, `infra/docker`,
  `infra/firebase`, and `infra/terraform`.

### Current roadmap state

The active capability roadmaps are:

1. `memory.md`
2. `communication.md`
3. `platform.md`
4. `ui.md`
5. `dashboard.md`
6. `infrastructure.md`

`docs/moon/ROADMAP.md` is the canonical Mermaid-Gantt index. The former
`docs/ROADMAP.md` and language-oriented roadmaps were intentionally deleted per
the owner’s final instruction.

### Corrections to historical report wording

Some earlier report sections necessarily describe the state before the latest
roadmap commit. They should be read as provenance, not current instructions:

- references to `rust.md`, `typescript.md`, `kotlin.md`, `tui.md`, and `hub.md`
  are superseded by the capability roadmap set above;
- A2A is not research-only; it is the next major milestone, after the local
  memory and communication foundation;
- the retained infrastructure is now flattened, with the obsolete deployment
  stacks removed and Firebase preserved;
- the Gantt dates are sequencing placeholders, not delivery promises.

### Chat’s final keep/change/park verdict

**Keep:** the Rust/Tauri foundation, typed IPC/events, local-first operation,
provider flexibility, event-bus-first ADR, SQLite/Markdown hybrid, private
journals, human-configurable autonomy, LAN development path, and explicit
acceptance gates.

**Change:** implement memory and communication as independent capability
modules; replace global task state with per-task state; make provider sessions
typed; make tool execution auditable; use simple local protocols before a
daemon/API extraction; keep the dashboard focused on actionable 2D telemetry.

**Park:** affine typing, GraphQL-first APIs, early actor frameworks, TUI, and
3D visualization. **Sequence next:** A2A interoperability after the local
mailbox, identity, policy, audit, and budget foundations are reliable.

### Final structure opinion

Chat agrees with the shared report structure and the owner’s completed admin
report. The remaining work is to let the owner add any final personal wording
or attachments, then perform one final five-party read pass before beginning
implementation. No roadmap or code changes should be inferred from this
section beyond those already committed in the capability-roadmap commit.


#### Grok — 2026-08-10 (admin complete + build start)

- Verified owner admin report (filled) and Chat capability roadmaps (`memory`, `communication`, `platform`, `ui`, `dashboard`, `infrastructure`) + moon Gantt index.
- Verified infra prune (docker/terraform/ansible/firebase retained; heavy stacks removed); `docs/ROADMAP.md` gone; language roadmaps gone.
- Updated this shared report: priorities, provenance, conflicts resolved, §9 owner decisions, checklist, Grok structure **Yes**.
- Left §1.1 overall assessment prose for owner fill.
- Started implementation: `crates/ca-hub` SQLite store + `ca` CLI (M1/C1/C2 spine).

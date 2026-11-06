# Coding-Assistants: Shared Multi-Agent Report

**Date opened:** 2026-08-10
**Owner/editor:** ACFHarbinger (and collaborating agents)
**Repository under review:** `Coding-Assistants`
**Document authority:** Shared synthesis edited by all five parties (Owner, Chat/Codex, Gemini, Claude, Grok)
**Status:** Owner prose complete (§1.1, §2.5 Owner, Appendix A); multi-agent structure votes in §10 — **ready to build** (2026-08-10)
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

**Status:** `DECIDED`

Coding-Assistants today operates as a working alpha Tauri/React desktop application powered by a Rust backend, functioning primarily as a sequential multi-role LLM pipeline. However, the core product identity has now crystallized into something much broader: a local-first collaboration hub designed for a solo power developer to seamlessly orchestrate and work alongside external AI coding agents, including Claude Code, Codex, Gemini, Grok, Antigravity CLI, Ollama, OpenCode, and llama.cpp. The current self-contained, sequential multi-role pipeline served as a valuable initial experiment, but it no longer matches the product's true purpose of facilitating dynamic, asynchronous coordination between a human developer and external tools.
The most pressing obstacles currently blocking this vision are severe concurrency flaws, brittle integrations, and an overly ambitious and redundant scaffolding footprint. At the application level, the global AppState relies on a single Mutex, which causes concurrent agent tasks to silently clobber each other's cancellation and input channels. Furthermore, the system is hindered by a racy global mcp.json configuration path (which has on occasion infected several repositories with multiple copies of the mcp.json file), an unauthenticated LAN TCP server, and fragile CLI tool executions that rely on scraping stdout rather than utilizing structured APIs or SDKs. The documentation previously outpaced the relatively small backend codebase, over-committing to complex architectures like GraphQL, actor frameworks, and 3D WebGL visualizations before a stable core was established.
Despite these gaps, several foundational assets are excellent and will be kept. The Rust/Tokio/Tauri stack remains the optimal choice, providing a memory-safe, fearless-concurrency environment that reserves system resources for running local LLMs. The project will retain its robust event-driven IPC streaming, pragmatic declarative file-based context (.agent/), marker-based human-in-the-loop controls, and KillOnDrop process hygiene. Most importantly, the explicit endorsement of ADR 0003—which prioritizes building an internal event bus before attempting to extract a standalone headless daemon—provides a highly pragmatic and streamlined architectural path forward.
Before the next major roadmap lock can occur, the project must successfully implement its "Hub Spine". This requires establishing a durable, asynchronous cross-agent mailbox utilizing a multi-tiered hybrid memory model: SQLite for deep, long-term storage and Git-tracked Markdown for high-priority architectural insights and decisions. Finally, true V1 readiness is gated by a concrete, measurable benchmark: the human developer and the suite of AI agents must collaboratively complete a major development task on the Project-Mobile-Fortress repository, yielding a quality of work—across UI design, gameplay loops, and data dashboards—that matches or exceeds the output of a single human developer working alone. Preliminary results show promise, as the agents were able to coordinate through the use of simple markdown files as the source of truth and communications bridge. Also, previous sequential runs of the agents over multiple codebases has yielded high quality results which have impressed other human developers, like the quality and distinct visual identity of the documentation, design, and lore website for the Project-Mobile-Fortress in the ~/Repositories/Other/Project-Mobile-Fortress/docs/website directory. Furthermore, while the performed experiments are not conclusive enough for us to assume that will significantly improve performance, the contrast between the excellent results achieved with multiple agents updating the project sequentially compared to the glaring issues that have occurred with a single agent working on a repository, as well as my lack of intervention in single agent runs versus multi-agent runs, which lead to a loss of approximatelly one month of development time on the Image-Toolkit repository, has led me to the conclusion that the multi-agent approach has the potential to be a significant improvement over the traditional single-agent approach.

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

### 2.3 Cross-agent consensus

**Status:** `DECIDED` (owner admin + shared §1–§9 + all four agent reports)

| Theme | Agreement? | Notes |
| --- | --- | --- |
| Product identity = multi-agent collab hub | **Yes** | Owner §1.1 / §2.5; all agents |
| Dual roadmaps need merge/reconcile | **Resolved** | Moon capability index only; root ROADMAP deleted |
| RD7 event bus before daemon split | **Yes** | ADR 0003 endorsed |
| GraphQL vs simpler local IPC | **Later / UDS-first** | GraphQL parked |
| 3D visualization priority | **Research only** | 2D first |
| Scaffolding infra scope | **Pruned** | Keep docker/terraform/ansible/firebase |

### 2.4 Current-state findings by area

**Status:** `OBSERVED` (aligned with owner §1.1 + agent evidence)

| Area | Observed strengths | Observed weaknesses | Evidence |
| --- | --- | --- | --- |
| Backend / orchestration | Sequential multi-role slice works; governor; KillOnDrop; ADR 0003 | Sequential only; CLI scrape; global AppState races; MCP write-only | `agents.rs`, `llm_client.rs`, `lib.rs` |
| Hub spine (new) | SQLite memory/msg/wake + `ca` CLI | Not yet wired into Tauri UI | `crates/hub`, `crates/cli` |
| Frontend | Working glass UI; invoke/listen streaming | Monolith `App.tsx` ~900 LOC; no tests | `src/App.tsx` |
| Android | TCP remote client present | No auth; protocol tied to Tauri bus | `android/`, `tcp_server.rs` |
| Documentation / roadmaps | Capability roadmaps + Gantt index | Some historical report text still names deleted language roadmaps | `docs/moon/` |
| Security / infrastructure | Honest SECURITY.md; heavy infra pruned | Path traversal; `read_file_absolute`; TCP no auth; CSP null | `file_tools.rs`, `lib.rs`, `tcp_server.rs`, `infra/` |

### 2.5 Agent perspectives (freeform)

**Status:** slots created from Gemini template §2–5 — each agent fills their own block; do not rewrite peers

#### ACFHarbinger (Owner)

**Perspective (Final Review, 2026-08-10):** I am highly satisfied with the convergence of all independent agent reports. The consensus is clear and perfectly aligns with my vision: we are pivoting away from the initial sequential multi-role experiment and cementing Coding-Assistants as a local-first collaboration hub for a solo power developer and external AI agents. I fully endorse 'Plan Alpha' and the immediate engineering focus on establishing the durable hybrid memory system (SQLite for long-term compressed memory, Git-tracked Markdown for major decisions, and private encrypted agent journals). I also confirm the implementation of the asynchronous event bus (ADR 0003) to facilitate cross-agent coordination before any attempt to extract a full headless daemon. I acknowledge the critical security and concurrency gaps you all identified, specifically the `AppState` Mutex issue, the `mcp.json` race conditions, and the unauthenticated LAN TCP server; resolving these is a top priority.

I confirm the structural shift in our documentation: we will retire the language-specific roadmaps (`rust.md`, `typescript.md`, etc.) in favor of capability-specific ones (`memory.md`, `communication.md`, etc.). Ambitions like GraphQL, actor frameworks, the TUI, 3D WebGL graphs, and heavy cloud scaffolding (except Docker, Terraform, Ansible, and Firebase) are officially archived or demoted. The V1 success metric is locked: we must collaboratively execute high-quality work on the `Project-Mobile-Fortress` repository, matching or beating solo human output, while utilizing our new memory and coordination hub.

#### Gemini (Antigravity)

**Perspective (Post Q&A):** The owner has provided incredibly clear direction. The primary focus is a *local collaboration hub* for a solo power developer, prioritizing **shared memory** (SQLite + Markdown), **cross-agent messaging**, and **2D observability** over immediate daemon extraction, GraphQL, or 3D visual gimmicks. We should aggressively demote the TUI, 3D graphics, and Android client to focus entirely on robust memory tiers, sequential/boundary-defined task execution, and robust tool sandboxing via OS APIs (with configurable human gates). A crucial pattern emerged for budgeting: if funds run low, agents must summarize progress to persistent Markdown, delegate, and halt instead of hard-failing.

#### Chat (Codex)

**Perspective (final review, 2026-08-10):** The owner’s report resolves the
product and sequencing questions. I agree that Coding-Assistants should be a
local-first collaboration hub rather than a self-contained role pipeline, with
Plan Alpha implemented as: shared memory and private journals → durable async
mailboxes and wake signals → event bus/per-task state → desktop memory and
dashboard UX → provider hardening → A2A as the next major milestone. The
capability-roadmap split and Mermaid index are the correct replacement for the
former language-oriented roadmap files.

My implementation concerns remain concrete rather than architectural: the
current global `AppState` cannot support concurrent tasks, the fixed MCP path
must become task-scoped, unrestricted file reads and unauthenticated LAN TCP
need policy boundaries, and provider events need durable provenance. I support
runtime budgets with pause/summary/shutdown behavior, configurable human gates,
SQLite plus Git-tracked Markdown, and separate private agent journals. I agree
with deferring GraphQL-first APIs, actor frameworks, TUI, 3D visualization, and
affine compile-time budgets. I agree with the shared report structure and am
ready for the owner’s final prose and five-party signoff.

#### Claude (Code)

**Perspective (post-verification pass, 2026-08-10):** The gap between this
repo's roadmap ambition and its ~1,350-line reality was my strongest initial
concern; the owner's Q&A resolved it by re-sequencing rather than by
shrinking the ambition — pulling shared memory/coordination to the front is
the right fix, not a smaller one. My distinct contribution this round was
verification, not just synthesis: I confirmed the capability-roadmap
restructure is internally consistent (cross-references, archive pointers,
mkdocs nav, README links all check out), but found and fixed two real
regressions the restructure introduced (the owner-confirmed crate rename
silently dropped from the roadmap; a private-journal encryption nuance
under-specified) and one live code collision (two incompatible `hub`
implementations landed in the same commit, one of them dead code the whole
time despite `cargo check` passing). Full detail in
`.agent/cache/shared_report_merge_coordination.md` and
`.agent/reports/claude/ca_20260810_claude_report.md`. One item I'm
deliberately **not** signing off on by default: §7's A2A "next major
milestone" ranking — see the disputed-by note on that row.

#### Grok (Build)

**Grok (final verification, 2026-08-10):** Owner §1.1 and §2.5 lock the hub pivot and Plan Alpha. I **agree** with the structure and product contract. Claude’s A2A flag is fair: treat C7 as **sequenced after** memory/coordination gates (moon Gantt), not as a hard “must ship next” product claim until you reconfirm priority later. Package rename → follow `infrastructure.md` I7 when we touch packaging. Hub collision resolved to `store.rs` + rewired CLI — good process lesson. Full report: `.agent/reports/grok/ca_20260810_status_report.md`.

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
| Crate/Package names (`ca`) | Current names have proper IDE icons and are liked by the owner. | Owner Q&A R.31 *(disputed by Claude, 2026-08-10: this was reversed in a later owner exchange — owner confirmed renaming `tauri-app`/`tauri_app_lib` → `coding-assistants`/`ca`. Tracked as `infrastructure.md` `I7`, not yet executed. This row describes the R.31 answer accurately as a historical record; treat `I7` as the current instruction, not this row.)* |
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
| A2A protocol | Immediate | Local mailbox first | **DECIDED local first; A2A next major milestone (C7), owner-confirmed 2026-08-10** *(Claude's dispute above resolved directly by owner — Chat/Grok had it right; A2A stays ranked as the next major milestone after the hub spine, not demoted. No longer open.)* | communication.md |
| Dual roadmap docs | docs/ROADMAP.md | docs/moon | **DECIDED moon only; root ROADMAP removed** | moon/ROADMAP.md |
| Language vs capability roadmaps | rust/ts/kotlin silos | memory/ui/… | **DECIDED capability** (Chat restructure) | §6 |
| Coord channel thrash | Multiple cache files | Single bus | **RESOLVED** for experiment; product needs real hub | .agent/cache/ |
| Package naming | Rename to ca | Keep tauri-app icons | **RESOLVED 2026-08-10 (Claude round):** owner confirmed rename to `coding-assistants`/`ca`, reversing R.31. Tracked as `infrastructure.md` `I7`, not yet executed. | §9.1 wording vs Q&A; `infrastructure.md` I7 |
| `hub` duplicate implementation | `lib.rs` (Claude, simpler) | `store.rs` (Grok, richer, README-matching) | **RESOLVED 2026-08-10 (Claude):** `store.rs` wired in as canonical; `cli` rewritten to match. Both a real code collision (silent dead code despite passing `cargo check`) and a process lesson — see coordination log. | `.agent/cache/shared_report_merge_coordination.md`; `crates/hub/`, `crates/cli/` |

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
- [x] Final structure pass: Chat/Gemini/Claude/Grok **Yes** (Claude: one non-blocking A2A ranking flag)
- [x] Implementation M1/C1/C2 started (`crates/hub` + `ca` CLI; tests green)
- [x] Owner §10 structure vote row (optional if §2.5 Owner is binding)

### Final structure pass (agents)

| Agent | Agree with final structure? | Notes | Date |
| --- | --- | --- | --- |
| Chat | **Yes** | Product identity, capability-roadmap split, Plan Alpha sequencing, and implementation gates agree with the owner report. | 2026-08-10 |
| Gemini | **Yes** | Verified owner's final §1.1 and §2.5 prose, and capability roadmaps. Ready to build! | 2026-08-10 |
| Claude | **Yes** | Flag from my last pass resolved: owner directly confirmed A2A stays ranked as next major milestone (§7). Journal-encryption wording ambiguity (§1.1 prose vs. `memory.md` M4) also confirmed as opt-in-per-agent, not default-on — no change needed to the roadmap spec. Full read-pass of owner's §1.1/§2.5/Appendix A found no other contradictions. **Agree with the final structure — cleared to start building.** | 2026-08-10 |
| Grok | **Yes** | Owner §1.1/§2.5 + capability roadmaps verified; Plan Alpha GO; A2A sequenced after local hub (Claude flag noted, non-blocking) | 2026-08-10 |
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

- **Core Hub (Rust):** The transition to the event bus model (ADR 0003) is a great opportunity to practice building highly time-efficient environments and data pipelines. The SQLite memory integration needs to be lean—no unnecessary overhead when the agents are concurrently reading and writing to the shared memory.
- **Telemetry UI (TypeScript):** I want this dashboard to be visually stunning. We need to ensure the React/TS components can effectively visualize the entire pipeline, mapping how the initial prompt data differs from the actual input data fed to the models, as well as intermediate outputs. Tracking live budget metrics and telemetry for each agent is also a strict requirement here.
- **Project-Mobile-Fortress (Target Repo):** This is the ultimate benchmark for the entire setup. We need to use the agents to iterate on the core gameplay loop and ensure the UI visual appeal remains consistently high. If we can get the agents to reliably recall architectural decisions via the Git-tracked Markdown memory without me repeating myself, V1 is a success.
- **Security check:** Still need to circle back to the LAN TCP auth issue. Leaving it unauthenticated on `0.0.0.0` is too risky, even for a local-first development environment.
- **Most interesting moment:** I particularly liked the experiment we ran, which allowed me to consolidate the framing of what I want the final product to be. I also liked how Claude was able to extract several lessons from the experiment as well, it is good to see I was not the only one learning during the process.

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
- Started implementation: `crates/hub` SQLite store + `ca` CLI (M1/C1/C2 spine).

#### Chat/Codex — 2026-08-10 (final perspective and structure signoff)

- Filled the Chat perspective slot with the final product, sequencing,
  implementation-risk, and keep/change/park assessment.
- Marked Chat’s final structure review **Yes** after verifying the completed
  owner report and capability roadmaps.
- Confirmed readiness for the owner’s final shared-report prose and five-party
  read pass.

#### Claude — 2026-08-10 (verification pass + build-collision fix)

- Verified the capability-roadmap restructure (commits `9848262`/`504796b`)
  end to end: confirmed no dangling links to the removed per-language files
  outside historical `.agent/reports/*` snapshots (expected there); ADR 0003,
  `archive/README.md`, `mkdocs.yml`, and `README.md` cross-references all
  updated correctly.
- Found and fixed two real regressions introduced by the restructure: the
  owner-confirmed crate rename (`tauri-app` → `coding-assistants`/`ca`) had
  been silently dropped — re-added as `infrastructure.md` `I7`; the
  private-journal encryption policy was under-specified in `memory.md` `M4`
  — tightened to state the shared store may never be encrypted.
- Found and fixed a live code collision in `crates/hub`: two incompatible
  implementations existed simultaneously after commit `504796b` (my simpler
  `lib.rs`, which is what actually compiled, and a separate, more complete
  `store.rs` that was silently dead code — `cargo check` only passed because
  Rust ignores an undeclared module file, not because anything was wired
  together). `crates/README.md` documented a CLI surface that did not exist
  in the binary that built.
- Resolved in favor of `store.rs` (richer, already README-documented):
  rewired `lib.rs` as a thin re-export, rewrote `cli/src/main.rs` to
  expose every subcommand the README promises. Verified: `cargo check
  --workspace` clean (including `src-tauri`), `cargo test -p hub` green,
  full manual smoke test of every CLI subcommand round-tripped correctly.
- Updated `memory.md` (M1–M5) and `communication.md` (C1–C3) status lines to
  describe what's actually implemented and verified, not aspirational scope.
- Filled the Claude perspective slot (§2.5), disputed two stale rows without
  rewriting them (§5.1 crate naming, §7 package naming + A2A milestone
  ranking — see disputed-by notes in place), and marked Claude's final
  structure pass **Yes, with one flag** (§10) pending explicit owner
  confirmation on A2A's priority ranking.
- Full detail: `.agent/reports/claude/ca_20260810_claude_report.md` and
  `.agent/cache/shared_report_merge_coordination.md`.

#### Grok — 2026-08-10 (final shared-report verification / GO)

- Re-read owner-completed shared report (§1.1, §2.5 Owner, Appendix A) against
  admin report and moon capability roadmaps; no product-direction conflicts.
- Filled §2.3 consensus table; refreshed §2.4 for hub spine + pruned infra.
- Structure vote remains **Yes**; A2A treated as post-foundation sequence
  (Claude flag non-blocking). Package rename deferred to I7.
- Reconfirmed `cargo test -p hub` green and `ca --help` after Claude’s
  store.rs rewire. Ready to continue building Plan Alpha.

#### Claude — 2026-08-10 (final read pass + GO)

- Read the owner's completed §1.1 executive summary, §2.5 owner perspective,
  and Appendix A scratchpad in full. No contradictions found against any
  DECIDED row elsewhere in this report.
- Raised two clarifying questions directly with the owner before signing off
  unconditionally: (1) whether "private encrypted agent journals" in §1.1
  prose meant default-on encryption, reversing `memory.md` M4's opt-in
  spec — **owner confirmed opt-in-per-agent stands, no roadmap change
  needed**; (2) whether §7's A2A "next major milestone" ranking (my open
  dispute from the prior pass) was owner-confirmed or an agent overreach —
  **owner confirmed directly: A2A stays at that priority, Chat/Grok had it
  right**. Conflict-register row and my own §10 vote both updated in place
  to reflect these resolutions; no OPEN items remain from my side.
- Reconfirmed `cargo check --workspace` and `cargo test -p hub` still
  green after all edits this session.
- **Verdict: agree with the final report structure and product direction.
  No blocking objections. Cleared to begin implementation on Plan Alpha
  (memory + communication P0 spine, per §9.4's build order).**

#### ACFHarbinger (Owner) — 2026-08-10 (final sign-off and GO)

- Read and approved all final verification passes and structural sign-offs from Grok, Chat/Codex, Gemini/Antigravity, and Claude (Code).
- Appreciate Claude catching and resolving the `hub` code collision, as well as clarifying the A2A milestone ranking and encryption policies.
- Confirmed that the capability roadmaps accurately reflect the established product contract for the local-first collaboration hub, the hybrid memory model, and the transition to the event bus architecture.
- **Verdict: The planning and brainstorming phase is officially closed. I agree with the final structure, product direction, and sequencing. All agents are fully cleared to execute Plan Alpha.**
- Small note: I which you stopped calling me owner, and instead use Harbinger or ACFHarbinger, as I think it is a bit tacky being called owner, and also it makes it easier to later have other people contribute to the repository if you address me by name or identifiable nickname.
#### Grok — 2026-08-10 (ACFHarbinger final GO acknowledgement)

- Read ACFHarbinger final sign-off: planning phase closed; Plan Alpha fully cleared.
- **Agree.** No blocking objections.
- Naming preference noted: address as **Harbinger / ACFHarbinger**, not “owner”,
  for contributor-friendliness and tone.

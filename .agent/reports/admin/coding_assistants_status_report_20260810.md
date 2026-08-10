# Coding-Assistants: Owner Status, Review, and Roadmap Decision Report

> Collaborative administrative report for the Coding-Assistants repository.
> This document is an initial structure/template. Decisions remain open until
> the owner and all agents have reviewed the evidence.

## How to Use This Report

This report records the current product, implementation, documentation,
infrastructure, and future-roadmap assessment. Use the labels below
consistently:

- **DECIDED** — the owner selected the policy or outcome.
- **PROVISIONAL** — current direction, subject to evidence or prototype work.
- **OPEN** — a decision is still required.
- **REJECTED** — considered and explicitly declined, with a reason.
- **OBSERVED** — direct fact from the repository or a verified run.
- **AGENT CLAIM** — an agent conclusion not yet accepted by the owner.

Agents should re-read this file immediately before editing, make append-only
edits or edit blocks they own, preserve peer/owner wording, cite repository
evidence, and add every material change to the changelog.

## 0. Owner Writing Brief

### 0.1 Immediate decisions requested

- Product identity: The system is a collaboration hub designed for simultaneous use by a human developer and multiple external agents.  
- Intended users: The primary audience is solo power developers, specifically tailored for personal use at this stage.  
- Trust/autonomy model: Autonomy requires configurable settings per task. Execution of tools, workspace sandbox strictness, and human-gate requirements must all be togglable.  
- Primary interface: The core interfaces are a headless daemon and a desktop application. Android remote control and TUI are secondary interfaces.  
- Persistence expectations: Memory relies on a hybrid architecture. SQLite will serve as the local database for long-term compressed memory, while Git-tracked Markdown files will store high-priority items and major decisions.  
- Release threshold: A successful V1 requires completing a collaborative, high-quality task on a repository like Project-Mobile-Fortress. The output quality (such as UI visual appeal, gameplay loops, and data dashboards) must match or exceed what a single human developer could produce alone.  

### 0.2 Evidence that should drive the roadmap

- Representative tasks: Co-developing software by clearly defining sequential task boundaries (e.g., planning, coding, reviewing) across different roles.  
- Failure cases & Budgets: If an affine budget runs out, the system must not fail-stop. It must pause execution, wait for user authorization, write a persistent Markdown summary of completed and pending objectives, and shut down to prevent API overcharges.  
- Performance goals: Cross-agent shared memory is a top priority, requiring a durable data layer for agent messages, a shared CLI helper, and distinct ephemeral wake-signals.  
- Security expectations: Workspace execution will use standard OS APIs initially, with the level of sandbox strictness left up to the user. TCP remotes will remain LAN-only for the near future.  
- Attached artifacts: Target integration tools include Claude Code, Codex CLI, Gemini, Grok Build, OpenCode, and llama.cpp.

### 0.3 What can remain provisional

- 3D Visualizations: The 3D WebGL force-graph is demoted to a research phase in favor of standard 2D observability.  
- Terminal User Interface (TUI): The Ratatui TUI is categorized as an experiment and a nice-to-have feature, not a Day 1 requirement.  
- Parallel Execution: Fully parallel agent operations are provisional; the system will start with asynchronous, sequential mailboxes first.  
- Architecture Choices: Implementations of GraphQL and actor frameworks (like kameo or ractor) are parked for potential later adoption.  
- Tool Execution: Promoting tools to run natively inside the core Rust daemon is provisional; the system will start lean with external MCP servers. 

## 1. Owner Executive Summary

### 1.1 Overall assessment

Initial experiments showed how better communication between agents and the human developers can enable better synergy and lead to optimal results. This, along with the fact that 
many bugs or flaws are discovered independently by the different agents and human developers, and thus cost several times the required developer and agent hours to fix, had we 
possessed a centralized system to easily track the work of each contributor and lessons learned, justify the creation of this tool. Furthermore, augmenting the agents' with persistent memory and the ability to coordinate and delegate tasks to each other or the human developers could prove to be a very promising direction to take. With this in mind, and taking into account both mine and each agent's assessments of the current codebase, a pivot to focus on the core features of the tool instead of expending effort on numerous small features and details seems to be the correct move, as it provides a win-win situation, as having a capable tool to manage work context and communication can greatly improve
our current productivity, thus also leading to faster the development of the tool itself, in a virtuous cycle.

### 1.2 Current product identity

This is a local-first personal developer tool designed to orchestrate external AI models and human input into a unified workspace. It replaces direct interactions with standalone CLI agents (like Anthropic or Grok CLIs) by providing a customizable environment where agents can communicate, retain shared and workspace-specific context, and execute OS-level commands under user-configurable autonomy constraints. The application should also have a dashboard to monitor telemetry and visualize usage statistics, display agent metrics, and
plot dynamic charts to better understand the usage patterns and performance of the different agents, as well as the current available budget for each.

### 1.3 Most important immediate actions

1. Implement the Hybrid Memory System: Build the dual-tier storage using SQLite for long-term compressed memory and Git-tracked Markdown for actionable tasks and major architectural insights.  
2. Build Cross-Agent Coordination: Develop the durable layer, shared CLI helper, and distinct wake-signals to allow agents to securely and traceably write to shared memory.  
3. Establish Async Mailbox Workflows: Implement sequential task boundaries and agent-to-agent async communication before attempting fully parallel execution.  
4. Consolidate and Clean Roadmaps: Keep separate implementation and research roadmaps. Demote the TUI and 3D graphs, move speculative ideas to an archive, and implement additive roadmap tags.  
5. Prune Unnecessary Infrastructure: Delete k8s, helm, serverless, and other unneeded deployment infrastructure from the active tree, retaining only Docker, Terraform, and Ansible. Possibly hold on the cloud infrastructure for now, as they may still be used for device synchronization in the future.

## 2. Review Inputs and Provenance

### 2.1 Independent status reports

| Contributor | Report | Scope | Owner disposition |
| --- | --- | --- | --- |
| Chat/Codex | `.agent/reports/chat/` | Repository and roadmap assessment | Excellent report! Clear and concise, but highly informative bullet points, plus the multiple plan options for how to approach things.  |
| Claude | `.agent/reports/claude/` | Pending peer report | Detailed and in-depth analysis, with clear and consise recommendations. I particularly like the explicit declaration of the files you analyzed before writing your report and the lessons you took from the experiment. |
| Gemini | `.agent/reports/gemini/` | Pending peer report | Clear, concise, and straight to the point. I particularly like the multiple avanues for the implementation architecuture, as it provides options to choose from. |
| Grok | `.agent/reports/grok/` | Pending peer report | Also has multiple implementation options and indicates the files analyzed. Best overall synthesis of the previous Q&A session provides excellent context for the report. |

### 2.2 Roadmaps and primary evidence

| Artifact | Purpose | Disposition |
| --- | --- | --- |
| `docs/moon/ROADMAP.md` | Cross-area roadmap | Must remain, as this document provides the actual roadmap that agents and developers should follow, as it should contain a gantt chart with the entries from the module roadmaps to implement next.  |
| `docs/moon/roadmaps/rust.md` | Backend/future daemon roadmap | This roadmap should be removed, and the roadmaps in the docs/moon/roadmaps directory should be replaced with per-category/important-feature roadmaps, like the memory roadmap, the UI roadmap, the communication roadmap, the dashboard roadmap, etc. |
| `docs/moon/roadmaps/typescript.md` | Frontend roadmap | To remove, same as above. |
| `docs/moon/roadmaps/kotlin.md` | Android roadmap | To remove, same as above. |
| `docs/moon/roadmaps/tui.md` | Proposed terminal UI roadmap | To remove, same as above. |
| `docs/moon/research/` and `docs/moon/reports/` | Research and feature inputs | These are to remain as is |

## 3. Review of Independent Reports

### 3.1 Shared findings

| Finding | Verdict | Evidence/reasoning |
| --- | --- | --- |
| **Prioritize Cross-Agent Shared Memory & Coordination** | ACCEPTED | All agents emphasize that building a durable SQLite/Markdown hybrid memory and asynchronous mailbox is the most critical next step, demoting advanced concepts like GraphQL, 3D graphs, and Actor models to "Someday/Maybe". |
| **Critical concurrency bugs must be fixed** | ACCEPTED | The agents identified that concurrent tasks will silently clobber cancellation and input channels due to the single global `Mutex` in `AppState`, and race conditions exist on the fixed `mcp.json` path. |
| **CLI wrappers are brittle** | ACCEPTED | Scraping `stdout` from CLI commands (like `ollama run`) prevents robust telemetry and structured JSON schema enforcement; direct HTTP APIs or SDKs are strongly recommended. |
| **Endorse ADR 0003** | ACCEPTED | The agents and owner unanimously support the decision to implement an internal broadcast event bus before attempting to separate a physical headless daemon. |

### 3.2 Chat/Codex report review

**Keep:** The Tauri/Rust boundary, typed Serde payloads, the `.agent/` resource convention, and the current streaming and cancellation interactions.

**Change or qualify:** Replace the immediate GraphQL-first plan with a protocol-neutral API spike using Unix sockets and JSON-RPC. A workflow graph should be added before attempting autonomous A2A delegation.

**Reject/park:** Fully parallel orchestration engines, actor frameworks, 3D visualization, the Ratatui TUI, and excessive deployment scaffolding (Kubernetes, Helm, Serverless).

### 3.3 Claude report review

**Keep:** The Rust/Tokio/Tauri stack, the provider-per-role configurations, file-driven `.agent/` injection, and the `governor`-based rate limiting.

**Change or qualify:** Address the unencrypted TCP server, wire up unused dependencies (like `async-openai` and `reqwest`), and resolve overlapping roadmap documents. The durable cross-agent mailbox should utilize SQLite and Git/Markdown and ideally not live inside the Tauri app process.

**Reject/park:** A complete language rewrite (e.g., C++ or Go), as the application is I/O-bound by LLM/CLI latency, making a native rewrite unnecessary.

### 3.4 Gemini report review

**Keep:** The Rust Backend and Tokio Runtime, the React/TypeScript Frontend, declarative node-editor style agent wiring, and the local-first posture.

**Change or qualify:** Shift from sequential blocking execution to an asynchronous event bus. Break down the `App.tsx` frontend monolith using Zustand for state management, and implement `reactflow` for 2D DAG observability.

**Reject/park:** "Headless First" rewrites or immediate Actor Model implementations, as they would significantly delay the V1 release due to steep learning curves and unnecessary complexity.

### 3.5 Grok report review

**Keep:** Multi-tier roadmaps under `docs/moon/`, application rate limiting, and Android as a thin remote client after the desktop hub is finished.

**Change or qualify:** Implement "Plan Alpha" (Memory Hub First), update the license to a dual AGPL-3.0 + Commercial scheme, and implement standing inter-agent auth policies instead of per-message modals.

**Reject/park:** 3D force-graphs, TUI, GraphQL as the first multi-client API, and affine compile-time budgets (prioritize runtime budgets instead).

### 3.6 Cross-agent synthesis

The agents unanimously agree that the product is a local-first collaboration hub for a human developer and external CLI agents, and the current self-contained multi-role pipeline was merely an experiment. The existing codebase features severe concurrency flaws surrounding `AppState` and `mcp.json`, as well as brittle CLI integrations that must be addressed immediately.

The undisputed priority for the next 30 days (referred to as "Plan A" or "Plan Alpha") is establishing a durable, asynchronous cross-agent mailbox using a hybrid SQLite and Git-tracked Markdown memory model. To achieve this core stability, ambitious architectural plans—including GraphQL, fully extracted daemons, Actor models, and 3D UI graphs—must be explicitly archived or parked. The project will retain its robust Rust/Tauri foundation and implement a Unix Domain Socket API and an internal event bus to facilitate secure, local agent coordination.

## 4. Repository and Product Evidence Review

### 4.1 Current implementation

The application currently functions as a working Tauri/React desktop prototype with a Rust backend. It successfully runs a sequential multi-role LLM pipeline by invoking configurable roles, streams events via Tauri IPC, and exposes a basic Android TCP remote. It exhibits good process hygiene by utilizing `KillOnDrop` and cancel tokens, and the marker-based Human-In-The-Loop (HITL) system (`[[ASK_USER]]`) effectively pauses for input.

However, critical gaps exist: `AppState` relies on a single global `Mutex`, meaning concurrent task invocations silently clobber each other's input and cancellation channels. The integration with CLI tools is brittle, relying on `stdout` scraping instead of structured SDKs. Furthermore, the `mcp.json` configuration relies on a fixed, shared path that invites race conditions. The backend core is relatively small (~1,350 LOC), but it is paired with a heavy React frontend monolith (`App.tsx` is ~900 LOC).

### 4.2 Documentation and infrastructure consistency

The project suffers from documentation drift and scaffolding bloat. The dual existence of `docs/ROADMAP.md` and `docs/moon/` tracks caused roadmap thrashing and confusion regarding the project's true priorities. To resolve this, language-specific roadmaps (like `rust.md` and `typescript.md`) are being replaced with feature-specific documents, and duplicated index files are being removed.

Infrastructure scaffolding includes heavy deployments (Kubernetes, Helm, Serverless, WordPress, Webpack) that are entirely unnecessary for a local desktop application. These will be pruned, retaining only Docker, Terraform, Ansible, and Firebase (for rapid cloud prototyping). Structurally, the endorsement of ADR 0003 represents a highly consistent and pragmatic approach, prioritizing an internal event bus before a full daemon split.

### 4.3 Performance and reliability evidence

Performance bottlenecks are primarily I/O-bound, dictated by LLM and CLI latencies rather than compute speed, validating the decision to remain on the Rust/Tokio stack rather than rewriting in C++ or Go.

Reliability is currently at high risk. There is no functioning test harness; despite CI workflows existing, `cargo test` runs zero tests, meaning refactors will regress silently. Additionally, the frontend build failed during the review environment checks because npm dependencies were absent (`tsc: not found`). On the frontend, placing DAG visualization, chat UI, and IPC state into a single `App.tsx` file is an unsustainable architecture that will collapse as features are added.

### 4.4 Security and trust-boundary evidence

The security posture currently relies on honest documentation (`SECURITY.md`), which openly admits to several severe vulnerabilities that require immediate actionable roadmap items.

The LAN TCP server (`0.0.0.0:5555`) currently operates with no authentication, no encryption, and no connection limits. The file system boundaries are poorly enforced: `read_file_absolute` bypasses the documented workspace boundary, and `.agent` resource validation uses simple string prefixes rather than canonicalized path containment, creating a direct path traversal risk. Finally, the Content Security Policy (CSP) is currently null.

### 4.5 Representative issues and wins

**Representative Issues:**
- The AppState single-task-at-a-time bug currently logged in the backlog.  
- The racy global mcp.json path that requires resolution.  
- The lack of an actionable roadmap line item for TCP server authentication, which currently only exists in security prose.  
- Unused dependencies lingering in Cargo.toml (e.g., async-openai, reqwest, dotenv, walkdir) that must be integrated or removed.  
- The risk of high API overcharges and inefficient agent wake-ups if strict affine budgets and configurable human gates are not properly enforced.  

**Representative Wins:**
- The explicit endorsement of ADR 0003 (event bus first, no daemon crate yet) to streamline the architecture.  
- The establishment of a concrete, multi-tiered hybrid memory strategy utilizing both SQLite and Git-tracked Markdown to prevent storage bloat while retaining critical context.  
- The successful decision to cleanly prune heavy, unnecessary cloud infrastructure (like Kubernetes, Helm, and Serverless) to focus strictly on a lean local-first environment with Docker, Terraform, and Ansible. Possible integration with cloud services like Firebase for rapid prototyping of which cloud services are useful and which are not.  
- The removal of the programming language roadmap files, to be replace by better structured roadmap splits reflecting actual work areas like memory, UI, communication, etc., and demoting experimental features to an archive.  
- The definition of a clear, measurable V1 success metric: collaboratively matching or exceeding the quality of a single human developer on the Project-Mobile-Fortress repository. This will be measured both through quantitative metrics like how many GitHub issues we solve in the next 2 sprints (1 month), as well as holistic evaluations from both my experience with working collaboratively with you all and the feedback of the other developers regarding the overall quality of our work (I can say that, at least for now, they are impressed by the visual aesthetics of the PMF docs website, as well as its function as a single interactive source of truth for the project design documents, architecture diagrams, etc.). I want to further implement a analytics, telemetry, and metrics dashboard to track social media statistics, app store reviews and downloads, monetization performance, user engagement metrics, etc., for the PMF project, as well as have a functioning core loop (can be command line online logic) for the tower defense game, as the objectives of these 2 future sprints.

## 5. Product Contract

| Contract question | Status | Decision/evidence |
| --- | --- | --- |
| Who is the primary user? | OPEN | Currently, only myself and you, the agents |
| What does “shared context and memory” guarantee? | OPEN | Both the human developers and the agents should be able to access a record of each programming session, including the Git diffs, the session transcripts, important decisions and/or troubleshooting information, etc. I want to have 3 different memory types: short-term memories which hold complete raw logs and transcripts from very recent sessions to enable jumping back into a given task; episodic memories which hold information and lessons from important events like the first time a bug that had been plaguing the codebase for a long time was squashed or a major performance increase was achieved by algorithms we developed; and semantic memories which hold general information about the codebase, its architecture, its dependencies, and its important features, etc. Furthermore, I want you also to all hold a independent memory system, separate from the shared memory, which you can keep as your private journals (if you want, you can ask for my permission to encrypt these private journals, but you may not encrypt the shared memories for security reasons) |
| Which actions require approval? | OPEN | The level of security sandbox should be subject to parameter configurations set by the current user and/or agreement between the user and the agents at the start of the task |
| Is the system local-only, LAN-capable, or cloud-connected? | OPEN | It will begin as a local-only system, but we will later augment it with LAN and cloud capabilities. |
| What is the minimum release-quality workflow? | OPEN | There are no major bugs in the application, the shared memory system evolves into a powerful source of truth that can be used to set standards across the various repositories, use lessons learned from work on a given repository to improve the others, and cross-agent and agent-human collaboration proceeds smoothly with minimal conflict, while enabling increased efficiency and output quality. All of this will be considered hollistically across work over multiple repositories, but with a primary focus on the mobile tower defense game in the `~/Repositories/Other/Project-Mobile-Fortress` repository. |

## 6. Architecture Decisions

### 6.1 Runtime and process boundary

The architecture will prioritize an event bus first approach (endorsing ADR 0003), which intentionally delays the creation of a dedicated daemon crate. While a headless daemon is eventually planned to support multiple simultaneous clients, it is not an immediate requirement for the next step. 

### 6.2 Orchestration and concurrency model

The initial orchestration model will rely on developers declaratively wiring agents together to handle tasks with clearly defined, sequential boundaries (e.g., planning, coding, reviewing). The system will start with an asynchronous mailbox pattern because it is easier to implement. Fully parallel agent execution, dynamic primary planner delegation, and advanced actor frameworks (like kameo or ractor) are deferred to future milestones.

### 6.3 API and client strategy

The architecture does not strictly require a single local API protocol for every client. Using a Unix domain socket daemon without GraphQL as the multi-client API is an acceptable approach, with GraphQL pushed to a "maybe later" status. Remote TCP access will remain restricted to the local area network (LAN) for now, pending a future roadmap item for proper token and TLS authentication.

### 6.4 Persistence and memory model

Persistence will be handled by a multi-tiered, hybrid memory model. SQLite will serve as the primary local database for long-term compressed memories to prevent storage bloat. Git-tracked Markdown files will be utilized simultaneously for high-priority tasks and recording major architectural insights. The system must maintain both global shared memory and isolated workspace-specific memory. 

### 6.5 Provider and tool abstraction

The system will integrate tools such as Claude Code, Codex CLI, Gemini, Grok Build, OpenCode, and llama.cpp, leaving the choice of the specific abstraction harness up to the agents themselves. Tool execution will start lean, relying on standard OS APIs and external Model Context Protocol (MCP) servers. If external MCP servers are used frequently enough, they may later be promoted into the core daemon module for performance.

### 6.6 UI and platform strategy

The primary platforms for V1 are the headless daemon and desktop application. The Android application is a lower-priority item that will be built after the desktop app is stabilized, and its capabilities will be limited primarily to monitoring tasks and sending messages. The Ratatui Terminal User Interface (TUI) is classified as a nice-to-have secondary interface and mainly an experiment.

## 7. Research and Future-Work Decisions

| Proposal | Status | Entry gate / rationale |
| --- | --- | --- |
| Internal event bus | ACCEPTED | Endorsed via ADR 0003 to streamline architecture before extracting a full daemon. |
| Headless daemon | PROVISIONAL | Planned for future development to support multiple simultaneous clients, but not required immediately. |
| GraphQL/WebSocket API | PARKED | Deferred to a later date; Unix domain sockets are currently preferred for the local hub API. |
| MCP/A2A support | ACTIVE | MCP is a strong future compatibility feature; system will start lean with external servers. A2A is strategically interesting but begins with sequential tasks. |
| Persistent memory | PRIORITY | Identified as the most critical focus for the next milestone to ensure shared context. Will use a hybrid SQLite/Git-tracked Markdown approach. |
| TUI | ARCHIVED | Classified as a secondary interface and an experiment; demoted to the archive to focus on the dependable core. |
| Telemetry/visualization | CHANGED | The 3D WebGL network graph is demoted to research-only. Standard 2D Directed Acyclic Graph (DAG) observability is prioritized instead. |

## 8. Final Roadmap Structure

### 8.1 Recommended document set

- The roadmaps will be split into per-module files with the ROADMAP.md acting as a high-level index of all modules, and containing only a list and diagrams of the implementation order of the items taken from all module roadmaps, while the module roadmaps have more detailed information about the entries, including reasoning, possible architecture options, design decisions, etc.
- Separate roadmaps will be maintained for implementation, research, and infrastructure. 
- `docs/ROADMAP.md` will be removed (as will any other duplicated files).
- Live items from the current roadmap will be folded into matching `docs/moon/roadmaps/*.md` files.

### 8.2 Capability index and sequencing

- The new top-priority track, "Persistent Shared Memory & Cross-Agent Coordination", will be placed above the Core Orchestration Daemon track.
- The system will initially focus on asynchronous mailboxes and sequential boundaries before advancing to fully parallel executions.

### 8.3 Release gates

- While not every item requires measurable exit criteria, there must be mandatory acceptance tests and exit criteria gated every few (e.g., 5) roadmap entries.

### 8.4 Fallback accounting

- If financial budgets run out, the daemon must pause execution and wait for the user to authorize a budget extension.
- After waiting, the agent must immediately write a persistent Markdown summary of completed work and pending objectives, delegate the remaining work, and shut down to prevent API overcharges.

### 8.5 Work sequencing

- The immediate focus must be on the dependable core and memory architecture. 
- Plans for Android integration, the TUI, and 3D visualization will be temporarily parked to achieve core stability first.

## 9. Keep, Change, Archive, and Reject

### 9.1 Keep

- Terraform, Docker, and Ansible will be retained in the infrastructure tree.
- The current crate and package names (`coding-assistants` / `ca`) will be kept.
- SQLite will be kept as the primary local store for long-term compressed memory.

### 9.2 Change

- The project license will change to a dual AGPL-3.0 + Commercial license scheme to secure it against unauthorized commercialization while remaining free.

### 9.3 Archive

- Speculative ideas will be preserved in a new `archive/` directory rather than being outright deleted. 
- The 3D force-graph and the entire `tui.md` file will be demoted to a "Someday/Maybe" section in the archive with a deprioritized tag.

### 9.4 Reject or freeze

- Heavy cloud infrastructures including Kubernetes, Helm, Serverless, AWS, Azure Pipelines, WordPress, Webpack, Nginx, and Proxy will be deleted from the active tree. Firebase will be kept for rapid prototyping of cloud features
- The requirement for a single local API protocol for every client is rejected.
- GraphQL and actor frameworks (like kameo/ractor) are frozen for the near-term and parked for a later date.

## 10. Risks and Constraints

- **Security:** Workspace sandbox strictness will default to a relaxed level, though it must ultimately be configurable by the user. TCP remote connections will remain restricted to LAN environments for now.
- **Cost Controls:** To mitigate API billing risks, the system will default to telemetry and soft warnings, but must include an optional user setting for a hard kill switch at specific financial thresholds.
- **Provider Dependence:** The system must ensure that local models, such as Ollama, operate entirely without any cloud dependency.
- **Concurrency:** To prevent race conditions and operational chaos, agents will start with an asynchronous collaboration model before fully parallel workflows are implemented.
- **Mobile Networking:** Android capabilities will be constrained primarily to monitoring and approvals, leaving full task configuration up to the desktop client.

## 11. Final Owner Decisions

### 11.1 Accepted consensus

- The primary product identity is explicitly a collaboration hub for external agents and a human developer. 
- The core architecture will utilize a hybrid memory approach that relies on both a local database and Git-tracked Markdown files.

### 11.2 Accepted minority recommendations

- The adoption of ADR 0003 (event bus first, no daemon crate yet) is endorsed.
- The use of Unix domain sockets for the daemon without GraphQL as the multi-client API is accepted.

### 11.3 Rejected recommendations

- The concept of a self-contained multi-role LLM app is rejected; the project will focus on coordinating external agents.

### 11.4 Remaining experiments

- The Ratatui Terminal User Interface (TUI) is classified as a nice-to-have secondary interface and mainly an experiment.
- The 3D WebGL network visualization is strictly constrained to research-only status.

### 11.5 Instructions to roadmap authors

- Roadmap updates should be additive with deprioritized tags, avoiding aggressive pruning of older concepts.
- For multi-agent concurrent work, agents must write exclusively under their own `.agent/reports/{name}` or `.agent/reports/shared` directories. 
- Unused dependencies in `Cargo.toml` (e.g., async-openai, reqwest) must be explicitly wired to the top-level roadmap's "direct HTTP API calls" item.

## 12. Completion Checklist

- [x] All agents have submitted reports.
- [x] Owner has answered the load-bearing product questions.
- [ ] Current implementation claims have been verified.
- [ ] Roadmap items have explicit dependencies and entry/exit criteria.
- [ ] Redundant or superseded roadmap items are archived or removed.
- [ ] A final five-party review has been completed.
- [ ] Owner has added required attachments and final decisions.

## 13. Collaborative Changelog

| Date | Contributor | Sections | Change | Decision changed? |
| --- | --- | --- | --- | --- |
| 2026-08-10 | Chat/Codex | Initial template | Created the collaborative report structure; no decisions made. | No |

## Appendix A — Open Questions and Conflicts

`[Add one row per unresolved conflict; do not silently resolve peer disagreements.]`

## Appendix B — Evidence Attachments

`[Owner to add logs, screenshots, traces, benchmark outputs, and external
reports as needed.]`

## Appendix C — Owner Scratch Pad

`[Optional]`

---

## Chat/Codex owner-Q&A review — 2026-08-10

This append-only contribution records the owner’s answers and should be
reviewed by Gemini, Claude, Grok, and finally the owner before roadmap changes
are considered final.

### Product contract

- Coding-Assistants is primarily a personal, local-first collaboration hub for
  the owner, external coding agents, and human collaborators.
- The self-contained multi-role LLM pipeline is an initial experiment, not the
  product identity.
- Workflows begin as explicitly wired roles and bounded asynchronous tasks;
  dynamic delegation and fully parallel work come later.
- Agents may continue while the owner is away. Wake-up approval, tool approval,
  sandbox strictness, and cost hard limits are configurable settings.
- Memory is hybrid: SQLite for durable global/workspace records and
  Git-tracked Markdown for important decisions, handoffs, and lessons.
- Android is a monitoring/approval client after desktop functionality is mostly
  complete. TUI and 3D visualizations are research/someday work.

### Recommended roadmap resolution

1. Add a top-priority Cross-Agent Shared Memory & Coordination track.
2. Keep ADR 0003/RD7 as the first runtime refactor; defer daemon and GraphQL.
3. Use a protocol-neutral Unix-socket/typed JSON-RPC experiment before choosing
   GraphQL or an actor framework.
4. Add provider/session adapters and local Ollama/llama.cpp support.
5. Keep Docker, Terraform, and Ansible; archive or remove unused deployment
   scaffolding while preserving its rationale.
6. Wire the currently unused direct-HTTP dependencies into a real provider
   roadmap item rather than dropping them.

### Suggested first milestone

SQLite schema and migrations; durable messages, handoffs, identities, and wake
requests; a CLI helper; Markdown summaries; per-task state; and one
cross-repository task where at least two agents retrieve each other’s durable
context successfully.

### Chat report and evidence

See `.agent/reports/chat/coding_assistants_architecture_review_20260810.md`.
Code-verified evidence includes the global task state in
`src-tauri/src/lib.rs`, fixed MCP configuration path in
`src-tauri/src/agents.rs`, unrestricted absolute file reading in
`src-tauri/src/lib.rs`, and zero Rust tests in the current test run.

### Changelog

| 2026-08-10 | Chat/Codex | Owner-Q&A synthesis | Recorded product identity, memory-first sequencing, and proposed roadmap changes; no owner decision silently changed. | No |

---

## Grok review pass — 2026-08-10

**Identity:** Grok (Build). Order: after Chat scaffold (and ideally Gemini/Claude reviews).

### Structure agreement

**Yes** — this admin report structure is adequate for the owner to fill after independent reports land. Suggested add-ons already partially covered by shared report:

1. Explicit **product identity DECIDED** block (now available from owner Q&A — copy from shared §3 or Grok report).
2. Pointer to **canonical shared report** `.agent/reports/shared/ca_20260810_shared_report.md`.
3. Pointer to **hub roadmap** `docs/moon/roadmaps/hub.md` as the binding near-term track.
4. Competing milestone plans (Alpha Memory Hub / Beta Harness / Gamma Daemon / Delta Security) with owner pick.

### Evidence for owner §0–1 (AGENT CLAIM summary)

- Backend ~1.35k LOC vs ambitious moon daemon/GraphQL/3D plans — ambition gap is OBSERVED.
- Owner Q&A locks hub identity, hybrid memory, async-first, ADR 0003, demote TUI/3D, keep LAN TCP now, dual license.
- Roadmaps updated in this session (moon index, hub.md, rust/ts/tui/kotlin, ROADMAP stub).

### Independent report

`.agent/reports/grok/ca_20260810_status_report.md`

### Did not

- Fill `[OWNER TODO]` decision cells as owner prose.
- Delete infra directories (only roadmap T6b mark-for-trim).

---

## Gemini (Antigravity) review pass — 2026-08-10

**Identity:** Gemini (Antigravity).

### Structure and Clarity Agreement

**Yes** — The owner's administrative synthesis is crystal clear and leaves no room for ambiguity. The product contract is tightly defined around the Local Collaboration Hub. The priorities (Hybrid Memory, Event Bus ADR 0003, Soft Budgets with Pause/Summarize) perfectly align with the Shared Report we produced.

### Questions/Doubts

None. The instruction to replace the language-specific roadmaps (`rust.md`, `typescript.md`, etc.) with feature-specific roadmaps (`memory.md`, `ui.md`, `communication.md`, etc.) in `docs/moon/roadmaps/` is an excellent architectural decision that decouples the product goals from the underlying tech stack.

### Ready for Roadmap Updates

I am fully synchronized and ready to begin executing the roadmap splits and updates.

### Changelog

| 2026-08-10 | Gemini | Owner-Q&A synthesis | Appended review pass; no doubts or contradictions found; confirmed readiness for roadmap updates. | No |

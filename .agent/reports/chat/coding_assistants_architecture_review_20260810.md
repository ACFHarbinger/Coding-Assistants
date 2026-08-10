# Coding-Assistants — Chat/Codex Architecture and Roadmap Review

**Date:** 2026-08-10  
**Contributor:** Chat / Codex  
**Status:** Independent report after owner Q&A; roadmap decisions remain subject
to the owner’s final review.

## Executive summary

Coding-Assistants is currently a promising Tauri/React desktop prototype with a
Rust backend that sequentially invokes configurable LLM roles, streams events,
pauses for human input, and exposes a basic Android TCP remote. Its intended
product is now clearer: a personal, local-first collaboration hub where the
human developer and external agents such as Claude, Codex, Gemini, and Grok
work together, with local models supported and cloud synchronization added
later. The self-contained multi-role chain is useful as an experiment, but it
is not the product center of gravity.

The next milestone should be durable shared memory and asynchronous
coordination. The system should record messages, decisions, task state,
provider events, approvals, and relevant Git changes in a local database, while
also producing human-readable Markdown for important decisions. Wake signals
should remain separate from durable state. This gives agents a reliable way to
leave work for one another before attempting a full parallel orchestration
engine.

The current roadmap is directionally good but over-commits to a future daemon,
GraphQL, actors, A2A, Ratatui, and 3D visualization before the core hub loop is
reliable. These should remain visible as future or research tracks, but the
active sequence should be memory → messaging → structured provider sessions →
configurable execution/approval → multi-client API → parallel orchestration.

## Product contract from owner Q&A

- Primary audience: the owner as a solo power developer, with possible use by
  trusted collaborators and eventual organizational deployment.
- Product identity: a collaboration hub for the human and external coding
  agents, not primarily a self-contained role pipeline.
- Execution: local-first, with future cloud synchronization between devices.
- Workflow: explicit wired roles and bounded tasks first; dynamic delegation and
  parallel work later.
- Persistence: both shared/global memory and per-repository memory; SQLite plus
  Git-tracked Markdown for important summaries and decisions.
- Agents may continue while the owner is away and may wake prior sessions,
  subject to configurable human-gate policies.
- Android: monitoring first, then approval/messaging; task configuration stays
  on desktop.
- Tool execution: allowed through OS APIs initially, with per-task approval and
  sandbox strictness settings.
- Cost controls: telemetry and soft warnings by default; optional hard limits.
- TUI and 3D graph: experimental/research-only for now.

## Strengths to preserve

1. The Tauri/Rust boundary is a reasonable starting point for local process and
   filesystem control.
2. Typed Serde payloads, Tauri events, and explicit process arguments provide a
   useful foundation for a stable protocol.
3. The existing `.agent/` resource convention is simple and inspectable.
4. Streaming, cancellation, user questions, and agent questions already prove
   the intended interaction shape.
5. ADR 0003 correctly recommends an event bus before a physical daemon split.
6. The Android app is appropriately treated as a remote client rather than a
   second orchestration implementation.
7. Roadmaps, ADRs, security documentation, and the multi-agent report process
   are valuable project memory and should remain.

## Implementation findings

### Critical

- `AppState` holds one global cancellation token and one input channel, so a
  second task can silently replace the first task’s control state.
- The fixed `~/.coding-assistants/mcp.json` path allows concurrent runs to race
  and potentially use the wrong configuration.
- `read_file_absolute` bypasses the documented workspace/resource boundary.
- `.agent` resource validation uses a string prefix rather than canonicalized
  path containment.
- The LAN TCP server has no authentication, encryption, or connection limits.
- `cargo test` currently runs zero tests.
- The frontend build could not run in the reviewed environment because npm
  dependencies were absent (`tsc: not found`).

### Important

- Marker parsing and magic approval strings should become typed messages with
  request IDs, sender identity, expiry, and audit records.
- Provider support is less complete than the roadmap implies: most providers
  route through OpenCode, LM Studio is partial, and direct HTTP dependencies
  are currently unused.
- Generated Markdown memory has no provenance, confidence, indexing, conflict
  handling, or correction workflow.
- Event broadcasts are not durable; disconnected clients can miss important
  work.
- `App.tsx` is already large enough that component extraction and state-machine
  separation will improve safety, even if a full state library is unnecessary.

## Recommended architecture

### Phase 1: durable hub inside the current application

Keep Tauri in-process. Add a core library/module independent of `AppHandle`
with:

- `TaskId`, `SessionId`, `AgentId`, `MessageId`, and `MemoryId`.
- SQLite migrations for tasks, sessions, messages, events, approvals, memory,
  provider calls, and wake requests.
- An internal broadcast event bus for live subscribers.
- Durable event writes before or alongside broadcasts.
- A CLI/helper executable for external agents to post, read, search, and poll.
- Markdown export for high-priority decisions and handoff summaries.

Alternatives:

1. SQLite plus a small Rust CLI — recommended for queryability and typing.
2. JSONL append-only log plus SQLite projection — strongest auditability, more
   implementation work.
3. Git-only Markdown — maximally transparent, but weak for concurrent queries
   and indexing.

### Phase 2: explicit workflows and permissions

Represent role wiring as a declarative workflow graph with sequential and
bounded parallel branches. Add standing policies such as “Claude may ask Grok
about Rust,” while retaining per-task overrides. Make human gates configurable
for wake-ups, tool execution, budget extensions, and workspace writes.

### Phase 3: provider/session adapters

Define a provider capability interface for CLI and local runners first:

- launch or attach to a session;
- stream typed events;
- send input;
- request cancellation;
- report usage and exit state;
- expose whether resume-by-conversation-ID is supported.

Keep OpenCode as a practical adapter initially, while adding native Ollama and
llama.cpp support. Add direct HTTP providers only where they provide a clear
benefit such as structured streaming, usage accounting, or independent
authentication. This is the appropriate place to wire the currently unused
`async-openai`, `reqwest`, and `dotenv` dependencies.

### Phase 4: multi-client boundary

When Android and external CLI clients need the same live hub, extract the core
into a daemon or library-backed service. Start with Unix domain sockets and a
typed JSON-RPC/event protocol. Consider WebSockets for remote devices and
GraphQL only if query complexity demonstrates a need. Actor frameworks can be
evaluated after concurrency measurements, not before.

## Roadmap decisions

### Keep active

- Cross-agent shared memory and coordination.
- Internal event bus.
- Cancellation safety and per-task state.
- Structured provider adapters and local model support.
- Human-in-the-loop approval and configurable trust policies.
- Runtime budgets, telemetry, soft warnings, and optional hard stops.
- SQLite/Markdown hybrid memory with global and workspace scopes.
- Test harness and security hardening.
- Desktop-first UI; Android monitoring/approval later.

### Change

- Replace the immediate GraphQL-first plan with a protocol-neutral API spike,
  likely Unix socket + JSON-RPC/events.
- Replace “actor framework adoption” with a state ownership and concurrency
  design milestone.
- Replace single-agent memory RP1–RP4 with shared multi-agent memory, keeping
  confidence/decay as later implementation details.
- Add a workflow graph before autonomous A2A delegation.
- Add acceptance criteria at least every few roadmap items and for each major
  track boundary.
- Keep `docs/ROADMAP.md` as a platform/product index with explicit links to
  the `docs/moon` implementation, research, and infrastructure tracks.

### Deprioritize, do not delete

- Ratatui/TUI.
- 3D force graphs.
- A2A interoperability.
- Affine compile-time budget delegation.
- Advanced MCP hosting and tool-poisoning automation.
- Kubernetes, Helm, serverless, Firebase, Azure Pipelines, WordPress,
  Webpack, Nginx, and proxy deployment scaffolding.

Keep Docker, Terraform, and Ansible. Preserve deprioritized material under an
archive or explicitly marked research/someday roadmap rather than silently
discarding it.

## Concrete competing milestone plans

### Plan A — Memory-first spine (recommended)

1. SQLite schema, migrations, and CLI helper.
2. Durable messages, handoffs, identities, and wake signals.
3. Event bus and per-task state.
4. Markdown summaries and memory review UI.
5. Basic external-agent adapters and a successful cross-repository task.

### Plan B — Provider-first spine

1. Provider trait and session adapters.
2. Structured streams, resume capability, and usage accounting.
3. SQLite event persistence.
4. Message/wake protocol.
5. Workflow wiring.

This may produce faster visible integrations but risks building adapters around
an unstable collaboration model.

### Plan C — Daemon-first spine

1. Extract a headless Rust core.
2. Unix-socket API.
3. Tauri and Android clients.
4. Durable memory and workflow engine.

This best supports eventual multi-client use but has the highest risk of
premature boundary design. I do not recommend it as the next milestone.

## Final recommendation

Choose Plan A. Define the first success benchmark as: the owner and at least two
external agents complete a meaningful task in another repository, where one
agent can retrieve a previous agent’s durable handoff without manual context
reconstruction, and the owner can review the transcript, memory, approvals,
and Git changes afterward.

Do not rewrite the project in C++ or replace Tauri yet. The current limitation
is product state, durability, and protocol design rather than measured CPU
performance.


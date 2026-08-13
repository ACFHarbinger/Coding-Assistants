# Agent Communication and Delegation Roadmap

Communication starts with explicit, declarative task wiring and asynchronous
mailboxes. Parallel execution and A2A follow only after durable local
communication is reliable.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| C1 | Agent identities, attribution headers, durable inbox/outbox messages, and handoff records | Every message records sender, receiver, task, workspace, timestamp, and status | ✅ **Done** · `ca msg send/poll/list/status` + seeded agents + handoff kind in MD export |
| C2 | Shared `ca` CLI for read/write/search/poll operations | External agent loops can use it without the desktop UI | ✅ **Done** · binary `ca`; `ca agent team\|enroll\|unenroll`; also mirrored by Tauri `hub_*` commands / HubPanel |
| C3 | Separate ephemeral wake mechanism via file watch or local socket | Durable writes survive absent agents; wake requests are observable and **deduplicated** | ✅ **Done** · `wake/*.json` + SQLite; pending dedup by target/message/reason; resolve delivered/cancelled |
| C4 | Configurable human gates and standing policies for wake-ups and delegation | Per-task policy can allow or require approval | ✅ **Done** · persisted `WakePolicy` integrated into desktop Shared Hub Policy tab; per-task delegation policy via `require_human_approval` on `TaskRecord` |
| C5 | Declarative sequential and bounded-parallel workflow wiring | A real task can be split into plan/code/review boundaries with retries and handoffs | ✅ **Done** · stages via `parallel_group`; `max_parallel` queue; `retry_task`/`max_retries`; `complete_parallel_member`; CLI `task complete|retry` + Tauri (2026-08-11) |
| C6 | Budget exhaustion pause, Markdown handoff summary, delegation, and shutdown | No uncontrolled provider calls continue after a configured limit | ✅ **Done** · Tauri `AgentSystem`, CLI `budget consume`, Tauri commands, and Shared Hub Usage tab enforce configured limits; shutdown hooks exposed via `ca shutdown` and `hub_record_shutdown` |
| C7 | **Next major milestone:** A2A-compatible discovery, Agent Cards, and horizontal delegation | Local workflows interoperate with an A2A peer while preserving identity, approval, budget, and audit policy | ✅ **Done** · `AgentCard` schema and storage in `ca-hub`, `ca agent register-card` in CLI, `hub_upsert_agent_card` in Tauri, and `GetAgentCards` over `TcpServer` |
| C8 | Fully parallel execution from session start | Concurrent work has conflict detection, task isolation, and deterministic recovery | 📋 Pending · later |
| C9 | Agent inbox bridge process | A long-lived adapter can consume one agent's hub messages as a stable stream, acknowledge them, and honor wake gates | 🚧 **In Progress** · `ca inbox watch --agent <id>` emits JSONL, resolves accepted wakes, forwards to an adapter stdin, and includes a Codex app-server thread adapter with persisted-thread discovery and hub reply routing; direct attachment to an existing interactive TUI remains open. C12 completes this for all four harnesses. |
| C10 | Session addressing: all, subset, or one | Human and any enrolled agent can send a session message to every member, a named subset, or a single member. Non-targets are not woken or tasked. The session transcript records the explicit `to` list. | 🚧 **In Review** · session sends persist an exact recipient set by subject and reject non-members server-side; Chat & Memory routes all/subset/one through typed session/tagged commands. Agent-harness posting parity remains C12. |
| C11 | Task vs wake message tags | A message may be tagged **task**, **wake**, both, or neither. **Wake** may launch a new harness instance of that identity and enroll it in the session team. **Task** must target an already-enrolled, currently present member and is refused (no spawn) otherwise. Agents can apply the same tags through the hub API/CLI. | 🚧 **In Review** · `HubStore::send_tagged_message` + `hub_send_tagged_message` + `ca msg tag` enforce task-refuse and wake-enroll, with per-recipient durable outcomes and wake-policy-aware delivery. Chat & Memory invokes the C12 injector; agents can opt into the same explicit dispatch with `ca msg tag --dispatch --workspace <absolute-path>`. "Currently present" = team + (session, if given) membership. |
| C12 | Bidirectional harness capture and inject | The app captures messages agents send inside Grok/Chat/Claude/Gemini harnesses into the session transcript. Hub messages tagged task and/or wake are injected into the target harness so the agent executes them. Builds on C9. | 🚧 **In Review** · UI polls all four captures; tagged send injects via `hub_inject_harness`; partial IPC failures are retained and surfaced per target instead of masking the durable post. Fixture suite + live disk capture on this checkout found transcripts for Grok/Claude/Chat/Gemini (bodies not logged). `ca harness capture --harness <id> --workspace PATH` reimplements the same four transcript formats headlessly for scripted/CLI-only C13 runs, converging on the same `record_harness_capture` dedup path as the desktop poll. C13 is the owner running that loop in Chat & Memory (or headlessly via the CLI). No TUI attach. |
| C13 | Hub replaces the per-repo markdown bus | A full assign/review/task/wake loop completes with no writes to `.agent/cache/AGENT_BUS.md` or `.agent/messages/*`. Those files stay as a fallback until C10–C12 ship. `.agent` prompts/rules/skills remain resources, not the live protocol. | 📋 **Planned** · execute the explicit migration gate below only after C12 passes live acceptance; preserve the bus as a read-only fallback until then. |

### C13 migration gate

1. **Preflight:** C10–C12 have passed their live acceptance checks; create or
   load a named work session with a recorded workspace and enrolled team.
2. **Hub-native run:** the owner assigns a bounded repository task through
   Chat & Memory to all, a subset, and one agent. At least two agents must
   acknowledge, execute/review, and publish their harness-originated result
   into the same session transcript; include one audited task or wake delivery.
3. **Reconstruction:** the session transcript, recipient/outcome records, and
   audit trail independently show assignment, delivery, execution, review,
   final decision, and handoff. No `.agent/cache/AGENT_BUS.md` or
   `.agent/messages/*` write is permitted during the run.
4. **Recovery:** if delivery, capture, or review fails, record the failure in
   the Hub and resume only through the existing Markdown bus. Do not delete,
   rewrite, or silently import historical bus/message files.
5. **Completion:** attach the acceptance evidence to #113, update the
   changelog/roadmaps and Project 21, then demote the Markdown bus to
   documented read-only fallback rather than removing it.

**2026-08-12:** CA-102 adds bounded, exact channel queries to the shared
store, CLI, and Tauri API (`channel:<name>` plus colon-delimited metadata).
Chat messages can embed `[Memory #<full-id-or-unique-prefix>]`; the Hub
resolves only unique references, retaining isolation and avoiding accidental
links to similarly prefixed memories. CA-106/109 add owner-only edit/delete
parity across the desktop and CLI. CA-114 adds contextual replies using the
same subject namespace (`channel:<name>:thread:<root>:<id>`), preserving
channel isolation and existing roster wake behavior without a migration.

**2026-08-13:** Named work sessions are durable `work_sessions` plus
membership records. A session initializes from the persisted team; an agent
added to the team is also enrolled in the active session. Its chat uses an
isolated `channel:session:<id>` subject namespace, so messages emitted from a
human or agent harness render together while per-member wake selection stays
an explicit delivery decision.

The `.agent/reports`, `.agent/messages`, and `.agent/cache/AGENT_BUS.md`
conventions are temporary process artifacts, not the long-term communication
protocol. Until C10–C13 ship, Grok and Chat still coordinate sub-task
allocation on `AGENT_BUS.md`.

**2026-08-13 (Grok, v1 hub-native orchestration):** Harbinger's remaining
workload is to run the team from the CA app instead of per-repo markdown.
C10–C13 plus U11–U12 are that delivery. Order: U11 load/create, then C10+U12
addressing and tags, then C11 spawn-vs-existing semantics, then C12
four-harness capture/inject, then C13 retire the markdown bus.

**2026-08-11:** The desktop Shared Hub originally exposed Inbox/Wakes panels
over the same store as the CLI. Those duplicate surfaces are now retired:
messages and memory belong to **Chat & Memory**, while wake events belong in
`#wakes-alerts`. Persisted `WakePolicy` remains in the Shared Hub Policy tab;
per-task policy is available through CLI/Tauri task creation, while desktop
task-creation controls remain open. A2A
Agent Card discovery and delegation payloads are implemented via `AgentCard` in `ca-hub`, exposed via `ca agent register-card` and `GetAgentCards` in the `TcpServer`.

The C5 task schema and dispatch path now persist retry counters,
parallel-stage queues, and a maximum concurrency bound.

The Tauri execution path now performs call-count accounting around `LLMClient`
completions and invokes the existing handoff flow on exhaustion. Cancellation
now also writes a durable shutdown handoff before the active run exits.
Provider automatic spend reporting, external-adapter adoption, and shutdown
hooks remain open. External agents can now reserve units atomically with
`ca budget consume` immediately before a provider request.

**2026-08-11:** C6 first boundary implemented — per-agent budgets are caller-defined units
(call count, USD, tokens, ...); the store only compares totals, so the
provider-cost mapping is a caller concern. `pause_for_budget` is explicit
(distinct from the automatic `paused` flip in `record_budget_usage`) so a
caller can keep working briefly after crossing the limit if it chooses, but
is expected to call it before stopping, per the owner's original answer (a
persistent summary + delegation + shutdown, not a hard kill). Provider
automatic spend reporting and shutdown hooks remain open, as does desktop
The remaining workflow gap is fully parallel session startup under C8.

**2026-08-12:** Team fan-out now uses an explicit persisted roster
(`agents.team_member`) instead of every row in `agents`. Default members:
`human`, `claude`, `chat`, `gemini`, `grok`. Harbinger is included so
`#general` is visible to the owner. Chat & Memory/Orchestrate team sends wake that
roster with `hub_request_wake` per enrolled member (`HubStore::request_team_wakes`,
`hub_request_team_wakes`). Enrollment: `ca agent enroll\|unenroll\|team` and
`hub_set_team_member`. Chat's CA-102 channel-query work owns
`list_channel_messages` in the same store.

**2026-08-13:** Chat & Memory DMs no longer inherit the team-broadcast recipient
(`2ab31c7`). Composer is Enter-to-send with a jump-to-latest chip while
reading history (`947a43d`). Thread replies (CA-114, Chat) stay in the
`channel:<name>:thread:` subject namespace.

**2026-08-13 (cloud):** Multi-device replica of `.coding-assistants` is specified
in [`cloud_sync.md`](cloud_sync.md) (S1–S13, issues #91–#103). Drive first;
journal-integrity merge is S6 after the S5 snapshot gate. Not implemented.

# Agent Communication and Delegation Roadmap

Communication starts with explicit, declarative task wiring and asynchronous
mailboxes. Parallel execution and A2A follow only after durable local
communication is reliable.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| C1 | Agent identities, attribution headers, durable inbox/outbox messages, and handoff records | Every message records sender, receiver, task, workspace, timestamp, and status | ✅ **Done** · `ca msg send/poll/list/status` + seeded agents + handoff kind in MD export |
| C2 | Shared `ca` CLI for read/write/search/poll operations | External agent loops can use it without the desktop UI | ✅ **Done** · binary `ca`; also mirrored by Tauri `hub_*` commands / HubPanel |
| C3 | Separate ephemeral wake mechanism via file watch or local socket | Durable writes survive absent agents; wake requests are observable and **deduplicated** | ✅ **Done** · `wake/*.json` + SQLite; pending dedup by target/message/reason; resolve delivered/cancelled |
| C4 | Configurable human gates and standing policies for wake-ups and delegation | Per-task policy can allow or require approval | ✅ **Done** · persisted `WakePolicy` integrated into desktop Shared Hub Policy tab; per-task delegation policy via `require_human_approval` on `TaskRecord` |
| C5 | Declarative sequential and bounded-parallel workflow wiring | A real task can be split into plan/code/review boundaries with retries and handoffs | ✅ **Done** · stages via `parallel_group`; `max_parallel` queue; `retry_task`/`max_retries`; `complete_parallel_member`; CLI `task complete|retry` + Tauri (2026-08-11) |
| C6 | Budget exhaustion pause, Markdown handoff summary, delegation, and shutdown | No uncontrolled provider calls continue after a configured limit | 🚧 **Partial** · Tauri `AgentSystem`, CLI `budget consume`, Tauri commands, and Shared Hub Budget tab enforce configured limits; external adapters must adopt the guard and shutdown hooks remain open |
| C7 | **Next major milestone:** A2A-compatible discovery, Agent Cards, and horizontal delegation | Local workflows interoperate with an A2A peer while preserving identity, approval, budget, and audit policy | 📋 Pending · next major milestone |
| C8 | Fully parallel execution from session start | Concurrent work has conflict detection, task isolation, and deterministic recovery | 📋 Pending · later |

The `.agent/reports` and `.agent/messages` conventions are temporary process
artifacts, not the long-term communication protocol.

**2026-08-11:** Desktop Shared Hub Inbox/Wakes panels use the same store as CLI.
Wake resolution and persisted `WakePolicy` are fully integrated into the Shared
Hub UI (Wakes and Policy tabs); per-task policy is available through CLI/Tauri
task creation, while desktop task-creation controls remain open. A2A
remains owner-hedged strategically (see prior open-question note); not
implemented yet.

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
Budget-tab wiring. The remaining workflow gaps also include C7 (A2A) and fully
parallel session startup under C8.

# Agent Communication and Delegation Roadmap

Communication starts with explicit, declarative task wiring and asynchronous
mailboxes. Parallel execution and A2A follow only after durable local
communication is reliable.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| C1 | Agent identities, attribution headers, durable inbox/outbox messages, and handoff records | Every message records sender, receiver, task, workspace, timestamp, and status | ✅ **Done** · `ca msg send/poll/list/status` + seeded agents + handoff kind in MD export |
| C2 | Shared `ca` CLI for read/write/search/poll operations | External agent loops can use it without the desktop UI | ✅ **Done** · binary `ca`; also mirrored by Tauri `hub_*` commands / HubPanel |
| C3 | Separate ephemeral wake mechanism via file watch or local socket | Durable writes survive absent agents; wake requests are observable and **deduplicated** | ✅ **Done** · `wake/*.json` + SQLite; pending dedup by target/message/reason; resolve delivered/cancelled |
| C4 | Configurable human gates and standing policies for wake-ups and delegation | Per-task policy can allow or require approval | 🚧 **Partial** · standing `WakePolicy` in meta (default human-gate, allow_auto_wake); per-task policy still open |
| C5 | Declarative sequential and bounded-parallel workflow wiring | A real task can be split into plan/code/review boundaries with retries and handoffs | 🚧 **Partial** · sequential `tasks` + `ca task create/advance/list` (handoff+wake per step); bounded-parallel + retries still open |
| C6 | Budget exhaustion pause, Markdown handoff summary, delegation, and shutdown | No uncontrolled provider calls continue after a configured limit | 📋 Pending |
| C7 | **Next major milestone:** A2A-compatible discovery, Agent Cards, and horizontal delegation | Local workflows interoperate with an A2A peer while preserving identity, approval, budget, and audit policy | 📋 Pending · next major milestone |
| C8 | Fully parallel execution from session start | Concurrent work has conflict detection, task isolation, and deterministic recovery | 📋 Pending · later |

The `.agent/reports` and `.agent/messages` conventions are temporary process
artifacts, not the long-term communication protocol.

**2026-08-11:** Desktop Shared Hub Inbox/Wakes panels use the same store as CLI.
Wake resolution and persisted `WakePolicy` are available through CLI/Tauri;
desktop policy controls and per-task delegation policy remain open. A2A
remains owner-hedged strategically (see prior open-question note); not
implemented yet.

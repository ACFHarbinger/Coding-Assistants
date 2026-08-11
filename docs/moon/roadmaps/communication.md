# Agent Communication and Delegation Roadmap

Communication starts with explicit, declarative task wiring and asynchronous
mailboxes. Parallel execution and A2A follow only after durable local
communication is reliable.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| C1 | Agent identities, attribution headers, durable inbox/outbox messages, and handoff records | Every message records sender, receiver, task, workspace, timestamp, and status | 🚧 In Progress · `ca msg` + agent seed list + desktop Hub inbox |
| C2 | Shared `ca` CLI for read/write/search/poll operations | External agent loops can use it without the desktop UI | 🚧 In Progress · binary `ca` in `crates/ca-cli` and Tauri Hub commands |
| C3 | Separate ephemeral wake mechanism via file watch or local socket | Durable writes survive absent agents; wake requests are observable and deduplicated | ✅ Core complete · `wake/*.json` side-channel + SQLite wake_requests with pending-request deduplication; watcher/policy work remains |
| C4 | Configurable human gates and standing policies for wake-ups and delegation | Per-task policy can allow or require approval | 📋 Pending |
| C5 | Declarative sequential and bounded-parallel workflow wiring | A real task can be split into plan/code/review boundaries with retries and handoffs | 📋 Pending |
| C6 | Budget exhaustion pause, Markdown handoff summary, delegation, and shutdown | No uncontrolled provider calls continue after a configured limit | 📋 Pending |
| C7 | **Next major milestone:** A2A-compatible discovery, Agent Cards, and horizontal delegation | Local workflows interoperate with an A2A peer while preserving identity, approval, budget, and audit policy | 📋 Pending · next major milestone |
| C8 | Fully parallel execution from session start | Concurrent work has conflict detection, task isolation, and deterministic recovery | 📋 Pending · later |

The `.agent/reports` and `.agent/messages` conventions are temporary process
artifacts, not the long-term communication protocol.

**Implementation note (2026-08-11):** the desktop Shared Hub exposes durable
inbox polling, wake requests, and the same `ca-hub` data directory used by the
CLI. Repeated pending wake requests with the same target, message, and reason
reuse the existing durable request rather than creating duplicate signals.

**Verification note (Claude, 2026-08-10):** C1–C3 status lines confirmed
accurate against the actual code after reconciling a duplicate-implementation
collision in `crates/ca-hub` (see `memory.md`'s implementation note) — `ca msg
send/poll/list`, the binary `ca`, and the `wake/*.json` + `wake_requests`
side-channel all build, pass `cargo test -p ca-hub`, and were smoke-tested
end-to-end. Not yet true: C3's "deduplicated" claim in the exit criteria —
`list_wakes`/`request_wake` have no dedup logic yet, a repeated wake request
just inserts another row/file.

**Open question, not resolved by this edit (Claude, 2026-08-10):** C7 labels
A2A the "next major milestone," ranked above the daemon/UI tracks in
`docs/moon/ROADMAP.md`'s priority order. The only owner quote on record for
A2A (Chat's Q&A round) is "strategically interesting, although I am still
unsure of what the results of such functionality will be" — a hedge, not a
milestone commitment. Two other agents (Chat, Grok) independently treated it
as settled in their shared-report contributions. Leaving the roadmap wording
as-is since it's already committed, but flagging in the shared report (§7)
for explicit owner confirmation rather than silently endorsing it.

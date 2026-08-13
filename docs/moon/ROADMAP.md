# Coding-Assistants Roadmap

> **Version:** 2.0
> **Date:** 2026-08-10
> **Product:** local-first collaboration hub for a human developer and
> external AI agents.

This is the canonical implementation-order index. Detailed work is organized
by capability rather than programming language. Research remains under
[`research/`](research/) and [`reports/`](reports/).

## Priority order

1. Memory and private journals
2. Communication and asynchronous coordination
3. Platform reliability, providers, tools, and security
4. Desktop UI, Ratatui TUI, and persistent settings
5. A2A interoperability (next major milestone)
6. Daemon/multi-client extraction when justified by a second client
7. Android monitoring and approval
8. Someday/research: 3D visualization, GraphQL, and actor frameworks

```mermaid
gantt
    title Coding-Assistants capability roadmap
    dateFormat  YYYY-MM-DD
    axisFormat  %b %d
    section Foundation
    SQLite memory and private journals       :memory, 2026-08-10, 21d
    Async mailboxes and wake signals         :coord, after memory, 21d
    Event bus and per-task state              :platform, after memory, 21d
    Provider/tool adapters                    :providers, after coord, 28d
    section Product
    Desktop memory and inbox UI               :ui, after coord, 28d
    Ratatui terminal client                   :tui, after coord, 42d
    Persistent settings                       :settings, after platform, 28d
    2D telemetry and collaboration dashboard  :dash, after ui, 21d
    A2A interoperability milestone            :a2a, after providers, 42d
    section Clients
    Local daemon / UDS evaluation             :daemon, after a2a, 28d
    Android monitoring and approvals          :android, after dash, 28d
    section Later
    3D research                               :later, after daemon, 42d
```

The dates are sequencing placeholders, not delivery promises. Each capability
roadmap must add acceptance criteria at least every few entries.

## Capability roadmaps

| Capability | Roadmap | Priority |
| --- | --- | --- |
| Memory and private journals | [`roadmaps/memory.md`](roadmaps/memory.md) | P0 |
| Communication and delegation | [`roadmaps/communication.md`](roadmaps/communication.md) | P0 |
| Platform/providers/tools/security | [`roadmaps/platform.md`](roadmaps/platform.md) | P1 |
| Persistent settings and local configuration | [`roadmaps/settings.md`](roadmaps/settings.md) | P1 · approved implementation plan |
| Desktop/mobile/TUI UI | [`roadmaps/ui.md`](roadmaps/ui.md) | P1 · approved TUI implementation plan |
| Telemetry and dashboards | [`roadmaps/dashboard.md`](roadmaps/dashboard.md) | P1 |
| Infrastructure and documentation | [`roadmaps/infrastructure.md`](roadmaps/infrastructure.md) | P1 |
| Cloud Drive sync | [`roadmaps/cloud_sync.md`](roadmaps/cloud_sync.md) | P1 · approved implementation plan |

## Product gates

- **Memory gate:** two agents retrieve and correctly use a prior handoff on a
  real repository task; shared and private records remain separated.
- **Coordination gate:** asynchronous wired workflows survive agent absence,
  wake according to policy, and preserve an auditable transcript.
- **A2A gate:** a compatible external agent can be discovered and delegated a
  bounded task without bypassing identity, approval, budget, or audit policy.
- **V1 gate:** the owner and multiple agents complete a meaningful task on a
  repository such as Project-Mobile-Fortress with quality matching or exceeding
  a single contributor.
- **V1 hub-native orchestration gate:** Harbinger creates or loads a team
  chat from Orchestrate, addresses all / a subset / one agent, and marks
  posts as **task** (existing member only) and/or **wake** (may spawn a new
  instance that joins the team). Agents can do the same. The app captures
  harness-side messages and injects tagged hub messages into the harnesses.
  That loop does not require `.agent/cache/AGENT_BUS.md`. Slices: U11, U12,
  C10, C11, C12, C13.

LAN TCP remains available during early development. Authentication and TLS are
  later platform work. **`.coding-assistants` multi-device replica transport
  starts with Google Drive**, then Firebase Auth + private Storage, Supabase
  Auth + private Storage, and finally Dropbox/OneDrive — see
  [`roadmaps/cloud_sync.md`](roadmaps/cloud_sync.md). Other unused deployment
  scaffolding is removed.

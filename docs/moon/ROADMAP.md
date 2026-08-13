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
4. Desktop UI and 2D dashboard
5. A2A interoperability (next major milestone)
6. Daemon/multi-client extraction when justified by a second client
7. Android monitoring and approval
8. Someday/research: TUI, 3D visualization, GraphQL, actor frameworks

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
    2D telemetry and collaboration dashboard  :dash, after ui, 21d
    A2A interoperability milestone            :a2a, after providers, 42d
    section Clients
    Local daemon / UDS evaluation             :daemon, after a2a, 28d
    Android monitoring and approvals          :android, after dash, 28d
    section Later
    TUI and 3D research                       :later, after daemon, 42d
```

The dates are sequencing placeholders, not delivery promises. Each capability
roadmap must add acceptance criteria at least every few entries.

## Capability roadmaps

| Capability | Roadmap | Priority |
| --- | --- | --- |
| Memory and private journals | [`roadmaps/memory.md`](roadmaps/memory.md) | P0 |
| Communication and delegation | [`roadmaps/communication.md`](roadmaps/communication.md) | P0 |
| Platform/providers/tools/security | [`roadmaps/platform.md`](roadmaps/platform.md) | P1 |
| Desktop/mobile/TUI UI | [`roadmaps/ui.md`](roadmaps/ui.md) | P1/P3 |
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

## 2026-08-13 status (Grok lead pass)

- **Memory gate:** live seed `M6-20260812` + Claude ACK. Board closed #82 /
  #80. Residual: Chat never posted a second `M6-ACK`; treat as board-closed
  with that caveat in `roadmaps/memory.md`.
- **Coordination / Slack loop:** team roster, team-wide wakes, enroll CLI,
  private DMs, scroll-pin, Enter-to-send. Claude closed the U10 follow-up
  epic as #90. #81 (wake policy leftovers) remains open.
- **Cloud sync:** approved plan in [`roadmaps/cloud_sync.md`](roadmaps/cloud_sync.md);
  GitHub S1–S13 are #91–#103. Not implemented yet.
- **U8 Usage quotas:** live remaining bars for Codex, Claude, and Grok
  (Grok weekly pool after `grok login`, `f9e255b`). Gemini/Antigravity
  plots exist; a real Antigravity adapter is still disclosed as open.
- **V1 hub-native orchestration:** U11–U12 and C10–C12 are implemented and
  ready for Harbinger to test in the desktop app (create/load session,
  all/subset/one, task/wake tags, four-harness capture poll + inject).
  C13 is that owner live loop; the markdown bus stays as fallback until then.

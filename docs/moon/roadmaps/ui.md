# User Interface Roadmap

The desktop application is the primary interface. Android follows desktop
stabilization and focuses on monitoring, approvals, and messages. TUI remains
an experiment.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| U1 | Split `App.tsx` into configuration, activity, memory, inbox, approval, and remote panels | Components have focused state boundaries and frontend tests | ✅ **Done** · Refactored App.tsx into ConfigPanel, ActivityPanel, RemotePanel, ApprovalPanel, and HubPanel with aesthetic redesign |
| U2 | Task history, transcript, and handoff browser | User can resume/review a prior task without reconstructing context manually | ✅ **Done** · Added Task Browser tab to Shared Hub rendering task metadata and message history
| U3 | Memory review UI with global/workspace/private scope indicators | User can search, edit, delete, and mark memories stale | ✅ **Done** · Hub memory tab includes search, inline editing, delete, promote, compact, export, and color-coded scope indicators |
| U4 | Configurable policy controls for tool execution, sandbox strictness, wake gates, and budgets | Settings are persisted per task/workspace and reflected in audit events | 🚧 **Partial** · Wake policy integrated into Shared Hub Policy tab; tool sandbox UI still open |
| U5 | Android monitoring and approval client | Mobile can watch events and send approved messages without configuring full tasks | ✅ **Done** · Added DashboardScreen to Android app for viewing events and approving/rejecting wakes via TCP |
| U6 | Project creation wizard | Simple flow to bootstrap `.agent/` directories in new workspaces | ✅ **Done** · Added `bootstrap_workspace` command and UI button to initialize `.agent/` skeleton in workspaces |
| U8 | Agent telemetry dashboard | Shared Hub visualizes per-agent budget, output, token, and call counters | ✅ **Done** · Usage plots quota windows and reset times across the configured Codex, Claude, Grok, Gemini/Antigravity, and other harness families; local budgets remain available separately. Only Codex and Grok re-query live on every call and keep the "live quota" badge — Claude, Gemini/Antigravity, and other non-live providers show a last-refreshed timestamp plus per-provider and refresh-all-stale buttons. |
| U9 | Existing model process connection | Orchestrate roles can attach to a running model service instead of always starting a child process | 🚧 **Partial** · Endpoint configuration and process discovery/add-to-team controls are available; connection health and streaming controls remain |
| U10 | Team chat and agentic memory | Chat & Memory is the sole human/agent conversation surface; Orchestrate is role/team setup, work-session creation, and Remote Control | 🚧 **Partial** · Private DMs, scroll-pin/jump-to-latest, Enter-to-send, persisted roster/team-wide wakes, enrollment controls, Edit/Delete, in-context replies, and named work-session chats with per-member wake selection are available. Shared Hub no longer duplicates Inbox, Memory, or Wakes; wake alerts live in `#wakes-alerts`. Remaining v1 surface work is U11–U12. |
| U11 | Orchestrate create and load team chat | Orchestrate has two buttons: **Create team chat** (named durable session from the current team) and **Load team chat** (picker of existing sessions). Either action focuses Chat & Memory on that session channel. | ✅ **Done** · Create/Load in Orchestrate (`46b1ba4`). Grok: chosen session is persisted and Chat & Memory opens `session:<id>` even when reloading the same session. Board: #108. |
| U12 | Session composer: all / subset / one, plus task and wake tags | The Chat & Memory composer can address every session member, a checked subset, or one member, and can mark the post **task**, **wake**, both, or neither. Agents posting through the hub get the same controls in the transcript. | 🚧 **In Review** · session controls select all/subset/one of the actual session members and invoke typed C10/C11 sends; recipient and intent badges render in the transcript. Agent-harness posting parity remains C12. |
| U13 | Create and delete Chat & Memory channels | Owner can add a named durable channel and remove a custom one from the sidebar. Built-in `#general`, `#team-coordination`, `#agent-memory`, and `#wakes-alerts` cannot be deleted. | ✅ **Done** · `chat_channels` in `HubStore`; `hub_list_channels` / `hub_create_channel` / `hub_delete_channel`; Chat & Memory sidebar + / × controls. Board: #114. |
| U7 | TUI/Ratatui experiment | Built only after the shared client protocol is stable | 💤 Someday/Maybe |

**2026-08-12:** Delivered U10 (Team Chat & Agentic Memory Hub) in `SlackChatPanel.tsx`. Includes channel sidebar, agent presence indicators, message stream, target recipient routing, wake policy controls, and inline memory drawer.

**2026-08-11:** Completed the U1 objective. Extracted `App.tsx` logic into `ConfigPanel`, `ActivityPanel`, `RemotePanel`, and `ApprovalPanel` along with a major glassmorphism redesign for premium aesthetics.

**2026-08-13:** Chat & Memory is the only conversation surface. Header badge is
**Local hub online** (not a second chat tab). DMs cannot team-broadcast.
Scroll stays put while reading. Enter sends. Journal tab (CA-111, Claude)
covers the audit-on-open checkpoint.

**2026-08-13 (Grok, U8):** Usage plots Grok's weekly subscription pool next to
Chat/Codex and Claude. The adapter uses the signed-in Grok CLI session and
the TUI `/usage` billing snapshot (`creditUsagePercent`, `billingPeriodEnd`).
Gemini and Antigravity support now land alongside the other configured
harnesses. Board: #86 closed.

**2026-08-13 (Shared Hub):** Retired the duplicate Inbox, Memory, and Wakes
tabs in favor of Chat & Memory and its `#wakes-alerts` channel. Policy
checkboxes persist optimistically and use an explicit high-contrast checked
state.

**2026-08-13 (Claude, U8):** Replaced the blanket "live quota" badge with a
per-provider distinction: Codex and Grok fetch live on every call and keep
the badge; Claude, Gemini/Antigravity, and other non-real-time providers
show a "last refreshed" timestamp, a per-provider Refresh button, and a
Refresh all stale quotas button (`hub_refresh_provider_quota` command).
Also disclosed that `gemini_quota()` currently returns hardcoded window
data — a real Antigravity CLI adapter is still open, tracked under #86.

**2026-08-13 (Grok, v1 hub-native orchestration):** U11–U12 are the desktop
half of moving Harbinger's orchestration off `.agent` markdown. U11 Create/Load
plus session focus/persist is done. Remaining: durable C10 recipient lists,
C11 spawn-on-wake, C12 harness capture/inject, C13 retire the markdown bus.

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
| U8 | Agent telemetry dashboard | Shared Hub visualizes per-agent budget, output, token, and call counters | 🚧 **Partial** · Dashboard tab and persisted local counters are available; Usage now plots provider-exact Codex quota windows with reset times, while Claude/Gemini/Grok quota adapters and historical charts remain |
| U9 | Existing model process connection | Orchestrate roles can attach to a running model service instead of always starting a child process | 🚧 **Partial** · Endpoint configuration and process discovery/add-to-team controls are available; connection health and streaming controls remain |
| U10 | Slack team chat | Slack Chat & Memory is the sole human/agent conversation surface; Orchestrate is role/team setup, work-session creation, and Remote Control | 🚧 **Partial** · Panel in `c9932ac`; private DMs, scroll-pin/jump-to-latest, Enter-to-send, persisted roster/team-wide wakes, enrollment controls, Edit/Delete, in-context replies, and named work-session chats with per-member wake selection are available. Live presence polish and dedicated thread views remain. |
| U7 | TUI/Ratatui experiment | Built only after the shared client protocol is stable | 💤 Someday/Maybe |

**2026-08-12:** Delivered U10 (Slack Team Chat & Agentic Memory Hub) in `SlackChatPanel.tsx`. Includes channel sidebar, agent presence indicators, Slack message stream, target recipient routing, wake policy controls, and inline memory drawer.

**2026-08-11:** Completed the U1 objective. Extracted `App.tsx` logic into `ConfigPanel`, `ActivityPanel`, `RemotePanel`, and `ApprovalPanel` along with a major glassmorphism redesign for premium aesthetics.

**2026-08-13 (Grok):** Slack is the only conversation surface. Header badge is
**Local hub online** (not a second Slack tab). DMs cannot team-broadcast.
Scroll stays put while reading. Enter sends. Journal tab (CA-111, Claude)
covers the audit-on-open checkpoint.

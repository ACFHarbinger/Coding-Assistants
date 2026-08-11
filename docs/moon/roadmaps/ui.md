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
| U8 | Agent telemetry dashboard | Shared Hub visualizes per-agent budget, output, token, and call counters | 🚧 **Partial** · Dashboard tab and persisted local counters are available; provider-exact telemetry and historical charts remain |
| U7 | TUI/Ratatui experiment | Built only after the shared client protocol is stable | 💤 Someday/Maybe |

**2026-08-11:** Completed the U1 objective. Extracted `App.tsx` logic into `ConfigPanel`, `ActivityPanel`, `RemotePanel`, and `ApprovalPanel` along with a major glassmorphism redesign for premium aesthetics.

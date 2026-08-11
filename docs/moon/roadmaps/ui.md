# User Interface Roadmap

The desktop application is the primary interface. Android follows desktop
stabilization and focuses on monitoring, approvals, and messages. TUI remains
an experiment.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| U1 | Split `App.tsx` into configuration, activity, memory, inbox, approval, and remote panels | Components have focused state boundaries and frontend tests | ✅ **Done** · Refactored App.tsx into ConfigPanel, ActivityPanel, RemotePanel, ApprovalPanel, and HubPanel with aesthetic redesign |
| U2 | Task history, transcript, and handoff browser | User can resume/review a prior task without reconstructing context manually | 📋 Pending · handoffs in inbox + MD export |
| U3 | Memory review UI with global/workspace/private scope indicators | User can search, edit, delete, and mark memories stale | 🚧 **Partial** · Hub memory tab (search/stale/delete/promote/compact/export); journals append via CLI/Tauri |
| U4 | Configurable policy controls for tool execution, sandbox strictness, wake gates, and budgets | Settings are persisted per task/workspace and reflected in audit events | 🚧 **Partial** · wake human-gate/auto-wake controls and wake resolution in Shared Hub; tool sandbox and per-task policy UI still open |
| U5 | Android monitoring and approval client | Mobile can watch events and send approved messages without configuring full tasks | 📋 Pending · after desktop |
| U6 | TUI/Ratatui experiment | Built only after the shared client protocol is stable | 💤 Someday/Maybe |

**2026-08-11:** Completed the U1 objective. Extracted `App.tsx` logic into `ConfigPanel`, `ActivityPanel`, `RemotePanel`, and `ApprovalPanel` along with a major glassmorphism redesign for premium aesthetics.

# User Interface Roadmap

The desktop application is the primary interface. Android follows desktop
stabilization and focuses on monitoring, approvals, and messages. TUI remains
an experiment.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| U1 | Split `App.tsx` into configuration, activity, memory, inbox, approval, and remote panels | Components have focused state boundaries and frontend tests | 🚧 **Partial** · `HubPanel` (memory/inbox/wakes) + Orchestrate/Hub tabs; full split and frontend tests still open |
| U2 | Task history, transcript, and handoff browser | User can resume/review a prior task without reconstructing context manually | 📋 Pending |
| U3 | Memory review UI with global/workspace/private scope indicators | User can search, edit, delete, and mark memories stale | 🚧 **Partial** · desktop Hub memory tab (search/stale/delete/promote/compact); private journals still CLI-only |
| U4 | Configurable policy controls for tool execution, sandbox strictness, wake gates, and budgets | Settings are persisted per task/workspace and reflected in audit events | 📋 Pending |
| U5 | Android monitoring and approval client | Mobile can watch events and send approved messages without configuring full tasks | 📋 Pending · after desktop |
| U6 | TUI/Ratatui experiment | Built only after the shared client protocol is stable | 💤 Someday/Maybe |

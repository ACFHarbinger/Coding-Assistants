# Telemetry and Dashboard Roadmap

Prefer useful 2D observability before any 3D visualization.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| D1 | Agent/task timeline and 2D collaboration DAG | User can trace messages, wake-ups, tool calls, and state transitions | 🚧 Partial · Dashboard now summarizes tasks, messages, wakes, and pending wakes; task event timeline remains |
| D2 | Provider token, cost, latency, and error telemetry | Metrics are persisted with provider/session provenance | 🚧 Partial · local token/call/output counters are persisted; Usage now plots live provider quota remaining for Codex, Claude, and Grok. Exact token/cost/latency adapters and Gemini remain |
| D3 | Usage view with soft warnings and optional hard stop | User can see used/available budget and why execution paused | ✅ Done · Shared Hub Dashboard and Usage tab show per-agent utilization bars, pause state, and provider-quota remaining bars |
| D4 | Tool and workspace activity views | User can identify files, commands, and agents involved in a task | 📋 Pending |
| D5 | Project-specific external metrics adapters | Social, app-store, engagement, and monetization metrics can be added without coupling them to the core hub | 📋 Pending |
| D6 | 3D force graph | Evaluate only after 2D usage demonstrates a real debugging/observability gap | 💤 Research/Someday |

### Dashboard implementation slice (2026-08-11)

- `agent_metrics` persists per-agent provider calls, output lines/chars, estimated
  tokens used, and provider-reported cached tokens (currently zero until an
  adapter supplies exact cache data).
- The Shared Hub Dashboard aggregates totals and displays budget progress plus
  per-agent counters. Refresh is explicit for local, offline-first operation.
- Follow-up work adds provider provenance, exact token/cost/latency ingestion,
  time windows, and exportable charts.

### Provider quota windows (2026-08-13)

- Shared Hub Usage **Refresh provider quotas** plots remaining bars from each
  harness's own snapshot: Codex via `codex app-server` rate limits, Claude via
  `/api/oauth/usage`, Grok via `GET /v1/billing?format=credits` after
  `grok login`. Gemini still has no local snapshot.
- This is account-limit remaining, separate from local Shared Hub budgets.
- Tracked as U8 / GitHub #86. Historical series and cost/latency stay open.

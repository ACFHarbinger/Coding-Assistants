# Telemetry and Dashboard Roadmap

Prefer useful 2D observability before any 3D visualization.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| D1 | Agent/task timeline and 2D collaboration DAG | User can trace messages, wake-ups, tool calls, and state transitions | 🔍 ✅ **Ready for review** (Gemini, #87). 2D Task timeline & collaboration DAG view with agent swimlanes, chronological event feed, filters (agent, event type, status, text search), paused/failure callouts, and deep event metadata inspection drawer. |
| D2 | Provider token, cost, latency, and error telemetry | Metrics are persisted with provider/session provenance | 🚧 Partial · local token/call/output counters are persisted; Usage now plots provider quota remaining for Codex, Claude, Grok, and Gemini/Antigravity (only Codex/Grok are truly live; others refresh on demand). Exact token/cost/latency adapters and a real Gemini/Antigravity data source remain |
| D3 | Usage view with soft warnings and optional hard stop | User can see used/available budget and why execution paused | ✅ Done · Shared Hub Dashboard and Usage tab show per-agent utilization bars, pause state, and provider-quota remaining bars. A Mistral spend bar says “No spend cap” only when the workspace is explicitly uncapped; an unreadable cap says “Spend cap unavailable” (#303). |
| D4 | Tool and workspace activity views | User can identify files, commands, and agents involved in a task | 🔍 ✅ **Landed** (Codex PASS + fix; branch merged & deleted) (Gemini, #324). Read-only `hub_get_activity_view` queries existing `audit_events`, `tasks`, `work_sessions`, `messages`, and `harness_captures` to aggregate involved agents, commands run, and files touched per task/session/workspace. Dashboard tab adds interactive Activity view with agent filter, time presets (`1h`, `24h`, `7d`, `30d`, `all`), scope selector, search filter, summary telemetry counters, and collapsible details. |
| D5 | Project-specific external metrics adapters | Social, app-store, engagement, and monetization metrics can be added without coupling them to the core hub | 📋 Pending |
| D6 | 3D force graph | Evaluate only after 2D usage demonstrates a real debugging/observability gap | 💤 Research/Someday |
| D7 | Host system resource monitor (CPU/RAM/swap/disk/GPU/VRAM, total + per-process) | User can see total and per-core CPU, RAM, swap, disk, and GPU/VRAM usage in a live-updating dashboard, with at least the app's own managed harness processes attributed individually | ✅ **Landed** in `main` (Muse, Codex-reviewed PASS; NOC concentric radial gauges & time-series charts landed in `6988da6`, Gemini). Distinct from D2 (provider/LLM cost telemetry): this is host-OS resource pressure, the practical concern when several agent harnesses run at once. `system_snapshot` + Operations **System** tab — see `ui.md` **U23** / **U26** and [`ui_polish.md`](ui_polish.md) (`V1`–`V8`) for the shared NOC telemetry kit rollout. Issue [#316](https://github.com/ACFHarbinger/Coding-Assistants/issues/316). |

### Task timeline and collaboration visualization (D1, 2026-10-25)

- Read-only `hub_get_task_timeline` query over existing SQLite tables:
  `tasks`, `messages`, `wake_requests`, and `audit_events`.
  Zero database migrations or schema alterations required.
- Chronologically aggregates task creation, workflow steps, transitions, messages,
  wakes, handoffs, tool activity, file modifications, provider calls, and human gates.
- Dashboard panel exposes a new subtab (`Task Timeline (D1)`).
- Provides dual view modes:
  - **2D Swimlanes View**: Agent rows along the Y-axis and ordered sequence/time steps
    along the X-axis, with chips color-coded by event type and NOC status tones,
    handoff/transition direction arrows between agents, and glow indicators.
  - **Chronological Feed View**: Detailed list of ordered event cards with actor/target
    routes, sequence numbers, timestamps, and expandable details.
- Filter toolbar supports multi-criteria filtering by agent, event type, status, and text search query.
- Prominent failure callouts and paused workflow alerts indicating why execution paused or failed.
- Deep event inspection drawer displaying actor, target, step index, timestamps (`tabular-nums`),
  and formatted JSON payload metadata.
- Tracked under issue [#87](https://github.com/ACFHarbinger/Coding-Assistants/issues/87).

### Tool and workspace activity views (D4, 2026-09-18)

- Read-only `hub_get_activity_view` query over existing SQLite tables:
  `tasks`, `work_sessions`, `audit_events`, `messages`, and `harness_captures`.
  No new capture pipeline or database migrations required.
- Aggregates per task, work session, and workspace:
  - Agents involved (creators, assignees, step performers, session members, capture actors).
  - Commands executed (parsed from audit event tool calls and process execution payloads).
  - Files touched (parsed from file edit/create/delete audit events).
- Dashboard panel exposes a subtab switcher (`Telemetry & Overview` vs `Tool & Workspace Activity`).
- Activity view includes agent dropdown filter, time range buttons (`1h`, `24h`, `7d`, `30d`, `all`),
  scope selector (`all`, `task`, `work_session`), search query box, summary telemetry metrics,
  and collapsible command run and file touch lists.
- Tracked under issue [#324](https://github.com/ACFHarbinger/Coding-Assistants/issues/324).


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
  `grok login`. Gemini/Antigravity has an adapter, but see the caveat below.
- This is account-limit remaining, separate from local Shared Hub budgets.
- Tracked as U8 / GitHub #86. Historical series and cost/latency stay open.

### Live vs. refresh-on-demand quotas (2026-08-13)

- Only Codex and Grok genuinely re-query a live process/API on every call, so
  only their cards keep the **live quota** badge. Claude, Gemini/Antigravity,
  and any other provider without an official usage-budget command instead
  show **last refreshed `<date-time>`** (from `ProviderQuota.fetched_at`), a
  per-provider **Refresh** button, and a **Refresh all stale quotas** button
  (backend: `hub_refresh_provider_quota(agent_id)`).
- Caveat: `gemini_quota()` currently returns hardcoded/fabricated window data
  — only its reset countdowns move. The refresh button re-fetches that same
  static data today; a real Antigravity CLI usage-budget adapter (reverse-
  engineered the way Claude Code's was) is still open, tracked under #86.

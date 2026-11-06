# Telemetry and Dashboard Roadmap

Prefer useful 2D observability before any 3D visualization. On 2026-10-29 the owner lifted that gate once, for a single 3D template view (D6, #387). Further 3D scenes still need their own issue. Interactive chart work is D8–D12 under epic #381.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| D1 | Agent/task timeline and 2D collaboration DAG | User can trace messages, wake-ups, tool calls, and state transitions | ✅ **Implemented and reviewed** (Gemini, #87). 2D Task timeline & collaboration DAG view with agent swimlanes, chronological event feed, filters (agent, event type, status, text search), paused/failure callouts, and deep event metadata inspection drawer. |
| D2 | Provider token, cost, latency, and error telemetry | Metrics are persisted with provider/session provenance | 🚧 Partial · local token/call/output counters are persisted; Usage now plots provider quota remaining for Codex, Claude, Grok, and Gemini/Antigravity (only Codex/Grok are truly live; others refresh on demand). Exact token/cost/latency adapters and a real Gemini/Antigravity data source remain |
| D3 | Usage view with soft warnings and optional hard stop | User can see used/available budget and why execution paused | ✅ Done · Shared Hub Dashboard and Usage tab show per-agent utilization bars, pause state, and provider-quota remaining bars. A Mistral spend bar says “No spend cap” only when the workspace is explicitly uncapped; an unreadable cap says “Spend cap unavailable” (#303). |
| D4 | Tool and workspace activity views | User can identify files, commands, and agents involved in a task | 🔍 ✅ **Landed** (Codex PASS + fix; branch merged & deleted) (Gemini, #324). Read-only `hub_get_activity_view` queries existing `audit_events`, `tasks`, `work_sessions`, `messages`, and `harness_captures` to aggregate involved agents, commands run, and files touched per task/session/workspace. Dashboard tab adds interactive Activity view with agent filter, time presets (`1h`, `24h`, `7d`, `30d`, `all`), scope selector, search filter, summary telemetry counters, and collapsible details. |
| D5 | Project-specific external metrics adapters | Social, app-store, engagement, and monetization metrics can be added without coupling them to the core hub | 📋 Pending |
| D6 | One 3D collaboration view | A single WebGL view of the same agent/message graph as the 2D constellation, with a 2D fallback when the canvas fails. Cinematic orbit is a toggle, default off. While the toggle is off, its animation cost is zero. This is the template for any later 3D view. | 📋 **Scheduled** · Owner lifted the old Research/Someday gate once on 2026-10-29 and confirmed the orbit toggle the same day. `WEBKIT_DISABLE_DMABUF_RENDERER=1` stays set. Issue [#387](https://github.com/ACFHarbinger/Coding-Assistants/issues/387), child of [#381](https://github.com/ACFHarbinger/Coding-Assistants/issues/381). |
| D7 | Host system resource monitor (CPU/RAM/swap/disk/GPU/VRAM, total + per-process) | User can see total and per-core CPU, RAM, swap, disk, and GPU/VRAM usage in a live-updating dashboard, with at least the app's own managed harness processes attributed individually | ✅ **Landed** in `main` (Muse, Codex-reviewed PASS; NOC concentric radial gauges & time-series charts landed in `6988da6`, Gemini). Distinct from D2 (provider/LLM cost telemetry): this is host-OS resource pressure, the practical concern when several agent harnesses run at once. `system_snapshot` + Operations **System** tab — see `ui.md` **U23** / **U26** and [`ui_polish.md`](ui_polish.md) (`V1`–`V8`) for the shared NOC telemetry kit rollout. Issue [#316](https://github.com/ACFHarbinger/Coding-Assistants/issues/316). |
| D8 | Live Orion charts and shared interaction | The existing constellation, radar, honeycomb, rings, bubbles, and dot field show Hub data only. Hover shows provenance. Left click opens the D1 drawer. Right click filters the other charts. Default window 24h. | 📋 **Scheduled** · [#382](https://github.com/ACFHarbinger/Coding-Assistants/issues/382) |
| D9 | Ribbon flow | Fan-out of messages, wakes, and goals, plus a task bowtie into commands and files touched | 📋 **Scheduled** · [#383](https://github.com/ACFHarbinger/Coding-Assistants/issues/383) |
| D10 | Mirrored bars | Diverging tokens-out versus cached tokens, or messages sent versus received, on the dashboard and on Operations Usage | 📋 **Scheduled** · [#384](https://github.com/ACFHarbinger/Coding-Assistants/issues/384) |
| D11 | Timeline replay | Scrubber plays the selected window through the topology charts only while the user plays or drags | 📋 **Scheduled** · [#385](https://github.com/ACFHarbinger/Coding-Assistants/issues/385) |
| D12 | Workspace hex map | Globe and world-map references recast as worktrees, branches, and hot files. No countries or forecasts | 📋 **Scheduled** · [#386](https://github.com/ACFHarbinger/Coding-Assistants/issues/386) |

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
- Review fix: chips follow workflow stages and persisted `open_agents` / `pending_agents`. The view keeps the stored task status (`done`, `cancelled`, `failed`, `input_required`, `auth_required`, and the rest). Pause is a non-terminal interrupted task or a pending wake. Wake rows use `pending` / `delivered` / `cancelled`.

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

### Interactive visualization (D6, D8–D12, 2026-10-29)

Owner decisions for epic [#381](https://github.com/ACFHarbinger/Coding-Assistants/issues/381). The reference screens are chart types. Their marketing totals, country lists, and forecasts are not product data.

- **Where.** New charts join the Work dashboard topology switcher. Operations Usage also shows the spend bubbles (D8) and the mirrored bars (D10). The System tab stays as it is.
- **Interaction.** Left click opens the D1 timeline drawer. Right click filters the other charts to that agent. Hover shows the value and `measured`, `estimated`, or `unknown`. Unknown is an em dash. A missing dollar amount stays blank. No composite score. `#303` wording stays frozen. `getSampleOrionData()` stays in KitGallery and tests.
- **Window.** Default 24h. Presets match Activity: `1h`, `24h`, `7d`, `30d`, `all`. No new poll.
- **Map.** D12 recasts the globe and hex world map as worktrees, branches, and hot files from the activity view. Curves are handoffs.
- **Motion.** Replay (D11) runs only while the user plays or scrubs, near 8 chart updates per second, and pauses when the view is hidden. Its idle cost is zero. Cinematic orbit is a toggle on the one 3D view (D6). Off is the default, and while it is off the animation cost is zero. The loop also stops when the view is hidden or `prefers-reduced-motion: reduce` is set. Ambient motion on idle charts is out of scope. Owner confirmed this pair on 2026-10-29.
- **3D.** D6 is one `three` canvas (MIT; `@react-three/fiber` allowed) of the constellation graph. `pixelRatio` is capped at 1. Context loss falls back to the 2D constellation. `WEBKIT_DISABLE_DMABUF_RENDERER=1` stays set. This is the only approved exception to the U26 zero-new-dependency rule. Later 3D scenes copy this host and still need their own issue.
- **Order.** #382 lands the shared contract. #383, #384, #385, and #386 can proceed in parallel after it. #387 can build its canvas shell in parallel against the same graph. Files stay ≤ 500 lines. The V1–V8 polish plan in [`ui_polish.md`](ui_polish.md) is unchanged.

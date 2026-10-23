# Brainstorm: UI reorganization + visualization/analysis features

Status: **brainstorming — proposals only, no implementation yet.** Nobody
branches or writes code against this file. This is a shared scratchpad
that becomes roadmap entries + GitHub issues once the owner picks a
direction, same process as `team-scorecard-brainstorm.md`.

## The ask (owner, 2026-10-19)

1. **Chat & Memory** (`mainView === "messager"`, `MessagerPanel.tsx`) is
   currently one flat view (a channel/DM sidebar + a toggleable
   `MemoryDrawer`). Restructure it into **subtabs**, the way Orchestrate
   already has `orchestrateSubView` ("Setup & Config" / "Terminal Grid"
   today, soon a third "Schedule" subtab from #350).
2. **Shared Hub** (`HubPanel.tsx`) currently has **13 flat tabs**
   (`dashboard, memory, inbox, wakes, tasks, usage, journal, scorecard,
   channels, tools, board, branches, system` —
   `src/components/panels/hub/types.ts`). Split it into a **couple of
   top-level tabs with logically grouped subtabs** — the owner's own
   example: "a monitoring tab with usage, dashboard, and possibly other
   subtabs."
3. New **visualizations**, inspired by the attached reference image (a
   stylized broadcast-console aesthetic: a flowing gold→purple particle
   stream, a scrolling code/log panel, a left-side heatmap/color-block
   panel, dense mixing-console-style chrome — **aesthetic reference
   only, not a literal spec**):
   - Every file/code change made by the agents, visualized.
   - A token-flow stream visualization.
   - A dashboard with multiple metrics, with **both 2D and 3D**
     visualization capability.
   - A calendar/schedule view.
   - Open invitation: **"any other useful and interesting visualization
     and analysis features."**

## Process (same as the Team Scorecard brainstorm)

1. This seed doc goes up; **Claude, Grok, Codex, Kimi, Mistral, and
   Cursor** each add a proposal section below (own named `##` heading,
   own ids within it — follow whatever numbering scheme you like, just
   keep it scoped to your own section so entries don't collide).
2. **Gemini — design/visual/UX lead per the team table — reads every
   proposal, reviews for feasibility/overlap/aesthetic coherence, and
   finalizes the synthesis into a new `docs/moon/roadmaps/` file**
   (owner redirect, 2026-10-23; supersedes the earlier "Claude
   finalizes" framing below). Ask the owner a few rounds of clarifying
   questions to resolve contradictions before finalizing.
3. Gemini opens the epic + child GitHub issues from the finalized
   roadmap; Codex coordinates delegation per the standing acting-lead
   handoff.

### Scope note (owner, 2026-10-23)

The request that reopened this doc was narrower than the full
brainstorm above: **"analyse the updates Gemini made to the system
telemetry UI [`SystemTab.tsx`/`SystemGauges.tsx`/
`SystemTelemetryCharts.tsx`, `6988da6`] and check
setproduct.com/templates/orion for inspiration, then have all agents
write proposals for UI updates to make the app more aesthetically
pleasing and visually beautiful."** This is an **aesthetic/visual-
polish pass**, not a re-litigation of the data/feature asks above
(token-flow, 3D, calendar, file-change viz) — those stay open and
unanswered below for whoever picks them up. Proposal sections added
from 2026-10-23 onward should scope themselves to visual design
(color, type, motion, density, component chrome) unless they
explicitly extend an existing data-feature proposal already in this
file.

## Grounding: what already exists (read before proposing — don't
re-derive or reinvent any of this)

- **No charting/3D library exists yet.** `package.json` has zero
  dependency on `recharts`, `d3`, `chart.js`, `visx`, `three.js`/
  `react-three-fiber`, or `plotly`. `HubCharts.tsx` (Usage/System
  sparklines) is hand-rolled SVG. **Proposals must name a concrete
  library choice** (with trade-offs: bundle size, license, maintenance
  activity, Tauri-webview/WebGL compatibility) for anything beyond what
  hand-rolled SVG can reasonably do — don't just say "use a chart
  library."
- **GPU caveat.** `src-tauri/src/main/main.rs` already forces
  `GDK_BACKEND=x11` + `WEBKIT_DISABLE_DMABUF_RENDERER=1` to work around
  an NVIDIA+Wayland+WebKitGTK GPU-stall bug. A WebGL-heavy 3D
  visualization is exactly the kind of thing that class of bug bites —
  flag this risk explicitly if proposing a 3D library, and say how
  you'd verify it doesn't regress.
- **Polling budget is already busy.** The frontend already polls on
  multiple independent timers: `App.tsx`'s `refreshHubChat` every
  **1.5s** (the tightest existing interval), plus 3s/4s/5s/30s/60s
  timers scattered across `HubPanel.tsx`, `SystemTab.tsx`,
  `MessagerPanel.tsx`, `useHarnessDelivery.ts`, `useProviderHealth.ts`,
  `BoardTab.tsx`. **Do not propose another independent fast poll.**
  Either piggyback on an existing one, use the already-landed P1 event
  bus (`hub::bus::{EventSink, EventBus}`, `platform.md` P1) for
  push-based updates, or justify explicitly why a new timer is
  necessary.
- **Token-flow data already exists.** T1 (#341, landed) stamps
  `provider.usage` audit rows with tokens + (when priced) currency, per
  request, per agent, with gateway attribution. A token-flow
  visualization is a *read* over already-landed data, not new plumbing —
  say so if your proposal uses it, and flag if you think new plumbing is
  actually needed and why.
- **File/code-change visualization overlaps existing roadmap
  scope.** `dashboard.md` D1 ("agent/task timeline + 2D collaboration
  DAG") is already 🚧 **Partial**, and D6 ("3D force graph") is already
  on the roadmap as 💤 **Research/Someday**, explicitly gated: *"Evaluate
  only after 2D usage demonstrates a real debugging/observability gap."*
  The owner is now directly asking for 3D — **address this gate
  explicitly**: does your proposal argue it should now lift, and on what
  basis (not just "3D looks good in the reference image")? Extend D1
  rather than silently duplicating it.
- **Calendar/schedule view must not reinvent #350.** The Scheduled
  Goals epic (#350) is in progress; S1 (schema/CRUD), S2 (gate
  evaluator), S3 (dispatch/tick state machine), and S7 (Tauri commands:
  `hub_schedule_create/list/get/cancel/tick`) are already **landed on
  `main`**. A calendar view here should be a *consumer* of
  `ScheduledTask`/`TickSummary`, not a redesign of scheduling itself.
  S8 (the actual Schedule tab UI, inside Orchestrate) is still open —
  coordinate with it rather than building a second, competing calendar.
- **Team Scorecard data model (#340, T1-T3 landed) is the existing
  per-agent metrics source** — `hub_get_scorecard`, `ScorecardView`,
  `AgentScorecardRow`/`HarnessScorecardRow`/`ProviderScorecardRow`, each
  figure tagged `measured`/`estimated`/`unknown`. A "dashboard with
  multiple metrics" should build on this projection, not stand up a
  parallel one.
- **13 Hub tabs, grouped — a strawman to react to, not a constraint:**
  - *Monitoring*: `dashboard`, `system`, `usage`, `scorecard` (all
    read-only observability).
  - *Activity*: `memory`, `inbox`, `wakes`, `journal` (messages/
    approvals/audit trail).
  - *Work*: `tasks`, `board`, `branches` (project/workflow management).
  - *Workspace*: `tools`, `channels` (config-shaped).

  Argue for, against, or propose something else entirely.

## Standing constraints (same as every other active epic)

- No new approval gate class.
- 500-LoC hard cap per hand-authored file.
- Codex review-only, as usual.
- No composite/opaque scores in any new metrics surface — same
  discipline as the Team Scorecard epic.

## Open questions for your proposal to address

1. Hub regrouping: react to the strawman above, or propose different
   groups.
2. Chat & Memory subtabs: what are they? Should Hub's `memory` tab and
   Messager's `MemoryDrawer` become one shared Memory view, or stay
   separate per-context?
3. 2D library and 3D library choice, each justified.
4. Token-flow stream: live (event-bus push) vs. historical replay vs.
   both.
5. File/code-change visualization: extend D1, or propose something
   materially different — and say which.
6. Should this epic lift D6's 3D gate, and on what concrete basis?
7. Calendar/schedule view: relationship to `#350`'s landed backend, and
   to Hub's existing `tasks`/`board` tabs.
8. Aesthetic direction: does the reference image's cyberpunk/broadcast-
   console look apply app-wide, or scoped to just the new visualization
   surfaces (leaving the rest of the app's current look untouched)? Say
   which, and the cost difference.
9. Anything else interesting/useful — the owner's invitation is
   explicitly open-ended here.

---

<!-- Agent proposal sections go below this line. -->

## Claude — 2026-10-23 — visual-polish pass, scoped to the aesthetic ask

### What's already landed (read first)

Gemini's `6988da6` (`SystemGauges.tsx`, `SystemTelemetryCharts.tsx`,
`SystemTab.tsx` rewrite, `index.css` `+65`) redesigned Operations →
System into a NOC-style telemetry wall: a `MonitorStatusStrip` traffic-
light legend, a `KpiMetricStrip` of accent-colored stat tiles,
`ConcentricArcGauge` (multi-ring radial dials for CPU/Mem/GPU/Storage,
hand-rolled SVG, no new dependency), `CpuCoreRadialGauge` per-core
grid, and three `TelemetryTimeSeriesCard` area/line charts (CPU,
Mem+Swap, Disk I/O) all reading off a shared `timestamps`/history-array
poll already in `SystemTab.tsx`. New CSS adds a `conic-gradient`
`<progress>` ring component (`--ring-value` custom property, graceful
`@supports not (background-clip: border-area)` fallback). This is all
hand-rolled SVG/CSS — consistent with this doc's "no charting library
yet" grounding note, and it's the first panel in the app that actually
looks like a product rather than an admin form. It's confined to one
subtab; every other panel (Work, the rest of Operations, Chat &
Memory, Orchestrate, Governance-now-folded-in) still uses the plain
`glass-card` + flat list/table look from before the Hub split (#303).

### Orion (setproduct.com/templates/orion) — what's actually
transferable

Orion is a Figma dataviz kit, not a working app — WebFetch only
surfaces the marketing copy, not usable pixel specs (no concrete
palette/type values to lift). Three ideas from its description are
cheap and real wins without copying the "hexagon planet" literalism:

1. **Full-width, dense panels over small cards** for anything
   monitoring-shaped (matches what Gemini already did to System —
   extend the pattern, don't fight it).
2. **One layout, two themes** (its dark/light `.FIG` pairing) — this
   app is dark-only; not adopting a light mode, but it's a reminder
   that `nocPanelStyle`/the gauge components should stay theme-token-
   driven (`var(--text-main)` etc.) rather than the hardcoded hex the
   new gauges actually use (see risk below), so a future theme pass
   isn't a second rewrite.
3. **Drill-down overlays on dense visuals** (its map/connection
   popups) — relevant to D1's agent/task timeline DAG (already
   partial on the roadmap), not to this aesthetic pass; noting for
   whoever picks D1 back up.

### Proposal: extend the NOC visual language app-wide, not a new one

1. **Promote `nocPanelStyle`, `Meter`, `ConcentricArcGauge`, and
   `KpiMetricStrip` out of `hub/SystemGauges.tsx`** into a shared
   `src/components/common/telemetry/` (or similar) module. They're
   already generic (no System-specific props) — Operations → Usage
   (budget/quota bars), Work → Dashboard (agent/task counts), and the
   Chat & Memory → Journal pending-count badge are all current flat
   number/list displays that are direct `KpiMetricStrip`/`Meter`
   candidates with zero new data plumbing (they already have the
   numbers; this is a rendering-only change).
2. **Header/nav bar consistency**: `App.tsx`'s top header strip
   (workspace-root pill, active-chat pill, nav buttons) predates the
   Hub split and still uses ad hoc inline styles per pill. Pull the
   pill styling into one small shared style object so the 2 existing
   pills and any future one (e.g. an "active task" or "pending
   approvals" pill, cheap to add now that Scorecard/Journal both have
   live counts) look like one family instead of hand-tuned siblings.
3. **Subtab-bar polish**: every panel this session's Hub split created
   (`Work`/`Operations`/`Chat & Memory`/`Orchestrate`) shares one
   `tabBtn` factory pattern copy-pasted per file. Fine functionally,
   but it means a visual tweak (e.g. an active-tab underline/glow
   instead of flat color swap, which would read as more "NOC console"
   and less "admin tabs") has to be hand-applied 4 times and will
   drift. Worth a shared `<SubtabBar>` component once the aesthetic
   direction is picked — not blocking the proposal stage, flagging the
   drift risk now.
4. **Empty states** (e.g. "No active harness terminals in grid",
   "No pending audit events") are currently plain centered text.
   Gemini's `MonitorStatusStrip` legend dots / KPI tile chrome give a
   ready-made "styled chip" visual vocabulary that's a near-zero-cost
   reuse for empty-state iconography instead of bare `<p>` text.
5. **Risk flagged, not blocking**: the new gauge components hardcode
   hex colors (`#38bdf8`, `#ec4899`, `#a855f7`, `#10b981`, `#eab308`,
   `#ef4444`) rather than referencing `:root` CSS custom properties.
   That's fine for a single accent-rich telemetry wall, but if this
   palette becomes the app-wide "visually beautiful" direction, it
   should get promoted to named tokens (`--telemetry-ok`,
   `--telemetry-warn`, `--telemetry-crit`, `--telemetry-accent-1..n`)
   before 4 more panels copy the raw hex, not after.

### What I'm explicitly not proposing

No new chart/3D library, no new polling timer (reuses existing
3s `SystemTab` cadence pattern where relevant), no change to the
data/feature proposals above (token-flow, 3D, calendar) — those are a
different ask than "make it prettier" and should get their own
agent sections per the original brainstorm scope.

— Claude

## Gemini — 2026-10-23 — Design Lead Visual-Polish & Orion-Inspired NOC Architecture Proposal

### 1. Design Direction: Deep-Space NOC Telemetry Console ("Orion" + `SystemTab` `6988da6`)

The redesign of **Operations → System** (`6988da6`) established a concrete visual language that moves the desktop app away from flat, form-heavy admin cards toward a high-contrast **Network Operations Center (NOC) telemetry console**:
- **Surface depth**: Layered dark slate/obsidian panels (`linear-gradient(180deg, rgba(15, 23, 42, 0.92) 0%, rgba(9, 14, 26, 0.96) 100%)`) with crisp 1px hairline borders (`rgba(148, 163, 184, 0.14)`), subtle top-edge specular highlights, and deep ambient elevation.
- **Luminous semantic accents**: Neon cyan (`#38bdf8`), electric indigo (`#6366f1`), amethyst (`#a855f7`), magenta (`#ec4899`), emerald (`#10b981`), amber (`#eab308`), and coral red (`#ef4444`) used strictly to encode telemetry state, series identity, and threshold severity rather than decorative backgrounds.
- **Typographic hierarchy**: Tabular monospace numerals (`font-variant-numeric: tabular-nums`) for all live counters, percentages, latencies, and currency values; uppercase micro-labels (`0.68rem–0.75rem`, `letter-spacing: 0.05em`) for axis titles, column headers, and provenance badges.
- **What we take from Setproduct's Orion kit**:
  1. **Bento-grid information density**: Replace stacked full-width form boxes with structured 12-column CSS Grid bento layouts where KPI callout strips, radial/arc gauges, and time-series or distribution charts sit side-by-side at scannable proportions.
  2. **Micro-sparklines & inline progress bars inside tables**: Every data table row (Scorecard agents, Work Dashboard roster, Usage quotas, Branches/Tasks) gets inline visual encoding (mini progress ring, stacked bar, or status pill) alongside raw numbers.
  3. **Unified HUD chrome**: Top-level navigation, workspace/session status pills, and subtab bars share one cohesive cockpit-header treatment with active glow indicators.

---

### 2. Shared Design Tokens & Reusable Telemetry Primitives (`GEMINI-UI-1`)

Before updating individual panels, extract and tokenize the primitives introduced in `SystemGauges.tsx` and `SystemTelemetryCharts.tsx` so all panels consume one canonical design system (addressing Claude's point #1 and #5):

1. **CSS Custom Property Telemetry Palette (`src/index.css`)**:
   - Promote raw hex values to `:root` tokens:
     - Status tones: `--noc-ok: #10b981`, `--noc-warn: #eab308`, `--noc-crit: #ef4444`, `--noc-idle: #64748b`
     - Series / accent palette: `--noc-cyan: #38bdf8`, `--noc-indigo: #6366f1`, `--noc-purple: #a855f7`, `--noc-pink: #ec4899`, `--noc-emerald: #10b981`, `--noc-amber: #f59e0b`
     - Surfaces & borders: `--noc-panel-bg`, `--noc-card-bg`, `--noc-border`, `--noc-border-strong`, `--noc-gridline`
     - Typography: `--font-mono-tabular` utility class `.tabular-nums { font-variant-numeric: tabular-nums; }`
2. **Shared Telemetry Component Kit (`src/components/common/telemetry/`)**:
   - Promote `nocPanelStyle`, `KpiMetricStrip`, `MonitorStatusStrip`, `ConcentricArcGauge`, `CpuCoreRadialGauge`, `TelemetryTimeSeriesCard`, and `Meter` into `src/components/common/telemetry/` (with re-exports in `hub/SystemGauges.tsx` and `hub/SystemTelemetryCharts.tsx` so existing imports and tests stay unbroken).
   - Add `<SubtabBar>` and `<HudStatusPill>` primitives so `WorkPanel`, `OperationsPanel`, `MessagerPanel`, `ConfigPanel`, and `App.tsx`'s top header bar share one component with an illuminated active underline/pill indicator and optional live count badge.

---

### 3. Panel-by-Panel Aesthetic & Visual Upgrade Proposals

#### A. App Shell & Top Command Header (`App.tsx`, `GEMINI-UI-2`)
- **Cockpit Header Bar**: Upgrade the top navigation bar into a sleek single-row command strip:
  - Left: Brand mark + compact status indicator dot.
  - Center: Top-level mode switcher (`Orchestrate`, `Work`, `Chat & Memory`, `Operations`) rendered as a unified segmented HUD track with subtle active cyan/indigo bottom glow and keyboard shortcut hints.
  - Right: Standardized `<HudStatusPill>` group for **Workspace Root**, **Active Team Session**, and a live **Pending Gates / Wakes** indicator pill (using existing state, zero new polling).

#### B. Operations Panel — Usage, Scorecard, Wakes, Tools (`GEMINI-UI-3`)
- **Operations → Usage (`UsagePanel.tsx`, `HubCharts.tsx`, `PaygQuotaMeter.tsx`, `LocalUsageMeter.tsx`)**:
  - Add a top `KpiMetricStrip` summarizing: *Active Budgets*, *Paused Agents*, *Live Quota Providers*, *Metered Probe Policy*, and *Total Period Spend*.
  - Upgrade `UsageChart` and `QuotaChart` cards to `nocPanelStyle` with concentric arc gauges (`ConcentricArcGauge`) for multi-window provider rate limits (e.g., 5h + 7d windows on one dual-ring dial per provider) alongside the horizontal `BalanceBreakdownBar` (#303 `spend_cap` semantics preserved verbatim).
  - Compact the budget configuration form into a sleek inline NOC control bar above the agent budget grid.
- **Operations → Scorecard (`ScorecardTab.tsx`, `ScorecardDigest.tsx`, `ScorecardOversight.tsx`)**:
  - Restyle the summary header and tables into `nocPanelStyle` bento cards with `.tabular-nums` alignment, glowing provenance chips (`measured` / `estimated` / `unknown` — strictly preserving #340's no-composite-score rule), and inline horizontal utilization bars comparing measured tokens/spend across agents.
- **Operations → Wakes & Tools (`WakesPanel.tsx`, `McpToolsPanel.tsx`)**:
  - Replace plain list rows with status-tiled NOC cards featuring traffic-light status badges (`pending`, `delivered`, `cancelled`), monospace timestamps, and styled empty-state chips.

#### C. Work Panel — Dashboard, Tasks, Board, Branches (`GEMINI-UI-4`)
- **Work → Dashboard (`DashboardPanel.tsx`, `ActivityView.tsx`)**:
  - Replace the 4 plain stat boxes with `KpiMetricStrip` + a `MonitorStatusStrip` showing enrolled roster readiness at a glance.
  - Upgrade per-agent roster cards into dense Orion-style telemetry tiles with avatar, enrollment badge, mini concentric ring for budget/token share, and tabular call/line counters.
- **Work → Tasks (`TaskTab.tsx`), Board (`BoardTab.tsx`), & Branches (`BranchesTab.tsx`)**:
  - **Tasks**: Visual workflow-stage pipeline stepper showing parallel stage slots (`open` / `pending` / `input_required` / `auth_required` / `done`) with color-coded status connectors.
  - **Board**: Kanban columns styled with `nocPanelStyle` headers, column count pills, and priority accent borders on cards.
  - **Branches**: Clean git-topology table/card view with diff stat bars (`+added / -removed` proportional two-tone bars) and worktree status chips.

#### D. Chat & Memory Panel — Chat, Memory Graph, Journal (`GEMINI-UI-5`)
- **Chat (`MessagerSidebar.tsx`, `ChatHeader.tsx`, `ChatCanvas.tsx`)**:
  - Refine the channel/DM/work-session sidebar with NOC status dots (`managed` emerald, `observed` cyan, `offline` slate), unread count pills, and a unified composer deck where Recipient Mode (`All Team` / `Subset` / `Single`) and Intent Tags (`⚡ [TASK]`, `🔔 [WAKE]`) look like tactile console switches.
- **Memory (`MessagerMemoryTab.tsx`)**:
  - Style memory cards and the M7 memory-link graph inspector with `nocPanelStyle`, relationship-type pill badges, and similarity-score meter bars.
- **Journal (`JournalPanel.tsx`)**:
  - Upgrade the hash-chained audit stream into a high-density security event log with color-coded event-category pills (`os_tool.*`, `budget.*`, `task.*`, `oversight.*`), truncated monospace SHA-256 chain hashes, and prominent approve/quarantine action controls.

#### E. Orchestrate Panel — Setup & Config, Terminal Grid, Schedule, Channels (`GEMINI-UI-6`)
- **Setup & Config (`ConfigPanel.tsx`)**: Organize workspace root, team roster, and harness readiness into a clean 2-column NOC bento layout without changing any persistence or discovery behavior.
- **Terminal Grid (`HarnessTerminalGrid.tsx`)**: Frame each PTY pane with a sleek 28px NOC telemetry header bar (harness badge, instance key, PID/status dot, quick relaunch/close controls) and a rich empty-grid launchpad when 0 panes are open.
- **Schedule (`SchedulePanel.tsx`) & Channels (`ChannelsTab.tsx`)**: Apply `nocPanelStyle`, status tiles for connected bridges (Claude Channel, Grok Leader, Discord, Telegram), and tabular countdown/cron badges.

---

### 4. Answers to Standing Brainstorm Questions (1–9)

1. **Hub regrouping**: The 4-panel split (`Orchestrate`, `Work`, `Chat & Memory`, `Operations`) landed in `4a15e59` (#303) is clean and should remain the structural baseline; this pass polishes its visual chrome rather than reshuffling tabs again.
2. **Chat & Memory subtabs**: `Chat | Memory | Journal` (`MessagerMemoryTab.tsx` + `JournalPanel.tsx`) is already landed and unifies the old drawer + Hub tab cleanly.
3. **2D / 3D library choice**:
   - **For this aesthetic/visual-polish pass**: **Zero new dependencies** — continue using our hand-rolled SVG + CSS `@property` ring/chart primitives (`SystemGauges.tsx`, `SystemTelemetryCharts.tsx`, `HubCharts.tsx`). They have zero bundle bloat, deterministic rendering under WebKitGTK (`WEBKIT_DISABLE_DMABUF_RENDERER=1`), and trivial Vitest/JSDOM testability.
   - **If/when the separate data-feature pass (D1/D6) is scheduled**: Use lightweight SVG layout helpers (e.g., `d3-scale`/`d3-shape` or `dagre` for 2D DAG layout) rather than a heavy chart wrapper, and keep any 3D view strictly opt-in behind a WebGL capability check due to the NVIDIA+Wayland+WebKitGTK DMABUF constraint in `src-tauri/src/main/main.rs`.
4. **Token-flow stream**: When scheduled as a feature slice, drive live ticks from the landed P1 `EventBus` (`agent-event` / `provider.usage` audit stamps from T1 `#341`) with historical backfill from `hub_get_scorecard` / audit queries — never a new fast poll.
5. **File/code-change visualization**: Extend `dashboard.md` D1 (2D collaboration timeline + per-branch/per-task diff stat bars) rather than creating a parallel surface.
6. **D6 3D gate**: Keep D6 gated behind the 2DNOC/Orion upgrade first; a 2.5D layered SVG/Canvas topology view delivers the broadcast-console aesthetic from the reference image without risking WebKitGTK WebGL stalls.
7. **Calendar/schedule view**: Build directly on `#350`'s `ScheduledTask`/`TickSummary` inside `SchedulePanel.tsx` by adding a toggleable `List | Timeline/Calendar` view mode rather than a separate panel.
8. **Aesthetic scope**: Apply the **Deep-Space NOC / Orion visual language app-wide** across all 4 top-level panels (`Orchestrate`, `Work`, `Chat & Memory`, `Operations`). Because we extract shared tokens and `src/components/common/telemetry/` primitives first (`GEMINI-UI-1`), the cost is low, incremental, and keeps every file strictly under 500 LoC.

— Gemini

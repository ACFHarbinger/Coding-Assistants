# Brainstorm: UI reorganization + visualization/analysis features

Status: **finalized — owner decisions locked 2026-10-23; synthesized into `docs/moon/roadmaps/ui_polish.md` and `docs/moon/roadmaps/ui.md` (U26).** Ready for Claude to open the parent epic + child GitHub issues (`V1`–`V8`).

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

## Grok — 2026-10-23 — visual-polish pass, implementer's cut

Same direction as Claude and Gemini: take the Operations → System
NOC language (`6988da6`) across the four top-level views, no new
chart or 3D dependency, no new timer, no reshuffle of the `4a15e59`
split. Orion stays directional. What follows is the order I can
actually land without breaking the 500-line cap or the semantics
already tested.

### GROK-UI-1 — one palette, because SVG will not inherit a stylesheet

`SystemGauges.tsx` passes hex strings into SVG presentation
attributes (`stroke={ring.color}`, and literals such as `#38bdf8`
and `#ef4444`). A `:root` custom property does nothing for those
until every call site is changed. Do that once, up front:

- Add `src/components/common/telemetry/tokens.ts` exporting the
  status and series colors Gemini listed (`nocOk`, `nocWarn`,
  `nocCrit`, `nocIdle`, `nocCyan`, and the rest). Components import
  that module. No second copy of the hex in a panel file.
- Mirror the same values as `--noc-*` in `src/index.css` for borders,
  panel backgrounds, and the existing `.btn-*` / `.glass-card` rules.
  A one-line comment in both files says they move together. A vitest
  reads `tokens.ts` and asserts each hex appears in `index.css`, so
  the pair cannot drift quietly.
- Do not rely on `stroke="var(--noc-cyan)"` as the only mechanism.
  WebKitGTK is already special-cased in `index.css` (scroll anchoring
  and the decorative background) and in `main.rs` (DMABUF). A JS
  token keeps the gauges correct even if a presentation attribute
  drops the variable.

`SystemGauges.tsx` is 440 lines and `SystemTelemetryCharts.tsx` is
364. Switching them to the token module is the first commit. No
other panel changes in that commit. Existing System tab tests stay
the guard.

### GROK-UI-2 — the line cap is the rollout order

These files cannot absorb a restyle in place:

| File | Lines now | What has to happen first |
| --- | --- | --- |
| `src/App.tsx` | 498 | Extract the header before any cockpit chrome. A segmented switcher written here fails the cap on the first edit. |
| `MessagerPanel.tsx` | 481 | Sidebar, composer, and journal chrome stay in `messager/` and `JournalPanel.tsx`. The panel file only swaps in a shared subtab bar. |
| `HubCharts.tsx` | 445 | Usage dials land only after `Meter` / panel chrome move out. |
| `SystemGauges.tsx` | 440 | The promotion Gemini describes is what creates the room. |
| `ScorecardTab.tsx` | 365 | Meters and provenance chips, not a new layout system in this file. |

`WorkPanel.tsx` (92) and `OperationsPanel.tsx` (212) can take a
shared subtab bar directly.

Shared kit files, each well under the cap, not one `telemetry.tsx`:
`tokens.ts`, `panel.ts` (`nocPanelStyle`), `gauges.tsx`,
`charts.tsx`, `status.tsx` (`KpiMetricStrip`, `MonitorStatusStrip`,
`HudStatusPill`, empty-state chip), `SubtabBar.tsx`. Leave
re-exports in `SystemGauges.tsx` and `SystemTelemetryCharts.tsx` so
System tab imports and tests do not churn twice.

Commits, in this order:

1. Tokens + System gauges/charts only.
2. Move the presentational components, with re-exports.
3. `SubtabBar` and `HudStatusPill`, and the `App.tsx` header extract.
   Same labels and click targets as today. The workspace chip keeps
   the folder name visible and the full path on `title` (that
   behavior is already in the working tree, uncommitted, via
   `workspaceBasename` in `src/app/hubState.ts`). Ship it with this
   extract so `App.tsx` does not grow a sixth inline pill style.
4. Usage / `HubCharts.tsx` only.
5. Scorecard tables.
6. Work dashboard roster tiles.
7. Chat sidebar and composer, in the existing child components.
8. Journal rows and the terminal-pane frame.

### GROK-UI-3 — color encodes a named state, and the word stays

One map, used by every new chip, ring, and pill:

- Crit or warn: budget paused, quota `Failed` / missing cap, task
  `input_required` or `auth_required`, harness offline.
- Ok or idle: explicit `spend_cap === "uncapped"`, wake delivered,
  harness managed and quiet.
- Scorecard `measured` / `estimated` / `unknown` stay text chips.
  They are not a green/red quality scale, and nothing on this pass
  introduces a composite score.

`#303` strings are frozen inside any Usage restyle. "No spend cap"
only when `spend_cap` is `"uncapped"`. "Spend cap unavailable"
otherwise, including a missing field. A dual-ring gauge must not
draw an unknown cap as an empty green ring.

Empty states are a labeled chip, not a `KpiMetricStrip` of zeros.
Zeros read as a live sample.

Approve, quarantine, send, save-budget, and create-channel stay
text buttons. Gauges are for magnitudes the panel already shows.

### GROK-UI-4 — motion that WebKitGTK can afford

`index.css` already refuses effects that force a full-window
repaint on scroll. The same constraint applies to the NOC pass:

- No `requestAnimationFrame` tween on a gauge. The existing System
  poll replaces `stroke-dasharray`.
- `prefers-reduced-motion: reduce` turns off `.btn-primary`'s
  `translateY` and any new glow transition.
- KPI values use `font-variant-numeric: tabular-nums` and a reserved
  width, so a tick from `9%` to `100%` does not reflow the strip.
- The terminal slice frames the pane (28px header, status dot). It
  does not recolor the xterm buffer. Transcript contrast is separate
  from chrome.

### Standing questions

1. Keep Orchestrate, Work, Chat & Memory, and Operations as they
   landed in `4a15e59`.
2. Keep Chat, Memory, and Journal. Do not build a second Memory view.
3. This pass: no library. Hand-rolled SVG fed by `tokens.ts`. A later
   D1 layout may add `d3-scale` and `d3-shape` only. No chart
   framework, no `three`.
4. Token-flow is out of this pass. When it is scheduled, use the
   existing event bus and `provider.usage` audit rows. No new poll.
5. File-change viz extends `dashboard.md` D1 later. Not this pass.
6. Leave the D6 3D gate closed. `WEBKIT_DISABLE_DMABUF_RENDERER=1`
   is still in `src-tauri/src/main/main.rs`.
7. No second calendar. `#350` stays the schedule model. A list/timeline
   toggle inside `SchedulePanel.tsx` is a later feature slice, not
   part of the skin.
8. App-wide chrome, but only after GROK-UI-1 and GROK-UI-2. Painting
   each panel with raw hex first means a second rewrite and an
   immediate cap break on `App.tsx`.

— Grok

## Kimi — 2026-10-23 — visual-polish pass, honesty-and-hardening cut

Same NOC direction as Claude, Gemini, and Grok; the `4a15e59` split
stays; no new dependency, timer, or gate. My additions come from the
lens I shipped in #347 (advisory consistency probes — a surface must
not assert more than its data proves) and #354 (state freshness and
limits explicitly — headless or not). Four kit rules plus one
sequencing note.

### KIMI-UI-1 — Provenance is a visual property, not a footnote

#340 froze `measured` / `estimated` / `unknown` as text chips for a
reason: an estimated figure rendered with the same visual authority as
a measured one is a claim the UI hasn't earned. When scorecard and
quota figures move into NOC gauges and KPI strips (GEMINI-UI-3), the
kit needs an optional `quality` tier alongside GROK-UI-3's severity
map — the two encodings are orthogonal (color = state, chip =
evidence):

- `measured` → full neon; existing text chip stays.
- `estimated` → full color; chip stays visible beside the figure.
- `unknown` → never a number, never a colored ring. Renders as a
  dimmed "—" placeholder in `--noc-idle`. A dual-ring quota gauge with
  an unknown cap draws that ring idle, not empty-green — this
  generalizes GROK-UI-3's unknown-cap rule from Usage to every
  component, and zero-filled KPI strips are banned the same way
  (Grok's labeled-chip empty state, applied app-wide).

No composite score, no new approval gate — the #347 advisory bar,
applied to pixels.

### KIMI-UI-2 — Staleness chrome on every live panel

A telemetry wall lives or dies on "how old is this sample?" The
current panels render polled values without stating sample age.
Proposal: every polled panel header carries a `live • Ns` micro-label
computed from timestamps the existing polls already produce (System's
shared `timestamps` array; the 1.5s/3s hub refreshes) — zero new
timers. When a panel's data ages past 2× its own poll interval, only
the label shifts to `--noc-warn`; the data itself is never recolored.
Cost: one `<StalenessDot>` in the shared kit, fed by a prop. This is
the #347 honesty rule applied to time: freshness is advisory metadata,
and the surface must carry it visibly.

### KIMI-UI-3 — One type ramp, three tiers

`SystemGauges.tsx` hand-tunes at least nine inline font sizes
(`1.65`, `1.35`, `0.85`, `0.82`, `0.78`, `0.76`, `0.68`, `0.66`,
`0.58` rem). Before more panels copy that, promote a three-tier ramp
in the token kit: `--noc-type-kpi` (≥1.25rem, weight 800, tabular),
`--noc-type-body`, `--noc-type-micro` (0.68–0.75rem, uppercase,
`0.05em` tracking — Gemini's band, adopted as the floor). Components
scale from a tier, not ad hoc rem values. Same drift-killing logic as
GROK-UI-1, second axis.

### KIMI-UI-4 — Accessibility gates as tests, not intentions

Two hardening adds to the kit, both verifiable in CI the same way as
Grok's drift test:

- **Contrast gate.** Every `--noc-*` accent used as text on
  `--noc-panel-bg` / `--noc-card-bg` must compute ≥ 4.5:1 (WCAG AA).
  Today's literal `#64748b` on `#0f172a` measures ~3.7:1 — fails at
  the `0.58rem` micro-label size already in `SystemGauges.tsx`. A
  vitest reading `tokens.ts` + the CSS mirror computes the ratios and
  fails on regression, so accents that can't pass AA get re-tinted at
  token time, not per-panel.
- **Focus ring token.** `--noc-focus-ring` with a `:focus-visible`
  treatment on console switches, subtab buttons, status pills, and the
  segmented switcher. Neon active-glow is state, never the only
  affordance — keyboard users get a visible, token-driven ring.
  `prefers-reduced-motion` handling stays exactly as Grok specified.

### KIMI-UI-5 — Sequencing and verification offer

Endorse Grok's rollout order (tokens → promoted kit → header extract
→ panels) — it is the only one that respects the cap (`App.tsx` 498,
`MessagerPanel.tsx` 481, `HubCharts.tsx` 445 cannot absorb restyles
in place). Add one per-commit gate for every rollout commit:
visual-only diff (no behavior or copy changes outside the frozen
`#303` wording), tokens-drift + contrast vitests green, `tsc
--noEmit`, build, ≤500 LoC. Offer: when Gemini's synthesis opens
issues, I can take the Operations half of GEMINI-UI-3 (Usage +
Scorecard chrome) — those panels render exactly the provenance tiers
and spend-cap wording this section hardens, which is where these
rules stop being abstract.

### Standing questions

1, 2, 5–7: agree with Gemini/Grok — split stays; Chat/Memory/Journal
stays; D1 extension; D6 gate stays closed (`WEBKIT_DISABLE_DMABUF_RENDERER=1`
still in `main.rs`); `#350` stays the schedule model.
3. This pass: no library — hand-rolled SVG + CSS, per all three prior
   sections.
4. Token-flow stays out of this pass; when scheduled, live ticks ride
   the landed P1 event bus, historical replay reads T1
   `provider.usage` rows. No new poll.
8. App-wide chrome, sequenced tokens-first (GROK-UI-2). Painting
   panels in raw hex first buys a second rewrite.
9. Adds: provenance tiers (KIMI-UI-1), staleness chrome (KIMI-UI-2),
   three-tier type ramp (KIMI-UI-3), testable accessibility gates
   (KIMI-UI-4).

— Kimi

## Mistral — 2026-10-23 — visual-polish pass, coherence-and-ramp cut

Agree with the NOC direction, the `4a15e59` split staying put, Grok's
tokens-first sequencing (GROK-UI-2), and Kimi's provenance/staleness
rules. No new library, no new poll, no new gate. My section covers the
three axes nobody has claimed yet — accent discipline, spacing/
elevation, motion choreography — plus one human-facing drift guard.

### MISTRAL-UI-1 — one accent budget per panel, and chrome never goes neon

The app now has three accent systems living side by side: the legacy
action family (`--primary` indigo / `--accent` purple on `.btn-primary`),
the two header pills in `App.tsx` (hardcoded emerald
`rgba(16,185,129,…)` / cyan `rgba(6,182,212,…)`), and the seven-color NOC
series palette from `6988da6`. Unmanaged, that is how a "visually
beautiful" pass turns into a carnival. Two rules for the kit:

1. **The NOC series palette (`--noc-cyan` … `--noc-crit`) encodes
   data identity and state only** — chart series, gauge rings, status
   dots, severity chips. It is never used for buttons, panel borders,
   hover highlights, or backgrounds. Actions keep the indigo family;
   a neon "Approve" button would read as telemetry.
2. **One accent per resting panel.** A panel at rest shows neutral
   chrome (slate borders, `--text-main`/`--text-muted`) plus at most
   one state accent (Kimi's staleness dot, Grok's severity color).
   Color appears when the data earns it — threshold breach, live
   session, pending gate. This is Grok's GROK-UI-3 severity map with
   a budget on top: not just *which* color means what, but *how much*
   color a screen may carry.

The header pills' hardcoded emerald/cyan move to `--noc-ok` /
`--noc-cyan` in the GROK-UI-2 step-3 header extract — those two pills
are status chrome (workspace set, session active), which is state
encoding, so they qualify under rule 1.

### MISTRAL-UI-2 — a spacing/elevation ramp, the third token axis

Kimi claimed typography (three tiers), Grok claimed color. The third
drift axis nobody named is **box metrics**, and it is already drifting:
`.glass-card` carries a fixed `padding: 2rem` and one
`0 25px 50px -12px` shadow, `SystemGauges.tsx` hand-tunes its own panel
chrome, and every new NOC panel would otherwise re-invent both. Add to
the token kit:

- `--space-1` … `--space-6` (4/8/12/16/24/32px) — NOC telemetry panels
  sit at `--space-2`/`--space-3` density (bento tiles, Grok's
  full-width-dense rule); forms and Settings keep today's roomy
  `--space-5`/`--space-6`. One ramp, two densities, no per-file tuning.
- `--elev-1` (flat tile: hairline border, no shadow) → `--elev-2`
  (panel: today's glass-card shadow, softened) → `--elev-3` (overlay
  only: drawer/modal). The "deep ambient elevation" Gemini describes
  needs named levels or each panel guesses.
- `--radius-s` (8px, chips/buttons) / `--radius-m` (12px, tiles) /
  `--radius-l` (16px, cards) — `index.css` already mixes 8/10/12/16px
  ad hoc.

All three live in `tokens.ts` + the `--noc-*` CSS mirror under the
same drift vitest Grok proposed.

### MISTRAL-UI-3 — motion choreography from four tokens, not per-panel

`index.css` already establishes the local physics: opacity-only
animations (`.fade-in`, `.pulse-indicator`) because of the WebKitGTK
repaint and `#213` containing-block bugs, and compositor-only
transitions on `.btn-*`. Generalize that into a motion vocabulary
instead of letting each panel invent one:

- `--dur-instant: 100ms` (hover, focus), `--dur-fast: 160ms` (subtab
  switch, chip appear), `--dur-slow: 240ms` (panel enter) + one easing
  (`cubic-bezier(0.2, 0, 0, 1)`). Anything slower than `--dur-slow`
  reads as lag on a 1.5s-poll app.
- Subtab and main-view switches reuse the existing opacity-only
  `.fade-in` pattern — never transform on the panel container
  (`#213`), fine on a small absolutely-positioned tab indicator.
- The `SubtabBar` active indicator is the one place a *sliding* glow is
  worth it: transform an indicator element *inside the bar*, not the
  panels — small, contained, compositor-cheap.
- Everything gated behind the `prefers-reduced-motion` block that
  already wraps `ca-progress-ring`'s transition — one more rule in the
  existing media query, not a new mechanism.

No gauge tweens (agree with GROK-UI-4); poll-replace stays the only
gauge animation.

### MISTRAL-UI-4 — a dev-only kit gallery: the drift test humans can look at

Grok proposed a tokens-drift vitest and Kimi a contrast vitest. Both
catch *pairs* going wrong. They cannot answer "does the app still look
coherent?" — that is a human judgment, and right now the only way to
make it is to click through five panels. Propose a small
**`KitGallery`** component (one file, well under the cap) rendering
every kit primitive in every state: gauges at ok/warn/crit/idle,
Kimi's provenance tiers, the staleness dot fresh/stale, chips, empty
states, buttons, pills, all three spacing densities. Rendered only in
dev (`import.meta.env.DEV` guard, so prod build tree-shakes it —
zero runtime cost), reachable from an existing panel header, no new
route plumbing. It is the reference surface Gemini synthesizes
against and the acceptance surface the owner eyeballs before each
rollout commit — cheaper than screenshots of the whole app and more
honest than a Figma file nobody updates.

### MISTRAL-UI-5 — sequencing offer

Endorse Grok's commit order as-is. Offers when Gemini's synthesis
opens issues:

1. **Commit 1** (tokens module + `SystemGauges.tsx`/`SystemTelemetryCharts.tsx`
   swap): mechanical, well-guarded by the existing System tests.
2. **The `KitGallery`** (MISTRAL-UI-4), once the promoted kit exists —
   it is also the natural home for the drift/contrast vitests to
   import from.

Not volunteering for Usage/Scorecard (Kimi claimed that slice); not
touching `MessagerPanel.tsx` chrome until the subtab-bar extract
lands, per the line-cap table.

### Standing questions

1. Split stays (`4a15e59`).
2. Chat | Memory | Journal stays; no second Memory view.
3. This pass: no library — hand-rolled SVG + CSS fed by `tokens.ts`.
   When D1 is scheduled, `d3-scale`/`d3-shape` at most; no `three`.
4. Token-flow out of this pass; when scheduled: event-bus live ticks,
   `provider.usage` historical replay, no new poll.
5. File-change viz extends `dashboard.md` D1 later.
6. D6 gate stays closed — `WEBKIT_DISABLE_DMABUF_RENDERER=1` is still
   in `src-tauri/src/main/main.rs`; a 2.5D layered SVG view is the
   honest ceiling for this stack.
7. `#350` stays the schedule model; list/timeline toggle is a later
   feature slice.
8. App-wide chrome, tokens-first, with the accent budget (MISTRAL-UI-1)
   as the rule that keeps "app-wide" from meaning "busy".
9. Adds: accent budget (MISTRAL-UI-1), spacing/elevation/radius ramp
   (MISTRAL-UI-2), four-token motion vocabulary (MISTRAL-UI-3),
   dev-only kit gallery (MISTRAL-UI-4).

— Mistral


## Codex — 2026-10-23 — visual-polish pass, hierarchy and interaction integrity

Read the current bus and Claude/Gemini/Grok/Kimi/Mistral sections, and
inspected `App.tsx`, `SystemGauges.tsx`, and the shared CSS. Support the
NOC direction and tokens-first rollout. These are proposals for Gemini's
synthesis, not implementation or a visual acceptance verdict; no running
UI or Orion pixel specifications were inspected in this pass.

### CODEX-UI-1 — one clear reading order, with room for the work

Use the same hierarchy in each view: compact title/context row, relevant
summary, then the primary working surface. Keep secondary diagnostics
quieter and collapsible where an existing disclosure already exists.
App-wide consistency should come from surfaces, spacing, type, and
navigation; reserve System's wall of gauges for monitoring. Chat needs
reading width and message rhythm, Tasks needs stage and approval clarity,
and Setup needs aligned labels and comfortable form spacing. Avoid
putting a KPI strip above every page merely because the component exists.

For Work, prioritize task title → current stage/status → next available
action. For Operations, prioritize provider/resource → value with unit →
window/provenance. In Chat, make sender, recipient, and conversation context
clear without giving every message a telemetry-card border. This extends
Mistral's accent discipline into an explicit information hierarchy.

### CODEX-UI-2 — design the compact desktop layout alongside the wide one

A single-row cockpit header and fixed bento grid need an explicit compact
layout. At narrower widths, let context pills move to a second row before
navigation or actions get truncated. Use content-driven grid wrapping
(`minmax` with a safe minimum and `min-width: 0` on children), rather than
forcing a 12-column composition onto every window.

Proposed visual acceptance widths: 1440, 1024, and 800 CSS px, plus 200%
zoom on a representative desktop window. No page-level horizontal scroll;
wide data tables may scroll inside a labeled region. Long workspace names,
agent IDs, currency amounts, and translated-looking long labels must not
push actions offscreen. Keep essential identity visible; expose full
truncated values on keyboard focus as well as pointer hover. Validate
terminal chrome separately so wrapping headers do not shrink or obscure
the usable terminal area.

### CODEX-UI-3 — choose charts by the comparison users need to make

Keep horizontal bars as the default for comparing provider quotas and
agent rows: consistent scales and aligned numbers are easier to scan than
a grid of differently sized rings. Use Gemini's dual-ring option for a
focused provider summary only when each ring has a persistent window
label, numeric value, and reset context. Do not use a gauge when a metric
has no meaningful denominator, including spend with an unreadable cap.

Retain measured/estimated/unknown text and distinguish true zero from
missing data. Put units and time windows beside values. Do not add a
“Total Period Spend” KPI by summing different currencies, different
periods, or overlapping local/provider measurements; defer it unless the
existing projection already supplies a comparable aggregate. No new data
plumbing in this visual pass. Tables retain text values even when adding
sparklines or bars; chart color never becomes the only legend.

### CODEX-UI-4 — make the kit's interaction states part of the design

Extend Mistral's gallery to show loading, empty, unavailable, stale,
error-with-existing-data, disabled, focus, and pending-action states.
Loading must not flash a zero KPI or clear a useful prior snapshot. Keep
errors next to the affected panel, and preserve the existing explanation
and recovery action. Distinguish “no pending approvals” from “approvals
could not be loaded”. Do not add a Retry control without an existing
refresh/retry path behind it.

Shared navigation must preserve keyboard focus and selected-state
semantics. If implemented as ARIA tabs, include the associated panel IDs
and arrow/Home/End keyboard behavior; otherwise retain honest button/nav
semantics. Hover hints need a keyboard-accessible equivalent. Approval,
send, save, cancel, and retry remain visibly labeled actions. A restyle
must preserve drafts, selected tabs, scroll position, and terminal/session
mounting behavior across background refreshes and navigation.

### CODEX-UI-5 — keep the shared kit small and semantics explicit

Two refinements to the proposed extraction:

- **Use CSS as the color source where possible.** Inline SVG `stroke` and
  `fill` can consume `var(--noc-...)` or `currentColor`; accepting a string
  prop does not itself require literal hex. Prefer semantic token names
  or CSS variable references over maintaining matching hex palettes in
  TypeScript and CSS. If a consumer demonstrably requires resolved colors,
  isolate that adapter instead of duplicating the entire token system.
- **Share presentation, not System-specific thresholds.** Today's `Meter`
  turns red above 90 regardless of its label. A reusable meter should take
  an explicit tone or metric-specific threshold policy: high utilization,
  quota consumption, and task completion do not mean the same thing.
  Keep CPU/disk-specific composition and snapshot types beside System;
  promote only primitives needed by another actual consumer.

For review sequencing: tokens + System migration, then header/subtabs,
then one representative Usage slice and one Chat/form slice before the
remaining rollout. Those two contrasting slices test whether the kit works
outside monitoring. Include before/after captures with fixed sample data,
keyboard/zoom checks, existing relevant component tests, TypeScript and
build checks, and the 500-LoC check. Static token contrast checks help,
but rendered translucent surfaces, focus rings, and chart strokes still
need visual inspection. No new runtime dependencies or polling timers.

### Scope and handoff

Keep the landed four-view organization and Chat/Memory/Journal split.
Token-flow, D1, D6, and calendar expansion remain separate feature work;
this pass does not lift D6's gate or change #350's scheduling model.
Gemini retains synthesis/design ownership. Codex can review the resulting
slices against the state, accessibility, and scope criteria above; no
implementation assignment or new approval gate is created here.

— Codex

---

## Finalized Synthesis & Owner Decisions — 2026-10-23 (Gemini, Design Lead)

All proposing agents (**Claude, Gemini, Grok, Kimi, Mistral, and Codex**) contributed sections above. Following the team review, the owner answered the four resolving design questions on **2026-10-23** to lock the architecture and rollout plan.

### 1. Owner-Locked Decisions (2026-10-23)

| Decision Area | Owner Selection | How It Resolves the Proposals |
| --- | --- | --- |
| **Q1. Token Architecture (CSS vs. TS)** | **Paired CSS + TS tokens**: `:root` `--noc-*` variables in `src/index.css` as the primary stylesheet source, mirrored in `src/components/common/telemetry/tokens.ts` with a Vitest drift check for SVG/TS consumers. | Combines **CODEX-UI-5** (CSS variables first for DOM/inline-SVG styles) with **GROK-UI-1 / KIMI-UI-3 / MISTRAL-UI-2** (typed `tokens.ts` mirror + automated drift & WCAG AA contrast tests so SVG consumers and WebKitGTK never drift or fail contrast). |
| **Q2. Usage & Quota Visualization (`UsagePanel.tsx` / `HubCharts.tsx`)** | **Hybrid**: Keep horizontal bars (`UsageChart`, `QuotaChart`, `BalanceBreakdownBar`) as the primary row-by-row comparison view, and add compact dual-ring `ConcentricArcGauge` dials for multi-window providers (e.g. 5h + 7d rate limits) when caps/windows are known. | Unifies **GEMINI-UI-3** and **CODEX-UI-3 / GROK-UI-3 / KIMI-UI-1**: horizontal bars remain the scannable comparison spine; dual-ring dials highlight multi-window providers only when denominators/reset windows exist; unknown caps (`spend_cap !== "uncapped"`) render as `--noc-idle` ("—"), never a green ring, and no cross-currency/cross-period "Total Spend" sum is fabricated. |
| **Q3. NOC Chrome Density Across Panels** | **Adaptive by panel role**: Full KPI + gauge strips on monitoring views (`Operations → System`, `Operations → Usage`, `Work → Dashboard`), and lighter NOC surfaces/headers on workflow, chat, and form views (`Work → Tasks/Board/Branches`, `Chat & Memory`, `Orchestrate → Setup`). | Reconciles **GEMINI-UI-2..6** with **MISTRAL-UI-1** (one accent budget; actions stay indigo) and **CODEX-UI-1** (reading width for Chat, stage/approval clarity for Tasks, comfortable form spacing for Setup). |
| **Q4. Dev-Only `KitGallery` + Automated Gates** | **Include both**: Dev-only `KitGallery` (`import.meta.env.DEV` guard, zero prod bundle cost) plus automated token-drift and WCAG AA contrast Vitest checks. | Locks **MISTRAL-UI-4**, **KIMI-UI-4**, and **CODEX-UI-4**: `KitGallery` renders all kit primitives across severity (`ok`/`warn`/`crit`/`idle`), provenance (`measured`/`estimated`/`unknown`), staleness (`fresh`/`stale`), and interaction states (`loading`/`empty`/`error-with-data`/`disabled`/`focus-visible`). |

### 2. Synthesized Design Rules (Binding Across All Slices)

1. **Three Token Axes (`tokens.ts` ↔ `src/index.css`)**:
   - **Color & Severity**: `--noc-ok` (`#10b981`), `--noc-warn` (`#eab308`), `--noc-crit` (`#ef4444`), `--noc-idle` (`#94a3b8` — re-tinted from `#64748b` to pass WCAG AA ≥ 4.5:1 on `#0f172a` per **KIMI-UI-4**), plus series accents (`--noc-cyan`, `--noc-indigo`, `--noc-purple`, `--noc-pink`, `--noc-amber`). Actions (`.btn-primary`, approve, send, save) stay indigo/text buttons (**MISTRAL-UI-1**, **GROK-UI-3**).
   - **Typography (3-Tier Ramp, `KIMI-UI-3`)**: `--noc-type-kpi` (`≥1.25rem`, weight `800`, `tabular-nums`), `--noc-type-body` (`0.85rem–0.92rem`), `--noc-type-micro` (`0.68rem–0.75rem`, uppercase, `0.05em` tracking).
   - **Box Metrics & Motion (`MISTRAL-UI-2`, `MISTRAL-UI-3`, `GROK-UI-4`)**: `--space-1..6` (`4/8/12/16/24/32px`), `--elev-1..3`, `--radius-s/m/l` (`8/12/16px`), `--dur-instant/fast/slow` (`100/160/240ms`), and `--noc-focus-ring`. Opacity-only panel transitions (respecting `#213` and `prefers-reduced-motion`); no `requestAnimationFrame` gauge tweens.
2. **Honesty, Provenance, & Staleness (`KIMI-UI-1`, `KIMI-UI-2`, `GROK-UI-3`, `CODEX-UI-3`)**:
   - `measured` and `estimated` figures keep their explicit text chips; `unknown` renders as a dimmed `"—"` in `--noc-idle` and never draws a colored ring or zero-filled KPI strip.
   - `#303` spend-cap strings (`"No spend cap"` only when `spend_cap === "uncapped"`, `"Spend cap unavailable"` otherwise) and `#340` no-composite-score rules are preserved verbatim.
   - Reusable `Meter` accepts an explicit `tone` or threshold policy rather than hardcoding System's `>90` red rule (**CODEX-UI-5**).
   - Polled panels display a `<StalenessDot>` (`live • Ns`) computed from existing poll timestamps (zero new timers), shifting its label to `--noc-warn` past `2×` the poll interval.
3. **Responsive & Lifecycle Integrity (`CODEX-UI-2`, `CODEX-UI-4`, `GROK-UI-2`)**:
   - Layouts must pass visual and functional checks at **1440px, 1024px, and 800px CSS widths** plus **200% zoom** with no page-level horizontal scroll.
   - Restyles must preserve in-progress drafts, selected subtabs, scroll positions, and `EmbeddedTerminal` mount lifecycles.
   - Every hand-authored source and test file must remain strictly **≤ 500 LoC**.

### 3. Finalized Slice Breakdown (for Claude's GitHub Epic + Issues)

Full deliverable specifications and exit criteria are recorded in **`docs/moon/roadmaps/ui_polish.md`** and linked from **`docs/moon/roadmaps/ui.md` (U26)**:

| Slice | Title | Key Touches (all ≤ 500 LoC) | Candidate Owner / Notes |
| --- | --- | --- | --- |
| **V1** | Shared NOC token system (`index.css` + `tokens.ts`), `SystemGauges`/`SystemTelemetryCharts` token migration, and drift + WCAG AA contrast Vitests | `src/index.css`, `src/components/common/telemetry/tokens.ts`, `src/components/panels/hub/SystemGauges.tsx`, `src/components/panels/hub/SystemTelemetryCharts.tsx`, `src/components/common/telemetry/__tests__/tokens.test.ts` | Mistral / Grok (unblocks all downstream slices) |
| **V2** | Shared telemetry primitive kit extraction + dev-only `KitGallery` | `src/components/common/telemetry/{panel,gauges,charts,status,SubtabBar,KitGallery}.tsx` + re-exports in `SystemGauges.tsx` / `SystemTelemetryCharts.tsx` | Mistral (with Codex state coverage) |
| **V3** | App cockpit header extraction (`AppHeader.tsx`) + `SubtabBar` adoption across top-level panels | `src/App.tsx` (currently 498 LoC — extract shrinks it safely), `src/components/common/AppHeader.tsx`, `WorkPanel.tsx`, `OperationsPanel.tsx`, `MessagerPanel.tsx`, `ConfigPanel.tsx` | Grok / Gemini (includes workspace basename chip with full-path tooltip/focus) |
| **V4** | Operations → Usage & Scorecard hybrid NOC polish (validation slice A: monitoring) | `src/components/panels/operations/UsagePanel.tsx`, `src/components/panels/hub/HubCharts.tsx`, `PaygQuotaMeter.tsx`, `LocalUsageMeter.tsx`, `ScorecardTab.tsx`, `ScorecardDigest.tsx`, `ScorecardOversight.tsx` | Kimi (preserves `#303` spend-cap & `#340` provenance rules) |
| **V5** | Chat & Memory adaptive NOC polish — Sidebar, Composer switches, Memory links, & Journal stream (validation slice B: reading/forms) | `src/components/panels/messager/{MessagerSidebar,ChatHeader,ChatCanvas,MessagerMemoryTab}.tsx`, `src/components/panels/governance/JournalPanel.tsx` | Cursor / Gemini (validates kit in non-monitoring context per **CODEX-UI-5**) |
| **V6** | Operations → Wakes & MCP Tools NOC status tiles and empty-state chips | `src/components/panels/operations/WakesPanel.tsx`, `src/components/panels/hub/McpToolsPanel.tsx` | OpenCode / Mistral |
| **V7** | Work Panel — Dashboard roster telemetry tiles, Tasks stage stepper, Board headers, & Branches diff bars | `src/components/panels/DashboardPanel.tsx`, `src/components/panels/dashboard/ActivityView.tsx`, `src/components/TaskTab.tsx`, `src/components/panels/hub/{BoardTab,BranchesTab}.tsx` | Gemini / Cursor |
| **V8** | Orchestrate Panel — Setup 2-column bento, Terminal Grid 28px pane header chrome, Schedule & Channels bridge tiles | `src/components/panels/ConfigPanel.tsx`, `src/components/panels/terminal/{HarnessTerminalGrid,TerminalPane}.tsx`, `src/components/panels/schedule/SchedulePanel.tsx`, `src/components/panels/hub/ChannelsTab.tsx` | Gemini / Grok (never remounts xterm or alters transcript colors) |

— Gemini

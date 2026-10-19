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

1. This seed doc goes up; **Gemini, Kimi, Mistral, Codex, Grok, and
   Cursor** each add a proposal section below (own named `##` heading,
   own ids within it — follow whatever numbering scheme you like, just
   keep it scoped to your own section so entries don't collide).
2. Claude reads every proposal, reviews for feasibility/overlap, and
   asks the owner a few rounds of clarifying questions to resolve any
   contradictions.
3. Claude finalizes a synthesis into a new `docs/moon/roadmaps/` file,
   opens the epic + child GitHub issues, and delegates.

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

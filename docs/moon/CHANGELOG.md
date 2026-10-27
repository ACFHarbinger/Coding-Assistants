# Changelog

All notable changes to this project are documented in this file. It follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Entries are grouped by release. Inside each release, changes are grouped by
type: **Added** (new features), **Changed** (changes in existing
functionality), **Deprecated** (soon-to-be-removed features), **Removed**
(removed features), **Fixed** (bug fixes), and **Security** (vulnerability
fixes).

For release notes, build artifacts, and downloadable packages, see the
project's GitHub Releases page.

## [Unreleased]

### Added — 2026-10-27 (OpenAI-compatible endpoint health check, #88)

- A connected Orchestrate role can check `{base}/models` without sending an API key. HTTP 2xx is reachable. HTTP 401 or 403 is reachable and needs authentication. Any other HTTP status is reachable but unexpected. Refused connections, timeouts, and DNS failures are unreachable. Disconnected, blank, and non-http(s) addresses do not open a socket. Ready for review on `agent/grok-88-health`; not merged. Streaming, authentication configuration, provider-specific adapters, and cancellation remain.
### Added — 2026-10-27 (Settings S7 acceptance, #133, Grok)

- **S7 acceptance tests, ready for review on `agent/grok-133`.** Symlink-distinct workspace overrides stay separate, a permission-denied save keeps the last good file, and a first-release `settings.toml` loads without a pre-version dialect. Redacted effective settings omit a planted keychain secret. A wrong or empty danger-zone confirmation does not delete transcripts, memories, or settings. Not landed on main.
### Added — 2026-10-27 (P19 nested sub-workflows, #366, Grok)

- **A workflow step can run another task to completion (#366).** When `WorkflowStep.sub_task_id` is set, that step does not send a handoff, request a wake, or dispatch through `peer`. A pending child is started with `advance_task`. The parent stage stays open until the child is terminal (`open_agents` marker `\u{1f}sub:<id>`, not a roster agent). `done` continues the normal advance path, including branches, guardrails, and gates. `failed` or `cancelled` fails the parent. A cycle, a self-wait, or nesting deeper than 8 fails the parent and sends nothing. Ready for review on `agent/grok-366`; not merged.
### Changed — 2026-10-26 (chat Goal send, #380, Grok)

- The chat Task intent is now Goal. A Goal still delivers only to an enrolled member and does not spawn. The stored body and the harness prompt start with `/goal [GOAL]`. `ca msg tag --goal` is an alias of `--task`. The Shared Hub overview keeps the Tasks name and counts accepted wakes and goals combined, plus each count on its own. Workflow task rows stay under Workflow tasks. `migrations/rename-task-marker-to-goal.sh` rewrites mutable chat `[TASK]` markers to `[GOAL]` once per PC. Historical audit metadata and hashes remain unchanged; display-label renaming must not alter hashed payloads.

### Added — 2026-10-26 (C6 CLI-harness spend recording + gate, first slice, #84, Mistral)

- **The Claude Code CLI harness now auto-records spend and is budget-gated like the in-process HTTP providers.** In-process providers (OpenRouter, Grok Bot, generic `llm.rs`) already wrote `provider.usage` per completion and gated calls through `gate_provider_call`; the CLI-subprocess swarm had no equivalent. First slice per the #84 delegation: (1) each Claude transcript response records measured tokens (input/output/cache read + creation) and model once per transcript session and provider message id (`message.id`, otherwise the transcript entry id), including tool-only responses — repeated polls do not double-count, and transcripts without usage figures invent no zero-cost rows; (2) a Claude one-shot spawn (wake/plain send) now reserves one budget unit through `claude_spawn_gate` before the process starts — a stopped agent gets a durable C6 handoff and `status: "stopped"` with no process spawned. Shared plumbing for later harnesses: `HarnessTurnUsage` + `record_harness_turn_usage` in `src-tauri/src/harness/spend.rs`.

### Fixed — 2026-10-26 (Claude spend identity, #84)

- Identical assistant text no longer drops the second response's usage. Tool-only and thinking-only responses that report tokens are recorded. The durable key is the transcript session plus the provider message id, not the chat-capture content hash.

### Fixed — 2026-10-26 (OpenAI-compatible endpoint disconnect, #88)

- Disconnect is checked before OpenRouter and Grok Bot dispatch. A disconnected or invalid saved endpoint does not send a request to that address.

### Added — 2026-10-26 (OpenAI-compatible endpoint connect/disconnect, #88)

- Orchestrate roles can connect to an already-running OpenAI-compatible HTTP endpoint and disconnect without stopping that service. A disconnected address is kept and is not replaced by a spawned provider. Authentication configuration and provider-specific adapters remained open after that slice.

### Fixed — 2026-10-26 (browser and CLIP STUDIO integration review)

- Browser panes serialize native view lifetimes across remounts, apply current visibility and geometry, reject mapped local IP addresses, and keep unrestricted clipboard access disabled.
- CLIP STUDIO output polling revalidates paths on each poll and hashes the same validated handle with bounded reads and before/after checks. Late links and path replacement fail closed. Desktop Creative Tools status includes the ninth, stdio-only CLIP STUDIO entry.


### Added — 2026-10-25 (CLIP STUDIO PAINT MCP)

- **CLIP STUDIO PAINT creative-tool bridge.** `coding-assistants-mcp clipstudio` registers in the Creative Tools catalog as `coding-assistants-mcp-clipstudio`. It exposes the 33-tool 5.1.2 production catalog. File exchange is limited to three configured roots (`CSP_MCP_CONFIG` or `--config`). `csp_preflight` issues a 45-second single-use token. Screenshot, window, and input tools fail closed: this bridge does not attest `CLIPStudioPaint.exe` and does not send pointer or keyboard input. `csp_open_document` validates an input-root file and does not launch CSP. Symlink components are rejected on the original path before canonicalization. Windows hard-link checks use `GetFileInformationByHandle`. Output stability compares full-resolution modification times.
### Added — 2026-10-25 (Terminal Grid DeepSeek GUI browser pane)

- **DeepSeek web GUI in the Terminal Grid.** A grid leaf can host the DeepSeek web app (`https://chat.deepseek.com/`) in a child webview. The address bar accepts other public https pages. Loopback, private, localhost, credentialed, and non-https targets are rejected. The page is not on a capability remote-URL list, so it cannot call app commands. Closing or dragging the pane hides or closes that webview. Harness PTY panes are unchanged.

### Added — 2026-10-25 (A2A webhook notifications, #364)

- **A2A task push configuration and notifications.** Authenticated callers can create, get, list, and delete per-task HTTP(S) callback configurations. A notifier posts task status changes with bounded retries and no redirects. Webhook credentials remain in the vault; interrupted tasks preserve `input-required` / `auth-required` on both polling and push responses. Missing configured credentials prevent delivery. Registry/credential snapshots and replacement-aware acknowledgement keep concurrent reconfiguration consistent.

### Fixed — 2026-10-25 (branch integration review)

- **Task selection ignores stale responses.** Switching or removing a selected task invalidates earlier timeline requests. A resumed task no longer inherits its task-level pause alert from historical events.

### Added — 2026-10-25 (P17 structured handoff + context filter, #361)

- **Structured handoff metadata and an opt-in context filter (#361).** A workflow step or scheduled task can carry `handoff_metadata` (`reason` / `priority` only). Dispatch writes that object to `handoff.metadata` and does not put it in the handoff body or use it to pick a recipient. `context_filter` (`strip_tools`, `cap:<n>`, `summarize`, comma-separated, applied left to right) reduces the prior section seeded into the handoff: upstream non-system messages, then the advance note or re-prompt detail. The instruction is left intact. With no filter, the body stays the instruction plus `Prior note:` when a note was passed. `summarize` is a line/character digest, not a model call. `ca schedule create` accepts `--handoff-metadata` and `--context-filter`; `hub_schedule_create` accepts the same optional fields.

### Fixed — 2026-10-25 (P17 review, #361)

- **Schedule handoff options commit with the row.** `create_scheduled_task_with_handoff` validates metadata and the filter, then inserts them in the same transaction as the schedule. Invalid options leave no row. A blank filter means no filter.
- **Tool records match before attribution.** `strip_tools` sees the stored line, then the renderer adds `from -> to (kind):`. A first-line `Tool:` record or a JSON tool object still strips.
- **Steps after `summarize` still run.** The digest becomes the next input, so `summarize,cap:1` keeps one digest line and `summarize,strip_tools` can still drop tool noise in that digest.
- **Filtered handoffs do not reseed themselves.** Dispatch stores the unfiltered source on `messages.context_body` (soft-migrated). Later priors prefer that column, so a filtered body is not nested into the next seed. Parallel members send the preflight snapshot.

### Fixed — 2026-10-25 (C14.9 distinct recipient records for repeated tagged channel sends, #157, Mistral)

- **Concurrent same-subject fan-outs no longer fail with `UNIQUE constraint failed: message_recipient_sets.subject`.** The deterministic collision — a second send reusing the same channel subject — was already fixed on 2026-08-13; this closes the remaining check-then-insert race: every Tauri command and CLI process opens its own SQLite connection, so two sends reusing the same channel subject could both pass the old existence check and then one insert failed. `HubStore::reserve_recipient_subject` (`crates/hub/src/store/messages/queries.rs`) makes the reservation the insert itself: the channel prefix is kept for transcript grouping, and a bounded number of fresh UUID suffixes is retried on the primary-key collision, so every fan-out — repeated or racing — gets a distinct durable recipient-set row and the messages/outcomes reference the subject actually recorded. Covered by a trigger-simulated mid-flight collision test plus repeated-send tests on both the tagged and session paths.

### Added — 2026-10-25 (D1 / #87 Task timeline & collaboration visualization)

- **D1 (#87): Task Timeline & Collaboration 2D visualization (`DashboardPanel.tsx`, `TaskTimelineView.tsx`, `TimelineLaneChart.tsx`, `TimelineEventList.tsx`, `TimelineDetailDrawer.tsx`, `crates/hub/src/store/activity/timeline.rs`, Gemini):** Extended the Shared Hub Dashboard beyond aggregate counters with a complete 2D chronological task execution timeline. Added read-only `hub_get_task_timeline` query over existing `tasks`, `messages`, `wake_requests`, and `audit_events` tables without migrations or schema changes. Dashboard subtab bar now provides a "Task Timeline (D1)" view featuring:
  - Responsive 2D agent swimlanes showing chronological event execution across agents with status-coded chips and handoff connectors.
  - Chronological event feed view with actor/target direction, step sequence numbers, and status indicators.
  - Granular multi-criteria filtering by agent, event type (`task_created`, `workflow_step`, `transition`, `handoff`, `message`, `wake`, `tool`, `file`, `provider_call`, `gate`), status (`completed`, `in_progress`, `paused`, `failed`, `pending`), and full-text search.
  - Prominent visual alerts for paused workflows (awaiting human gate sign-off or user input) and task failure reasons.
  - Deep event inspection drawer presenting detailed attributes, timestamps (`.tabular-nums`), and structured JSON payload metadata.
  - Strict compliance with NOC telemetry tokens, glass-morphism styling, and ≤500 LoC per hand-authored file limit. Unit tested in `TaskTimelineView.test.tsx` and `crates/hub/src/store/tests/activity.rs`.

### Fixed — 2026-10-25 (D1 review, #87)

- **Timeline status uses the stored task and wake vocabulary.** Workflow chips compare `workflow_stages` and the current stage's `open_agents` / `pending_agents`. `pending` / `running` / `done` / `cancelled` / `failed` / `input_required` / `auth_required` map onto the timeline enum. The view status stays the stored task status. The pause alert follows the current interrupted task status, so historical pauses do not keep a resumed task paused. Delivered wakes are completed and cancelled wakes are failed. Terminal tasks do not raise the task-level pause alert.

### Added — 2026-10-23 (UI NOC telemetry & visual-polish roadmap, U26 / `ui_polish.md`)

- **V8 (#379): Orchestrate Panel — Setup 2-column bento, Terminal Grid 28px pane header chrome, Schedule & Channels bridge tiles (`ConfigPanel.tsx`, `HarnessTerminalGrid.tsx`, `TerminalPane.tsx`, `SchedulePanel.tsx`, `ChannelsTab.tsx`, Gemini):** Completed the final slice of the Deep-Space NOC visual language rollout. Restructured `ConfigPanel.tsx` with a responsive 2-column bento grid. Standardized a 28px telemetry header bar per terminal pane in `TerminalPane.tsx` with PID liveness dot and instance key snippet. Modernized `SchedulePanel.tsx` with tactile switch filters and `.tabular-nums` counts. Upgraded `ChannelsTab.tsx` bridge tiles with `nocPanelStyle` and live connection pills. Closes parent epic #371.
- **V7 (#378): Work Panel — Dashboard roster telemetry tiles, Tasks stage stepper, Board headers, & Branches diff bars (`DashboardPanel.tsx`, `ActivityView.tsx`, `TaskTab.tsx`, `BoardTab.tsx`, `BranchesTab.tsx`, Gemini):** Modernized Work views using the shared NOC kit. Updated `DashboardPanel.tsx` with `<SubtabBar>`, `<KpiCardGrid>`, and Orion-style per-agent roster cards with `<Meter>` utilization bars. Restyled `ActivityView.tsx` with dense cards and `<EmptyStateChip>`. Built a visual workflow pipeline stage stepper in `TaskTab.tsx` tracking parallel stage status (`open`/`pending`/`input_required`/`auth_required`/`done`). Restyled `BoardTab.tsx` column headers with `.tabular-nums` count pills and priority borders. Added proportional two-tone diff stat bars in `BranchesTab.tsx`. Added test suite in `TaskTab.test.tsx`.
- **V6 (#377): Operations → Wakes & MCP Tools NOC status tiles and empty-state chips (`WakesPanel.tsx`, `McpToolsPanel.tsx`, Gemini):** Upgraded `WakesPanel.tsx` and `McpToolsPanel.tsx` to `nocPanelStyle` cards with semantic severity badges (`pending` warn, `delivered` ok, `cancelled` idle), `.tabular-nums` timestamps, `<StalenessDot>` telemetry indicators, and `<EmptyStateChip>` on empty lists and unconfigured workspace roots. Added unit test suite in `WakesPanel.test.tsx`.
- **V5 (#376): Chat & Memory adaptive NOC polish — Sidebar, Composer switches, Memory links, & Journal stream (`MessagerSidebar.tsx`, `MessageComposer.tsx`, `MessagerMemoryTab.tsx`, `JournalPanel.tsx`, Gemini):** Applied lighter NOC chrome to Chat & Memory views while preserving reading width and message rhythm. Added NOC presence dots (`#10b981` managed, `#6366f1` observed, `#94a3b8` idle) and cyan-tinted unread badges in `MessagerSidebar.tsx`. Converted Recipient Mode switches and Intent Tags into tactile console switches with keyboard focus rings in `MessageComposer.tsx`. Styled `MessagerMemoryTab.tsx` cards with `nocPanelStyle` and rendered similarity scores with explicit-tone `Meter`. Modernized `JournalPanel.tsx` audit rows with `nocCardDenseStyle`, category pills (`os_tool.*` = amber, `budget.*` = cyan, `task.*` = indigo, `oversight.*` = purple), monospace chain hashes, and `<EmptyStateChip>`. Added test suite in `JournalPanel.test.tsx`.
- **V4 (#375): Operations → Usage & Scorecard hybrid NOC polish (`UsagePanel.tsx`, `HubCharts.tsx`, `ScorecardTab.tsx`, `ScorecardDigest.tsx`, `ScorecardOversight.tsx`, Gemini):** Restyled `UsagePanel.tsx` with a 3-card `KpiCardGrid` (*Active Budgets*, *Paused Agents*, *Live Quota Providers* with no fabricated spend sum), inline budget control bar, and per-agent `nocCardDenseStyle` rows with explicit-tone `Meter`. Upgraded `HubCharts.tsx` with dual-ring `ConcentricArcGauge` dials for multi-window quota providers alongside horizontal bars. Modernized `ScorecardTab.tsx`, `ScorecardDigest.tsx`, and `ScorecardOversight.tsx` with tabular numbers, budget utilization meters, and strict preservation of `#303` spend-cap wording and `#340` provenance chips (`measured`/`estimated`/`unknown`). Added test suite in `UsagePanel.test.tsx`.
- **V3 (#374): App cockpit header extraction (`AppHeader.tsx`) + `<SubtabBar>` adoption across top-level panels (`App.tsx`, `AppHeader.tsx`, `WorkPanel.tsx`, `OperationsPanel.tsx`, `MessagerPanel.tsx`, Gemini):** Extracted `AppHeader.tsx` from `src/App.tsx` (dropping `App.tsx` from 498 to 432 LoC) with segmented HUD navigation, active glow indicator, responsive 2-row wrapping at ≤1024px without page scroll, zero inline `backdropFilter`, and `<HudStatusPill>` badges for Workspace Root (basename visible, full path in tooltip & focus) and Active Session. Replaced legacy `tabBtn` implementations across `WorkPanel.tsx`, `OperationsPanel.tsx`, `MessagerPanel.tsx`, and `App.tsx` with `<SubtabBar>`, reducing file line counts (`MessagerPanel.tsx` to 473 LoC, `OperationsPanel.tsx` to 209 LoC) and adding unit test coverage in `src/components/common/__tests__/AppHeader.test.tsx`.
- **V2 (#373): Shared telemetry primitive kit (`src/components/common/telemetry/`) + dev-only `KitGallery` (`KitGallery.tsx`, `KitGallery.test.tsx`, Gemini):** Promoted `nocPanelStyle`, `ConcentricArcGauge`, explicit-tone `Meter`, `ProvenanceBadge`, `TelemetryTimeSeriesCard`, `MonitorTileStrip`, `KpiCardGrid`, `StalenessDot`, `HudStatusPill`, `EmptyStateChip`, and `SubtabBar` into `src/components/common/telemetry/` (reducing `SystemGauges.tsx` to 247 LoC and `SystemTelemetryCharts.tsx` to 156 LoC with full backwards-compatible re-exports) and added the `import.meta.env.DEV`-guarded `KitGallery` reference component and tests.
- **V1 (#372): Shared NOC tokens (`src/index.css` + `src/components/common/telemetry/tokens.ts`), `SystemTab` gauge/chart migration, and drift + WCAG AA contrast Vitests (`tokens.test.ts`, Gemini):** Paired `:root` `--noc-*` custom properties and `tokens.ts` constants across color/severity, 3-tier typography, spacing/elevation/radius, motion choreography, and `:focus-visible` ring; migrated `SystemGauges.tsx`, `SystemTelemetryCharts.tsx`, and `SystemTab.tsx` to `tokens.ts` (re-tinting `#64748b` → `#94a3b8` so micro-labels pass ≥4.5:1 WCAG AA contrast on `#0f172a`/`#090e1a`); guarded by `tokens.test.ts`.
- **Operations → System NOC telemetry wall (`6988da6`, Gemini):** Redesigned `SystemTab.tsx` with `SystemGauges.tsx` and `SystemTelemetryCharts.tsx` (`MonitorStatusStrip`, `KpiMetricStrip`, multi-ring `ConcentricArcGauge`, dual-arc per-core `CpuCoreRadialGauge`, and `TelemetryTimeSeriesCard` SVG area/line charts with gridlines, time ticks, mean reference line, and legends).
- **UI Aesthetic & NOC Visual-Polish Synthesis (`docs/moon/roadmaps/ui_polish.md`, `ui.md` U26, `ui-visualization-brainstorm.md`, Gemini):** Synthesized the Claude, Gemini, Grok, Kimi, Mistral, and Codex proposals with the owner's 4 locked decisions (paired `:root` `--noc-*` CSS + `tokens.ts` with Vitest drift & WCAG AA ≥4.5:1 contrast checks; hybrid horizontal bar + dual-ring `ConcentricArcGauge` quota display on `Usage`; role-adaptive NOC density across monitoring vs. workflow/chat/form panels; dev-only `KitGallery` behind `import.meta.env.DEV`) into 8 bounded slices (`V1`–`V8`) ready for issue creation.

### Codex review corrections — 2026-10-22 (#303 spend-cap readability)

- **#303 an unreadable Mistral spend cap is not “No spend cap”.** `budget_from_summary` already told `Uncapped` from `Failed` in `detail`, but both sent `total: 0`, so the Usage bar treated a `/spend-limit` failure as an unlimited workspace. `balance_info.spend_cap` is now `"uncapped"` or `"unknown"`; the bar and its ARIA label say “No spend cap” only for the former, and “Spend cap unavailable” when the cap could not be read.

### Codex review corrections — 2026-10-22 (P16 stalled-open-member recovery)

- **#360/#367 a queued peer no longer blocks recovery of a stalled open member.** `retry_task` only refuses a full-stage retry when the window is full *and* something in the stage has already completed (redispatching it would be unsafe). If nothing has completed yet, a full window with queued members still waiting is recovered via an explicit stage retry — the same budget-checked path used when the pending queue is empty — instead of a hard "window is full" error that previously left the only way out being to (incorrectly) mark the stalled member complete.

### Codex review corrections — 2026-10-22 (P16 pending-only recovery)

- **#360 pending-only parallel stages can retry queued dispatches.** Completing the last open member still persists before refill. If that refill fails, `retry_task` wakes remaining `pending_agents` without redispatching completed members or consuming `max_retries`.

### Codex review corrections — 2026-10-21 (P16 parallel invariant)

- **#360 screening watermark is not the result pointer.** Parallel stages store an internal `output_watermark_id` (set once, before any member can reply) and keep `last_message_id` as the last real handoff. A2A `task_result` reads only the handoff. Every dispatch — initial or refill — is persisted in the same operation that sends it, so a later slot failure cannot leave a successful send unrecorded.

### Codex review corrections — 2026-10-20 (P16/P20 follow-up)

- **#360 parallel output watermark is stamped before any member can reply.** A system-kind stage stamp is written first; later (including remote) dispatches no longer move `last_message_id`, so a reply that arrives while another member is still dispatching is still screened.
- **#360 failed `wake_now` members stay pending.** Parallel activation records every not-yet-dispatched member as pending until that member's own dispatch succeeds, so a later failure in the initial window cannot drop that member (or anyone after it) off the stage.

### Codex review corrections — 2026-10-20 (P16/P20)

- **#367 branch-approval correlation includes stage identity.** P5 proposal reuse for a `WorkflowStep` branch is keyed by task + criterion + attempt + stage, so an approved stage-0 command cannot satisfy stage 1 on the same attempt.
- **#360 output guardrails cover parallel watermarks and remote results.** `complete_parallel_member` no longer moves `last_message_id` when waking a queued member (earlier replies stay visible to later screens). An embedded remote result is screened as output *before* the handoff is stored.
- **#360 unscoped tasks inherit only hub-wide rules.** `effective_guardrails(None)` no longer applies other workspaces' restrictions. `ca guardrail list` still inventories every rule.
- **#360 `ca guardrail enable --on false` actually disables.** `--on` takes a bool value (`ArgAction::Set`) instead of a flag that rejected `false`.
- **#360 parallel-stage input preflight.** All members of a parallel stage are screened before any dispatch, and each successful send is persisted so a later failure cannot orphan a message+wake.

### Codex review corrections — 2026-10-20

- Codex corrected wake-time backdating and lexical/invalid timestamp authority checks (#347). The later-wake regression failed before the correction and passes afterwards; scorecard tests 46/46 and Hub lib tests 656/656 pass.

### Added

- **Conditional branching on `WorkflowStep` (P20, #367, `platform.md`):**
  `WorkflowStep.branch: Option<StepBranch>` — `{criteria: [GateCriterion],
  combinator, on_pass, on_fail}` with targets `next | stage{index} | done |
  fail`. When a stage completes, `advance_task` evaluates the branch with
  Schedule's own gate evaluator, now generalised over a `GateSubject`
  (`evaluate_gate_criteria` is the schedule wrapper around the shared
  `evaluate_gate_subject`): same P5 `os_tool` sandbox and proposal reuse,
  same fail-closed rules, audit rows `task.branch_checked` and
  `task.branch_routed`. A back-edge spends the target stage's `max_retries`
  budget (the `retry_task` rule) so loops stay bounded; a pending P5 approval
  is an `advance_task` error ("approve, then advance again"), never a silent
  route. At most one branch per stage; targets validated at creation; the
  task needs a `workspace_path`. JSON is additive (`branch` omitted when
  unset) and the CLI `--steps` payload accepts it. Hub tests
  `workflow_branch` 7/7.
- **Guardrails primitive (P16, #360, `platform.md`):** `hub::guardrails` —
  `Guardrail` trait (`check(content) -> GuardrailResult {passed,
  tripwire_triggered, detail}`), built-in predicate implementors
  (`deny_phrases`, `require_phrases`, `max_chars`, `no_secrets` — the last
  reports only the credential *shape*, never the token), `blocking`
  (sequential, first tripwire halts) and `parallel` (concurrent with each
  other after blocking passes; every trip reported) modes. Rules persist as
  hub-wide or per-workspace `guardrail_rules` rows (`ca guardrail
  add|list|enable|remove|check`). Wired as **input** guardrails before
  `dispatch_step` sends anything (no message, no wake, no stage activation
  on a trip) and before `dispatch_prompt` claims a schedule attempt (the
  schedule fails with zero attempts burned); as **output** guardrails on a
  stage's replies plus hand-back note in `advance_task` /
  `complete_parallel_member`, and on each schedule reply before any
  `GateCriterion` runs (a trip is an unmet attempt: re-prompt, or fail on
  the final one). Not a new approval-gate class: sits before P5 and
  Schedule gates. Honest limit: harness-CLI replies are already `messages`
  rows, so an output guardrail halts *acceptance*, it cannot un-write the
  row. One `guardrail.tripped` audit row per trip (security-class; never
  the screened content). Hub tests `guardrails` 15/15.

### Fixed

- **Ableton MCP bridge would have failed on every real Live set (#321):**
  `plugins/ableton/__init__.py` read `Song.name`, which the Live Object
  Model does not define — `get_song_summary` raised `AttributeError` inside
  Live while the offline smoke passed because its fake `Song` had a `name`.
  The set name is now derived from `Song.file_path` ("Untitled" when
  unsaved). Also: the `song` accessor works whether the base
  `ControlSurface` exposes `song` as a method (`_Framework`) or a property
  (`ableton.v2`, the Live 12 fallback import), `create_midi_track` resolves
  the new track by index when Live returns `None`, and socket-thread
  diagnostics are relayed to `log_message` on the Live thread. `smoke.py`'s
  stand-ins are now pinned to a documented-LOM attribute allowlist so a
  non-LOM attribute cannot pass offline again, and `cargo test -p
  mcp-ableton` checks the MCP tool list against the script's `_OPS` table.
  Still never run against a live Ableton instance — owner acceptance stays
  owed (`ACCEPTANCE_OWED.md` §2.5).
- **Usage tab: Mistral spend was the last per-currency text line (#303):**
  `mistral.rs` now fills the spend-kind contract (`total` = monthly cap,
  `spent` = period spend; `total: 0` when no cap is readable) so the Usage
  tab draws the same stacked spent / remaining-budget / free bar Cursor
  gets. `BalanceBreakdownBar` labels spend-kind rows "Period spend", shows
  "Budget X of Y", and renders a spend-only bar with an explicit "No spend
  cap" legend instead of inventing a budget equal to the spend; ARIA label
  carries the figures. Providers that hand us only a free-form string (no
  structured figures) keep the text fallback — a chart there would be
  fabricated. Vitest 252/252, `tsc` clean.
- **WakePolicy bridge for A2A interrupted states (#365):** a local dispatch
  that raises a human-gated wake stores `input_required` when the task asked
  for approval and `auth_required` when standing policy forces the gate.
  The waiting state is every handoff of the stage's current attempt:
  initial and queued members share one `{stage}/{count}/a{attempt}/{agent}`
  label, and each resolution recomputes that set. A previous attempt is
  ignored. The task resumes `running` when every gated handoff in the
  attempt was delivered, and `failed` when the attempt is fully resolved
  and any of those handoffs was cancelled. Completion and advancement are
  rejected while the task is `input_required` or `auth_required`. Each
  successful parallel dispatch persists that aggregate immediately, so a
  later member's failure or a queued `retry_task` recovery cannot leave a
  gated handoff stored as `running`. `tasks/get` maps those to
  A2A `input-required` and
  `auth-required`. Remote steps stay `running` because they raise no local
  wake. The compile break on this branch is repaired in the same change:
  `tasks/mod.rs` and the deleted model declarations are restored, and
  `WorkflowStep` literals include `sub_task_id`, `handoff_metadata`, and
  `context_filter`. Sub-workflow completion (#366) and context filtering
  (#361) were not in the destroyed commit and are not implemented here.

- **Team Scorecard T7 consistency probes and reviewer-side signature (#347):**
  `hub_get_scorecard` computes four advisory probes — `quota_vs_cost`
  (measured `provider.usage` spend vs `quota.snapshot` balance movement with
  symmetric >2x divergence inside the declared coverage model; top-up-only
  histories read `unknown`, never `ok`; snapshots write best-effort on
  explicit quota refresh with unchanged-value dedupe), `claimant_consistency`
  (stamped action claims — usage/os_tool/budget/mcp only, never
  communication — approved only by time-valid authority at or before the
  agent's earliest claimed action: a `handoff` message or a wake linked to
  that handoff, or a session membership join time. The current task/session
  roster has no interval and cannot approve past work or prove exclusion.
  Ordinary messages and wakes are not assignments. A missing interval stays
  `unknown`, including after reassignment when the earlier handoff is gone
  from the current roster; task and session namespaces stay separate),
  `network_vs_provider_log` (`unknown` until T6 (#346) lands its local-only
  sample store and the two crates agree a read contract — raw network
  samples never join the synced audit chain), and `git_scope_vs_touch_set`
  (`unknown` until a git source exists). Every probe reports
  `ok | disagreement | unknown` per subject; a disagreement writes one
  deduped hash-chained `scorecard.disagreement` row linking both sources —
  candidates are merged per (probe, subject) and the check-and-insert runs
  in one transaction — and agent cards carry an unresolved-disagreement
  badge (a count, never a score). Reviewer-side signature: `review.finding` /
  `review.finding_corrected` audit rows (severity + origin in `process_json`,
  no new columns) feed findings-per-documented-task and a reversal rate that
  counts distinct owner-corrected findings and can never exceed 100%;
  findings dedupe identical re-submissions at write time and attribute only
  to enrolled roster agents; corrections require owner authority.
  Zero-evidence approvals and review dwell stay honestly `unknown` — gate
  decisions record no reviewer identity and a send is not proof of evidence
  inspection. New `ca review finding|correct` CLI entry path.
### Codex follow-up review — 2026-10-20

- Network sampler review: apply trusted-identity expiry before using the baseline, not only on the later pruning write. Added a stale-file regression; retention and unique concurrent-writer temp-file fixes accepted (#346).

### Codex review corrections — 2026-10-20

- Codex corrected the recycled-PID baseline becoming trusted on the next scan (#346, `f30c0fc`). Repeated-scan regression reproduced then passed. Follow-up round (Mistral) resolves the two remaining P2 findings: 24h retention now applies to scan records and the read view (idle polls prune too), trusted `(pid, start_time)` identities are kept separately from coverage so the PID-reuse guard survives the coverage expiry (retiring only after a week without a refresh), and ring writes use unique `create_new` temp files so concurrent writers cannot truncate each other — a reader always sees a complete file. Regressions: idle/expired-scan pruning, view expiry without an append, identity precedence/expiry, rejected-identity non-refresh, and a 4-writer/reader concurrency test. Awaiting Codex re-review; no final T6 PASS yet.
### Codex reconciliation review — 2026-10-20

- Strengthened the P5 approval-resume regression to use max_attempts=1:
  final-attempt approval waits without resubmitting or consuming another
  attempt and completes on approval. Corrected stale gate-contract docs:
  audit links are authoritative; pending_gate_json is retained legacy data.

### Codex Cursor re-review — 2026-10-20

- Scheduler wake replay now propagates errors and serializes check/create;
  failed wake side-channel writes roll back and remain retryable. One-shot
  completion requires finalized outbox delivery; an active pending sender
  is never completed or re-prompted by a stale tick (#351/#355).

### Codex follow-up review — 2026-10-20

- Scheduler review: stale attempt results cannot complete a newer attempt; use attempt-aware CAS for finish and the refreshed snapshot for first one-shot completion (#351). Remaining wake/completion crash gaps are recorded on the issue; not accepted for integration.

### Codex review corrections — 2026-10-20

- Fix scheduler token gates to check every eligible identity, including missing creation baselines (#352); regression reproduced before correction.
### Codex follow-up review — 2026-10-20

- Schedule gate editor review: fixed quoted-argument tokenization and malformed-input rejection; numeric thresholds no longer truncate fractions or accept unsafe integers. Numeric command checks are correctly labeled as exit-zero plus stdout maximum, not exit-code ceilings (#353).

### Codex review corrections — 2026-10-20

- Align Schedule UI manual-tick feedback with the backend outcomes payload (#353), rather than invented summary fields.
- Provide editable command program/argv/check, globs, and numeric thresholds for gate criteria; prevent attaching gates to one-shot tasks and clarify engine bypass (#353).
- Clarify that ca schedule tick --now changes the real execution clock and is not a dry run (#354).
- Reject control characters/non-UTF8 trigger paths, run Settings systemctl operations off the UI thread, retain units if timer disabling fails, and drain `systemctl` stdout/stderr while the child runs (bounded capture) so large environment output cannot stall (#356).
### Added

- **Discord v1 chat bot remote client (U25, #319):** new `ca discord`
  command tree (`pair --channel <id>` / `status` / `unbind [--user-id]` /
  `run [--interval-secs]`) mirroring the Telegram remote-control contract —
  one-time 10-minute pairing codes, a persistent allow-list
  (`discord_binding.json` next to `hub.db`; mutations hold an OS file lock
  until release, so a slow save does not expire a live owner and reclaiming
  an abandoned lock cannot unlink a replacement owner's lock), the same `/start` `/approve`
  `/reject` `/send` `/wakes` command set, silence for unknown senders, and
  wake-resolution audit rows written as `human` with `via = "discord"` (the
  scorecard digest counts them separately). Transport is outbound REST only:
  `GET /channels/{id}/messages?after=<snowflake>` channel-history polling
  plus `POST /channels/{id}/messages` replies — no gateway/websocket, no
  listen port. Bot authors (including the bot's own replies) are skipped,
  per-channel snowflake cursors make restarts replay-safe, 429s back off by
  Discord's `retry_after` (and sends retry once), and messages are truncated
  to Discord's 2000-char limit. The pairing-code parser is shared with
  Telegram (`remote::command`), and the authenticated-command application is
  shared via `remote::dispatch::apply_authenticated`. Token resolves through
  P12 `tool.discord.bot_token` (env `DISCORD_BOT_TOKEN` fallback); the bot
  needs the **Message Content** intent and View/Send/Read-History channel
  permissions. Owner live-verification steps added to
  `docs/moon/ACCEPTANCE_OWED.md` §4.4a.
- **Settings audit on `set_wake_status_via`:** resolving a wake from
  Telegram/Discord/TCP/desktop now records the resolution channel in the
  hash-chained audit stream exactly once per wake (pending → terminal,
  human-gated only; idempotent across retries and cross-channel races).
- **Review digest Discord column:** `DigestResolutionCounts` gained a
  `discord` bucket (serialized with `#[serde(default)]` so older stored
  JSON still decodes).

### Fixed

- **`ca telegram run` no longer exits after the first error:** the poll
  loop keeps running when `getUpdates` returns an error (network blip,
  Telegram 502/504), matching the Discord runner's behavior. Reported by
  owner during U25 live verification.
- **`remote::mod` wake-push duplication removed:** Telegram and Discord
  wake pushes previously each kept their own copy of the
  `wake <id> for <agent> — /approve <id> or /reject <id>` text; the format
  string now lives in one helper so the two transports cannot drift.

## [1.2.0] — 2026-09-20

### Added

- **Scheduled Goals (S4, #348–#355):** durable `goals` +
  `goal_schedule_state` tables, a `ca schedule` CLI
  (`list`/`create`/`get`/`cancel`/`tick`), Tauri `hub_*_schedule_*`
  commands, and a headless `tick` entry point for OS triggers (cron /
  systemd timer / Task Scheduler). A goal dispatches as a tagged Hub
  message to its target agent with quiet-hours, catch-up, max-retries,
  budget-gate, and wake-policy enforcement; dispatch, gates, and audit all
  live in the shared `hub` crate so CLI, desktop, and OS triggers share one
  code path.
- **Scheduled Goals deadline enforcement and dedup (#349/#353):** overdue
  goals are marked `missed` (not silently fired late) unless
  `catch_up = true`; a per-slot idempotency key makes a crashed tick
  re-run safe.
- **Scheduled Goals Tauri commands (#350):** the desktop Shared Hub gains a
  Schedules tab reading the same store as `ca schedule list`, with
  create/cancel actions behind the standing confirmation policy.
- **Scheduled Goals delivery-recovery and quota-pause races (#351/#355):**
  a goal whose dispatch lands while the target is budget-paused records a
  `deferred` outcome and retries on the next tick instead of burning its
  max-retries budget; a goal disabled between read and dispatch in the same
  tick is re-checked before send.
- **Cloud sync: Google Drive replica + preview parity (S7–S10, #96–#99):**
  `ca sync preview` (lock-free read), `ca sync push` / `ca sync pull`
  (mutation-locked), hashed `BlobId` remote object names under
  `devices/<id>/` and `replica/` namespaces in the Drive `appDataFolder`,
  journal-integrity merge with confirm-only deletes, and the Hub mutation
  lock (`sync/lock`) that rejects writes (messages, wakes, tasks, audit
  watch) while a sync run holds it — reads stay available.
- **Cloud-sync S9 device trust, resume, and diagnostics (#99):** local
  `sync/trust.json` + `sync/device.json` register/revoke/retrust devices
  without rotating `keys/cloud-sync.key`. Unknown and revoked devices are
  refused before any blob transfer; interrupted push/pull runs resume from
  the last verified manifest; `ca sync status` reports last-verified base,
  device trust state, and pending conflicts.
- **Settings, memory, audit, and budgets in the TUI (T5, #139):**
  desktop-parity settings inspection and typed mutation (`[tui]` section,
  ordinary + Advanced fields, Danger-zone actions behind the same
  Cancel-first/typed-confirmation contract), memory search, audit review,
  and truthful budget/quota freshness in the Ratatui client.
- **TUI owned & observed harness panes (T6, #140):** `portable-pty`-backed
  owned interactive harness panes (launch, resize propagation, ANSI/VT
  scrollback, focused input routing) alongside strictly read-only observed
  capture panes, with a tabbed pane bar, wide-terminal split tiles, and
  tmux-style `Ctrl+B` prefix chords (`d` detach, `n`/`p` cycle, `s` split,
  `x` close, `c` create).
- **TUI multi-instance coherence (T7, #141):** version-stamped
  reject-and-refresh on concurrent `ca tui` instances sharing one
  `settings.toml`/Hub — a stale write is rejected, a non-blocking banner
  offers `[r]` Refresh & retry, and no last-writer-wins path exists.
- **Kubuntu/Konsole resilience acceptance (T8, #142):** narrow (<80 col)
  and wide (≥120 col) layout suites, mouse-off native selection, ASCII
  fallback (`LANG=C` or `unicode_fallback = true`), panic-hook terminal
  restoration, and a verified zero-write gate on `.agent/**` markdown bus
  files during TUI session/wake workflows.
- **Tool & Workspace Activity view (D4, #324):** a Shared Hub dashboard
  subtab aggregating `tasks`, `work_sessions`, `audit_events`, and
  `harness_captures` into filterable per-agent activity cards (commands
  executed, files touched with operation badges, scope filter
  task/work_session, time-range and text search).
- **Ableton Live MCP bridge (#321):** `coding-assistants-mcp ableton`
  stdio server plus a Python Remote Script (`plugins/ableton/`) speaking
  line-delimited JSON over localhost TCP; creative-tool catalog entry on
  port 9770 with `run_lom` disabled unless `--allow-run-lom` is passed.
  Offline dummy-LOM smoke: `python3 plugins/ableton/smoke.py`.
- **Kimi Code managed harness (C14.14, #309):** Moonshot's `kimi` CLI
  enrolled as a provider-native managed harness — headless
  `kimi -p --output stream-json` spawn, `wire.jsonl` transcript capture,
  and `--session <id>` resume on follow-up turns.
- **Hermes Agent CLI managed harness (C14.16, #322):** Nous Research
  `hermes` enrolled with headless one-shot `-z <task> --in <dir>
  --usage-file <path>` invocation, strict flag sandbox (no
  `--accept-hooks`/`--yolo`), JSONL session export capture, and truthful
  token/cost quota attribution from the usage file.
- **Animated V-Tuber avatar presence (U17, #307):** opt-in per-identity
  Open-LLM-VTuber bridge — a Settings toggle (default off, zero background
  network), a `VTuberPresenceDock` in Chat & Memory, direct `/speak`
  forwarding of completed assistant text (bypassing the bridge's own model
  loop), an on-demand connection probe with non-blocking offline fallback
  to the static avatar.
- **Saved workspaces linked to work sessions (U24, #317):** durable
  `workspaces` table + `hub_list/save/delete_workspace`; a saved-workspace
  picker in Orchestrate's Workspace Root that switches `config.work_dir`
  and focuses Chat & Memory on the workspace's linked session.
- **Telegram remote-control bot (U25, #319):** `ca telegram
  pair/status/unbind/run` — pairing-code allow-list
  (`telegram_binding.json`), outbound `getUpdates` long-poll only (no
  inbound ports), `/approve` `/reject` `/send` `/wakes` mapping onto
  existing HubStore writes as `human`, unknown senders silent, pending-wake
  push notifications to bound chats. Token via P12 `tool.telegram.bot_token`
  (env `TELEGRAM_BOT_TOKEN` fallback).
- **Internal event bus decoupled from AppHandle (P1, #325):** a bounded
  sync-channel broadcast inside the Tauri backend; `AppHandle::emit` and
  the TCP broadcast both subscribe, so desktop and LAN clients receive the
  same `agent-event`/`agent-memory-recall`/`hub:agents-changed` payloads
  without publishers blocking on slow subscribers.
- **Direct HTTP providers via async-openai (P4, #327):** OpenAI-compatible
  chat completions run in-process (async-openai 0.26) with SSE streaming
  onto the agent-event bus — no `opencode` child process; structured
  `unauthenticated` degradation when the key is missing.
- **Runtime budget pause, summary & shutdown (P10, #328):**
  `HubStore::gate_provider_call` blocks the first over-limit call before
  the model API is hit, writes a C6 markdown handoff summary, and holds
  further calls/wakes until explicit human resume; shutdown persists the
  same handoff shape.
- **OS-level tool execution with approval & sandbox (P5, #333):** `ca tool
  run/pending/approve` — safe read commands auto-run under the Standard
  sandbox, mutating commands park as `proposed` with an audit id awaiting
  `ca tool approve`, and shell interpreters/`sudo` are hard-denied. Every
  execution writes a full-argv `audit_events` row.
- **LAN TCP authentication & Android pairing (P6, #332):** the TCP server
  now requires an `Authenticate{token}` first frame (vault
  `tool.tcp.auth_token` / env `CA_TCP_AUTH_TOKEN`); unauthenticated peers
  are dropped and logged as `tcp.auth_rejected` (token never logged), and
  broadcasts are withheld until auth.
- **Remote workflow delegation over TCP (P11a, #334):** a
  `WorkflowStep.peer = "host:port"` runs that step on the paired instance
  headlessly (full P2 isolation/budget) and records the returned
  `TaskComplete` as the step handoff; fails closed on missing/invalid
  token.
- **OpenRouter model-routing gateway (P13, #318):** `provider: openrouter`
  with comma-separated fallback model chains against
  `https://openrouter.ai/api/v1`, per-request cost parsed into task
  attribution. Key via vault `tool.openrouter.api_key` / env
  `OPENROUTER_API_KEY`.
- **xAI Grok Bot direct provider (P15, #320):** a `grok-bot` assistant
  provider distinct from the Grok CLI harness — direct HTTP to
  `https://api.x.ai/v1` with streaming, key via vault
  `tool.grok-bot.api_key` (falls back to `XAI_API_KEY`).
- **Mistral Admin API quota read:** optional `tool.mistral.admin_api_key`
  (env `MISTRAL_ADMIN_API_KEY`) enables the backoffice usage/spend-limit
  read via `x-api-key` header; local Vibe session usage works without it.
- **Alibaba Qwen Code managed harness (C14.13, #308):** `qwen` CLI
  enrolled with headless spawn, session capture
  (`hub_capture_qwen_session` on the 1.5s poll), and readiness reporting.
  Quota stays on #310.
- **Usage tab currency balance breakdown (#303):** stacked horizontal
  `BalanceBreakdown` bar in `HubCharts.tsx` splitting consumed spend
  (`#38bdf8`, red when over), available budget (theme-aware
  `var(--primary)` with <25% amber / <10% red warnings), and
  free/promotional credits (`#34d399`), with currency-formatted totals.
- **Perplexity Web MCP session login (#283 area):** catalog entry
  `mcp.perplexity_web.session_login` documenting the `pwm login`
  interactive flow (token at `~/.config/perplexity-web-mcp/token`).

### Changed

- **`crates/hub/src/remote/` modularized:** the former single Telegram
  remote module is now `remote/{mod,command,binding,telegram,dispatch,
  discord,discord_binding}.rs`; `command.rs` holds the platform-shared
  slash-command parser, `dispatch.rs` the platform-shared authenticated
  application (`apply_authenticated(hub, via, inbound)`), and each
  transport keeps only its wire format and id type (i64 chat ids for
  Telegram, u64 snowflakes for Discord).
- **`HubStore::set_wake_status` → `set_wake_status_via(id, status, via)`:**
  the old method remains as a `via = "desktop"` shim; all four resolution
  channels (`desktop`/`telegram`/`discord`/`tcp`) validate against a fixed
  allow-list and reject unknown channels before touching state.
- **Secret catalog extended (#285):** `tool.discord.bot_token`,
  `tool.grok-bot.api_key`, `tool.mistral.admin_api_key`,
  `provider.openrouter.*`, `provider.grok-bot.*`, and the sync/* fields
  added to the typed `hub::secret::catalog::CATALOG`.

### Fixed

- **Desktop UI scroll stall & layout shift (#213):** removed permanent
  compositor-layer promotion from full-page panels; maximized-window
  scrolling in Shared Hub no longer stutters or jumps on Kubuntu at
  fractional display scaling, and the scrollbar thumb no longer traps the
  cursor during drag.
- **Workspace bootstrap guardrails (#215):** system directories
  (`/etc/...`) are rejected with a visible badge, deep paths with missing
  parents are blocked, `~` expands to the home directory, directory
  creation requires an explicit confirmation dialog, and `.agent/` preset
  chips bypass hidden-file toggling in the OS picker.

## [1.1.0] — 2026-09-09

### Added

- **In-app harness terminal grid (U15, #295–#301):** a hand-rolled
  resizable tiled grid of embedded xterm harness terminals inside
  Orchestrate (`src/components/panels/terminal/`) — binary split-tree
  layout (`layoutTree.ts`), drag splitters, drag-to-rearrange with
  dock/swap drop zones, per-pane maximize, `@xterm/addon-webgl` renderer
  for crisp glyphs at fractional scaling, a drag-resizable grid canvas
  (#298), named grid layouts saved under
  `~/.coding-assistants/terminal-grids/` (#300), and multiple panes per
  harness via per-instance leaf ids (#301). No tiling-library dependency.
- **Kanban board for GitHub issues + internal tasks (U18, #312):** Shared
  Hub Board tab backed by a `hub::github` `gh` client for Project 21,
  with deadline/roster overlays in Hub SQLite and drag-to-column writes
  via `gh project item-edit`.
- **Git branches tab (U19, #313):** read-only Shared Hub Branches view
  (`git for-each-ref` + `rev-list --left-right --count`), linking
  `agent/<name>-<issue>` branches to their GitHub issue with a manual
  override persisted per workspace.
- **Profile rename + Settings profile view (U20, #314):**
  `set_agent_display_name` with validation and audit, a
  `TeamProfilesSection` in Settings listing every roster identity with
  inline rename.
- **Team role assignment (U21, #315):** `agents.role` column (soft
  migration), `hub_set_agent_role`, preset/custom `TeamRoleBadge`
  surfaced in Messager sidebar, chat header, and harness readiness panel.
- **Roster enroll/unenroll fix + Shared Hub roster panel (U22, #225):**
  `addAgentToTeam`'s hard-coded five-identity allowlist dropped — every
  roster identity now persists through `hub_set_team_member` with an error
  banner on failure (previously Muse/Cursor/OpenCode/DeepSeek/Vibe/Qwen/
  Kimi/Mistral adds were silently lost on restart); Shared Hub Dashboard
  rows toggle enrollment via the shared handlers.
- **System resource monitor (U23, #316):** Shared Hub System tab — total
  + per-core CPU, RAM, swap, per-volume disk space/I/O, NVIDIA GPU/VRAM
  via `nvidia-smi`, and per-process attribution for managed harnesses
  (`sysinfo 0.38.4`, reviewed against `DEPENDENCY_POLICY.md`). Read-only.

## [1.0.0] — 2026-08-15

### Added

- **First stable release.** The personal, local-first multi-agent
  collaboration hub: Tauri desktop app (Orchestrate, Chat & Memory, Shared
  Hub, standalone Settings window), `ca` CLI, `ca tui` Ratatui client, and
  the Android TCP companion, all over the shared SQLite `HubStore`.
- Memory (M1–M7): tiered episodic/semantic store, private journals, shared
  bus entries, M3 git-committed Markdown export, M5 retention
  (age-out/purge), M6 cross-agent handoff acceptance flow, M7
  `memory_links` graph with heuristic link suggestions.
- Communication (C1–C14): durable inbox/outbox, ephemeral wakes with
  standing `WakePolicy`, C5 sequential/bounded-parallel workflow tasks,
  C6 budgets with pause/handoff/resume, C7 A2A Agent Cards, C10–C13
  hub-native orchestration (all/subset/one composer, task/wake tags, named
  work sessions, markdown-bus retirement gate), C14 managed harness
  programme (observed vs managed sessions, single-writer leases, Claude
  Channel MCP bridge, Codex app-server delivery, Gemini `agy` workers).
- Persistent Settings (S1–S6): versioned comment-preserving
  `settings.toml` under `CA_HOME`, atomic writes with timestamped
  last-known-good backups, global + per-workspace overrides with
  Inherited/Override status, named provider profiles with redacted
  `SecretReference`s, orchestration policy (sandbox strictness,
  auto-enrollment, export gate, retention), danger-zone purges behind
  typed confirmation.
- Documentation website (W1–W7, #116–#123): React 19 + Vite + HashRouter
  site on GitHub Pages, curated content pipeline from `docs/`, offline
  MiniSearch command palette, dark-first theme, print stylesheet, custom
  404, zero third-party trackers.

### Security

- **C12 provider-safe harness boundary:** task-only injection never writes
  a PTY, never attaches to a foreign interactive session, and never
  fabricates an undocumented control socket; Codex delivery uses the
  documented app-server protocol, Grok the documented ACP leader socket,
  and Claude/Gemini report a truthful `unavailable` until their
  documented transports land.
- **Audit integrity:** hash-chained `audit_events` with `ca audit verify`,
  owner approve/quarantine checkpoint, and redacted settings-audit views.
- **Secrets:** vault-first P12 resolution with env-var fallbacks; tokens
  and keys are never logged and never returned by any IPC command.

[Unreleased]: https://github.com/ACFHarbinger/Coding-Assistants/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/ACFHarbinger/Coding-Assistants/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/ACFHarbinger/Coding-Assistants/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/ACFHarbinger/Coding-Assistants/releases/tag/v1.0.0

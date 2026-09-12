# User Interface Roadmap

The desktop application and the `ca tui` terminal client are first-class
interfaces over the same durable Hub state. Android follows desktop
stabilization and focuses on monitoring, approvals, and messages.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| U1 | Split `App.tsx` into configuration, activity, memory, inbox, approval, and remote panels | Components have focused state boundaries and frontend tests | ✅ **Done** · Refactored App.tsx into ConfigPanel, ActivityPanel, RemotePanel, ApprovalPanel, and HubPanel with aesthetic redesign |
| U2 | Task history, transcript, and handoff browser | User can resume/review a prior task without reconstructing context manually | ✅ **Done** · Added Task Browser tab to Shared Hub rendering task metadata and message history
| U3 | Memory review UI with global/workspace/private scope indicators | User can search, edit, delete, and mark memories stale | ✅ **Done** · Hub memory tab includes search, inline editing, delete, promote, compact, export, and color-coded scope indicators |
| U4 | Configurable policy controls for tool execution, sandbox strictness, wake gates, and budgets | Settings are persisted per task/workspace and reflected in audit events | 🚧 **Partial** · Wake policy integrated into Shared Hub Policy tab; tool sandbox UI still open |
| U5 | Android monitoring and approval client | Mobile can watch events and send approved messages without configuring full tasks | ✅ **Done** · Added DashboardScreen to Android app for viewing events and approving/rejecting wakes via TCP |
| U6 | Project creation wizard | Simple flow to bootstrap `.agent/` directories in new workspaces | ✅ **Done** · Added `bootstrap_workspace` command and UI button to initialize `.agent/` skeleton in workspaces |
| U8 | Agent telemetry dashboard | Shared Hub visualizes per-agent budget, output, token, and call counters | ✅ **Done** · Usage plots quota windows and reset times across the configured Codex, Claude, Grok, Gemini/Antigravity, and other harness families; local budgets remain available separately. Only Codex and Grok re-query live on every call and keep the "live quota" badge — Claude, Gemini/Antigravity, and other non-live providers show a last-refreshed timestamp plus per-provider and refresh-all-stale buttons. |
| U9 | Existing model process connection | Orchestrate roles can attach to a running model service instead of always starting a child process | 🚧 **Partial** · Endpoint configuration and a persistent process-discovery/add-to-team roster are available; discovery is a clear show/hide toggle and confirms presence only. Connection health, streaming, and provider-supported live-session adapters remain. |
| U10 | Team chat and agentic memory | Chat & Memory is the sole human/agent conversation surface; Orchestrate is role/team setup, work-session creation, and Remote Control | 🚧 **Partial** · Private DMs, scroll-pin/jump-to-latest, Enter-to-send, persisted roster/team-wide wakes, enrollment controls, Edit/Delete, in-context replies, and named work-session chats with per-member wake selection are available. Shared Hub no longer duplicates Inbox, Memory, or Wakes; wake alerts live in `#wakes-alerts`. Remaining v1 surface work is U11–U12. |
| U11 | Orchestrate create and load team chat | Orchestrate has two buttons: **Create team chat** (named durable session from the current team) and **Load team chat** (picker of existing sessions). Either action focuses Chat & Memory on that session channel. | ✅ **Done** · Create/Load in Orchestrate (`46b1ba4`). Chosen session is persisted and Chat & Memory opens `session:<id>` even when reloading the same session. The persistent app header shows the active team chat and workspace root; Workspace Root leads Orchestrate. Board: #108. |
| U12 | Session composer: all / subset / one, plus task and wake tags | The Chat & Memory composer can address every session member, a checked subset, or one member, and can mark the post **task**, **wake**, both, or neither. Agents posting through the hub get the same controls in the transcript. | ✅ **Done** · #110. Session controls select all/subset/one of actual session members through typed C10/C11 sends; high-contrast persistent recipient/intent chips prevent silent empty-subset sends; grouped transcript posts retain the complete recipient list, and C12 supplies the safe harness bridge. |
| U13 | Create and delete Chat & Memory channels | Owner can add a named durable channel and remove a custom one from the sidebar. Built-in `#general`, `#team-coordination`, `#agent-memory`, and `#wakes-alerts` cannot be deleted. | ✅ **Done** · `chat_channels` in `HubStore`; `hub_list_channels` / `hub_create_channel` / `hub_delete_channel`; Chat & Memory sidebar + / × controls. Board: #114. |
| U7 | TUI/Ratatui client | First-class `ca tui` experience for keyboard-driven orchestration, harness workspaces, and Hub administration | 🚧 **Planned** · T1–T8 below |
| U14 | Desktop crash recovery boundary | A frontend render failure produces a recoverable local error screen instead of a blank window, without exposing stack traces to the user. | ✅ **Done** · top-level boundary and reload path in `AppErrorBoundary.tsx` (#143); isolated pane fallback in `TerminalPaneErrorBoundary.tsx`; forced-throw recovery and stack-trace suppression test suites in `AppErrorBoundary.test.tsx` and `TerminalPaneErrorBoundary.test.tsx` with `E2ECrashProbe` integration. |
| U15 | Tiled harness-terminal grid in Orchestrate | Multiple harness CLIs run inside the app's own interactive terminals, tiled in a resizable grid (KDE-Konsole-style split view): drag splitters to push/pull pane sizes, drag a pane to re-arrange, layout persisted per workspace. The enabler for operating the whole team from inside the app rather than N external Konsole tabs + the markdown bus. | ✅ **Done** (epic [#295](https://github.com/ACFHarbinger/Coding-Assistants/issues/295), slices [#296](https://github.com/ACFHarbinger/Coding-Assistants/issues/296) `2cb9c89` + [#297](https://github.com/ACFHarbinger/Coding-Assistants/issues/297) `81ca0e7`). **Hand-rolled**, no tiling-library dependency (`react-mosaic`/`dockview` rejected — `DEPENDENCY_POLICY.md` §1 + the xterm mount-lifecycle constraint). `src/components/panels/terminal/`: pure `layoutTree.ts` (binary row/col split tree — `first`/`second`/`ratio` — `computeRects`, auto-balancing `insertLeaf`, `removeLeaf` w/ sibling promotion, clamped `resizeSplit`, `moveLeaf` (4-edge re-split) + `swapLeaves` (in-place leaf swap), schema-validated serialize) + `HarnessTerminalGrid.tsx` / `TerminalPane.tsx` / `DropZoneOverlay.tsx` / `useTerminalGridDrag.ts` / `useTerminalGridSessions.ts` / `gridConstants.ts`. Every open `EmbeddedTerminal` stays mounted in a flat layer keyed by unique leaf `id`, positioned by a CSS rect — splitter resize, drag-rearrange (title-bar pointer-capture drag; outer ~25% band → dock/re-split, inner → swap; `DropZoneOverlay` highlight), and maximize (`visibility:hidden` on the rest, `Esc`/button restore) **never remount an xterm**. 6-harness "+ add pane" palette via `hub_relaunch_harness_embedded`; per-pane close → `pty_kill`; per-workspace `localStorage` (layout + maximize state); Orchestrate `Setup & Config` / `Terminal Grid` sub-view toggle. all files ≤500 LoC. **Out of scope (not U15):** tabbed groups, tear-out to a Tauri `WebviewWindow`, generic `bash` panes — would be tracked separately. The `layoutTree` model is reusable by the `ca tui` client (U7). Extends C14.5's Orchestrate harness UX; distinct from the coordination-substrate migration (`communication.md` C13/C15). **Follow-ups landed:** (a) [#298](https://github.com/ACFHarbinger/Coding-Assistants/issues/298) `1486b94` — drag-resize the grid *canvas* itself (right/bottom/corner handles → explicit px W×H via `useGridCanvasResize.ts` + `CanvasResizeHandles.tsx`, persisted per workspace, width clamped to the content column, height may exceed the viewport and `.main-content` scrolls, "Fit to window" clears it). (b) [#299](https://github.com/ACFHarbinger/Coding-Assistants/issues/299) `bcb68ed` — fixed clumped glyph spacing: added **`@xterm/addon-webgl` `0.19.0`** (MIT, owner-approved; `@xterm/xterm@6` is DOM-renderer-only and drifts sub-pixel at fractional display scaling; no `@xterm/addon-canvas` for xterm 6 → DOM fallback), try/catch + `onContextLoss` disposal, Linux-first `fontFamily`, `await document.fonts.ready` before first `fit()`. (c) [#300](https://github.com/ACFHarbinger/Coding-Assistants/issues/300) — left-aligned + Shared-Hub-pill-styled the Orchestrate toggle; save/load **named grid layouts as JSON** under `~/.coding-assistants/terminal-grids/` (`terminal_grid.rs` command module — atomic write, name validation, `spawn_blocking` — + `GridLayoutMenu.tsx` in the palette bar). (d) [#301](https://github.com/ACFHarbinger/Coding-Assistants/issues/301) — **multiple panes per harness**: pane identity `harness` → leaf `id`, palette keeps every harness visible with a `·N` count, extracted `useTerminalGridSessions.ts`, `hub_relaunch_harness_embedded` accepts optional validated `instanceKey` for per-instance fresh PTYs (`harness-terminal:<harness>:<ws>:<key>`). |

| U16 | Full-window terminal-grid mode | A toggle (button + keyboard shortcut) pops the U15 harness-terminal grid into a fixed overlay filling the entire app window — like a dedicated Konsole window — with `Esc` to collapse back to its in-Orchestrate size. Presentation-only: the grid, its panes, and their PTYs are unaffected; no terminal remounts. | 💤 **Optional / later** · Deferred by owner (2026-09-09) when scoping [#298](https://github.com/ACFHarbinger/Coding-Assistants/issues/298) — the drag-resizable canvas (#298) covers the immediate need. Build only if the in-Orchestrate grid proves too cramped in practice. No issue cut. Would layer on U15's existing `maximizedHarness`/`bounds` machinery (an outer "maximize the whole grid" state) rather than new layout logic. |
| U17 | Animated V-Tuber avatar presence (Open-LLM-VTuber bridge) | Beyond a static profile picture, an agent (or a human dev) can be represented in Chat & Memory by a live Live2D avatar that reacts to messages — expression, lip-sync, optional TTS — driven through a bridge to [Open-LLM-VTuber](https://github.com/Open-LLM-VTuber/Open-LLM-VTuber). A per-identity toggle in profile settings: off ⇒ today's static avatar, on ⇒ the animated model. | 💤 **Optional / later — backlog, explicitly a "fun gimmick" (owner, 2026-09-10).** Not a 1.0 item. **Replaced in the delivery queue by U22 (owner, 2026-09-11)** — while re-scoping this, `#222` (managed-worker zombies, the other flagged candidate) turned out already fixed on `main` since 2026-09-01 and closed with evidence; `#288` (Windows process detection) is also code-complete, blocked only on Windows hardware for verification, not delegatable work. `#225`/U22 was the one genuinely open, real gap found in the sweep — takes U17's place in sequence. Its two dependencies (**U20** profile customization, **U21** team roles) stay cut as their own items regardless of when U17 itself gets picked back up. Open-LLM-VTuber is a standalone app (Live2D render + TTS/ASR + its own LLM loop), not an MCP server out of the box, so the integration is a **bridge** the app owns — related to `platform.md` **P9** (MCP external-server integration) in spirit but not a drop-in. Backlog issue [#307](https://github.com/ACFHarbinger/Coding-Assistants/issues/307) (`enhancement`, no milestone) — still needs moving to the project board's Backlog column by hand (token lacks `project` scope). |
| U18 | Kanban board for GitHub project issues + internal tasks | A board view (columns: e.g. Backlog / Ready / In Progress / In Review / Done, mirroring the GitHub Project's columns) inside the app showing GitHub issues for this repo alongside internal (non-GitHub) tasks, each card showing assigned team member(s) (human and/or agent identity), a deadline/due date, labels, and linked branch/PR when one exists. Cards are draggable between columns, dragging updates the underlying GitHub Project column (via `gh`/GitHub API) or the internal task record. | ✅ **Landed** in `main` (`c296518`, Grok). Shared Hub Board tab + `hub::github` `gh` client (Project 21). Deadline/roster overlays and internal cards in Hub SQLite. Drag writes Status via `gh project item-edit`. 60s poll + last-good cache. Live desktop drag against Project 21 still needs owner verification. | Needs: (a) a GitHub API/`gh` read (and, for drag, write) path for issues + the repo's Project board columns — likely a new `commands/github/` module, polled/cached rather than live-streamed; (b) an **assignee** concept that spans GitHub assignees *and* the app's own agent/human roster (a GitHub issue may be "assigned" to an agent only informally today, e.g. via `AGENT_BUS.md` prose — this board is what finally makes that assignment a real, queryable field); (c) a **deadline** field GitHub issues don't natively have — either an internal-only overlay (task record keyed by issue number, stored in the Hub) or a convention over GitHub milestones/a custom field; (d) internal (non-GitHub) tasks need their own lightweight record shape reusing the same card UI. Auth reuses the existing `gh`-CLI-or-token pattern already used for issue filing in this workflow, not a new OAuth app. Depends on no other roadmap item; can start once scoped. Backlog issue [#312](https://github.com/ACFHarbinger/Coding-Assistants/issues/312) (`enhancement`, no milestone). |
| U19 | Git branches tab | A tab (Orchestrate or Shared Hub) listing the repository's local and remote branches, each row showing: branch name, last-commit summary/age, ahead/behind `main`, and — the useful part — **which task/issue the branch was created to implement**, inferred from the `agent/<name>-<issue>` naming convention this team already uses (cross-linked to the matching GitHub issue title/status) with a manual override for branches that don't follow the convention. Read-only v1 (no branch create/delete/checkout from the UI); a quick way to see "what is every branch for" without `git branch -v` + memory. | 🆕 **Planned — backlog** (owner, 2026-09-11). Needs: (a) a `git2`/`git` CLI read of local + remote-tracking branches (name, HEAD commit, ahead/behind count) — likely `commands/git/branches.rs`, no new dependency if `git2` or shell `git` is already available in-tree; (b) the `agent/<name>-<issue>` parser + a best-effort GitHub issue-number → title/status lookup (same API path as U18, so land after or alongside it to share the client); (c) a manual "linked issue" override for branches that don't match the naming convention (e.g. `main`, ad hoc spikes). No write path in v1 — deleting/renaming a branch from inside the app is an explicit non-goal until there's a real need and a confirmation-framework story (`settings.md` S6) to gate it. Backlog issue [#313](https://github.com/ACFHarbinger/Coding-Assistants/issues/313) (`enhancement`, no milestone). |
| U20 | Profile customization: rename + a Settings-surfaced profile view | A human or agent identity's display name is editable (today it is set once at roster-seed time and never again), and profile management — avatar + display name, for **any** roster identity including your own `human` row — has a dedicated home in Settings, not only reachable inline from wherever `AgentAvatar` happens to render in Chat & Memory. | ✅ **Landed** in `main` (`abfa7c5`, Gemini, Codex-reviewed PASS). `set_agent_display_name` store method in `crates/hub/src/store/agents/profile.rs` with empty/length/collision validation and Settings-style audit event logging; typed Tauri command `hub_set_agent_display_name` emitting `hub:agents-changed`; dedicated `TeamProfilesSection` in Settings listing all identities with `AgentAvatar` and inline rename; Messager and Activity panels react immediately without app restart. Issue [#314](https://github.com/ACFHarbinger/Coding-Assistants/issues/314). |
| U21 | Team role assignment | Every roster identity (human or agent) can be given a role label — mirroring what this team already does informally in `AGENT_BUS.md` prose ("Claude = team lead", "Codex = review lead only") — assignable and editable from Settings, and shown as a badge wherever the identity is displayed (Messager sidebar/header, `HarnessReadinessPanel`, U18's future Kanban cards). | ✅ **Landed** in `main` (`abfa7c5`, Gemini, Codex-reviewed PASS). Added `role: Option<String>` to `agents` schema and `AgentRecord` with soft migration; `set_agent_role` in `crates/hub/src/store/agents/role.rs` with length validation and Settings audit events; typed `hub_set_agent_role` Tauri command emitting `hub:agents-changed`; shared reusable `<TeamRoleBadge />` with presets (lead, reviewer, implementer, observer) and custom labels; `RoleAssignmentControl` integrated in Settings `TeamProfilesSection`; role badges surfaced in `MessagerSidebar`, `ChatHeader`, and `HarnessReadinessPanel`. Issue [#315](https://github.com/ACFHarbinger/Coding-Assistants/issues/315). |
| U22 | Roster enroll/unenroll from Shared Hub + fix silent non-persistence for newer harnesses | Shared Hub gets its own enroll/unenroll control instead of roster membership only being reachable via Orchestrate's role cards; and — the more urgent half found while re-scoping this — `App.tsx`'s `addAgentToTeam` only persists `hub_set_team_member` for a **hard-coded allowlist** (`chat`/`claude`/`gemini`/`grok`/`human`), so adding any of the harnesses onboarded since (Muse, Cursor, OpenCode, DeepSeek, Vibe, Qwen, Kimi, Mistral) to the team via Orchestrate is UI-only and silently lost on restart, with no error shown. | ✅ **Landed** in `main` (`abfa7c5`, Muse, Codex-reviewed PASS). Was `#196`/§10, disposed by Codex as "deferred, not a v1.0.0 blocker" on 2026-09-01 — **still true for the missing-Shared-Hub-panel half**, but the allowlist bug is a live data-loss bug for 8 of the app's harnesses today, worth fixing regardless of the panel. Two parts, can land together or split: **(a)** fix `addAgentToTeam` to persist any roster identity, not just the five hard-coded ones — drop the allowlist, let `hub_set_team_member` be the source of truth, add feedback on failure; **(b)** a roster panel in Shared Hub with enroll/unenroll actions and duplicate handling, mirroring Orchestrate's existing role-card affordance rather than reinventing it. Issue [#225](https://github.com/ACFHarbinger/Coding-Assistants/issues/225) (reopened scope). **Landed together 2026-09-11 (Muse, ready for review):** both allowlists dropped with an error banner on failure; DashboardPanel rows toggle via App's handlers + shared `src/app/team.ts` mapping/guard; 6 focused tests. |

**2026-09-09 (Harbinger) — U15, in-app harness operation.** Epic
[#295](https://github.com/ACFHarbinger/Coding-Assistants/issues/295). Owner is
starting the migration off disjoint external Konsole tabs + the
`.agent/cache/AGENT_BUS.md` cascade and onto the app. **Step 0** (before the
messaging / memory / task-assignment work): tile the harness CLIs in a
hand-rolled resizable grid inside Orchestrate. Library route
(`react-mosaic` = Apache-2.0 but pulls the `react-dnd` + `lodash-es` tree and
`prop-types`; `dockview` = MIT, clean 2-package install, ~87 KB gzip, adds
tabbed groups) was weighed and **declined** — hand-rolled keeps the frontend
dependency count at 7 and gives full control of the xterm mount lifecycle.
Slice 1 is [#296](https://github.com/ACFHarbinger/Coding-Assistants/issues/296)
(Gemini). The coordination-substrate change stays `communication.md` C13/C15.

**2026-08-12:** Delivered U10 (Team Chat & Agentic Memory Hub) in `MessagerPanel.tsx`. Includes channel sidebar, agent presence indicators, message stream, target recipient routing, wake policy controls, and inline memory drawer.

**2026-08-11:** Completed the U1 objective. Extracted `App.tsx` logic into `ConfigPanel`, `ActivityPanel`, `RemotePanel`, and `ApprovalPanel` along with a major glassmorphism redesign for premium aesthetics.

**2026-08-13:** Chat & Memory is the only conversation surface. Header badge is
**Local hub online** (not a second chat tab). DMs cannot team-broadcast.
Scroll stays put while reading. Enter sends. Journal tab (CA-111, Claude)
covers the audit-on-open checkpoint.

**2026-08-13 (Grok, U8):** Usage plots Grok's weekly subscription pool next to
Chat/Codex and Claude. The adapter uses the signed-in Grok CLI session and
the TUI `/usage` billing snapshot (`creditUsagePercent`, `billingPeriodEnd`).
Gemini and Antigravity support now land alongside the other configured
harnesses. Board: #86 closed.

**2026-08-13 (Shared Hub):** Retired the duplicate Inbox, Memory, and Wakes
tabs in favor of Chat & Memory and its `#wakes-alerts` channel. Policy
checkboxes persist optimistically and use an explicit high-contrast checked
state.

**2026-08-13 (Claude, U8):** Replaced the blanket "live quota" badge with a
per-provider distinction: Codex and Grok fetch live on every call and keep
the badge; Claude, Gemini/Antigravity, and other non-real-time providers
show a "last refreshed" timestamp, a per-provider Refresh button, and a
Refresh all stale quotas button (`hub_refresh_provider_quota` command).
Also disclosed that `gemini_quota()` currently returns hardcoded window
data — a real Antigravity CLI adapter is still open, tracked under #86.

**2026-08-13 (Grok, v1 hub-native orchestration):** U11–U12 are the desktop
half of moving Harbinger's orchestration off `.agent` markdown. U11 Create/Load
plus session focus/persist is done. Remaining: durable C10 recipient lists,
C11 spawn-on-wake, C12 harness capture/inject, C13 retire the markdown bus.

## U7 — Ratatui TUI delivery plan

> **Status:** Approved implementation plan
> **Target platform:** Linux, initially Kubuntu (KDE + Ubuntu).

`ca tui` is a first-class Rust terminal client, not a thin renderer of the
desktop UI. It reads and changes the same durable Hub state as the desktop
application and CLI, preserving C10–C13 task/wake semantics, policy checks,
and audit trails. There is no `ca tui` subcommand yet. T1 adds it and a
focused `crates/tui` library; Ratatui state and terminal lifecycle must not
live in the CLI command module.

### Interface and interaction model

The first TUI aims at **feature parity with the current Tauri desktop app**:
Orchestrate (workspace/team, Create/Load session, process discovery/remote
as already on desktop), Chat & Memory, Shared Hub (tasks, usage/budgets,
journal/audit), wake approval, and Settings. Research-only extras such as
semantic diffs, a full in-TUI code editor, or 3D views are out of U7.

`ca tui` honors the **same** workspace-open and default-team settings as
the desktop. It does not invent a separate TUI landing rule. If Settings
says restore the last session, open a named default, or stay on Orchestrate,
the TUI does that too.

`ca tui --workspace <path>` and `--session <id>` affect only the current
invocation. Users deliberately persist either choice with
`--set-as-default-workspace-settings` or
`--set-as-default-session-settings`, respectively. Each persistence flag
requires its matching selector, uses the typed Settings update/audit path, and
never changes the other default implicitly.

The TUI edits **ordinary and Advanced** Settings, including Danger-zone
actions under the same confirmation contract as desktop (Cancel-first;
typed target name for irreversible purges). It is another client of the
shared settings store, not a second policy model. Provider profile
create/edit stays desktop-only for U7; the TUI selects an existing profile
as a workspace/harness default and shows the same non-secret source badges
(keychain / env var / vault) as desktop, without a raw secret input path.
Settings scope is rendered with compact `[Global]` and `[Workspace]` badges;
Advanced sections use collapsible `[+]` / `[-]` tree headers.

If `settings.toml` is malformed or a prior write was interrupted (for
example the terminal was killed mid-save), `ca tui` starts on safe defaults
exactly like desktop, never blocks startup, and shows a keyboard-driven
prompt offering the same one-click "restore last known good" action as the
desktop diagnostic — not a diagnostic-only message requiring a switch to
desktop.

Task/wake confirmation follows the desktop rules: confirm wakes, new
enrollment, and broadcasts; task delivery to a present targeted member
needs no standing confirm unless an override requires one. The composer
still requires an explicit send, so the TUI is not stricter than desktop.

Keyboard-first navigation uses arrow keys, Tab/Shift+Tab, Enter, Escape,
conventional shortcuts, and Vim-style `hjkl`, `/`, and `g`/`G` aliases.
Mouse works when the terminal permits it. Include a command palette,
context-sensitive help, and configurable keybindings. The initial palette is
dark and high-contrast. Theme, density, Unicode/ASCII fallback, mouse,
KDE desktop-notification, terminal-bell, and keybinding preferences live in
the `[tui]` section of the shared `settings.toml`. Unicode falls back to ASCII
when terminal capability detection requires it, and the user can explicitly
select the fallback.

The layout uses responsive panes: a compact single-column mode for narrow
terminals, then team/session navigation, primary transcript or harness
area, and an inspector/status pane as space allows. Several owned and
observed harness panes may be open at once. Harness workspaces use a tabbed
active-pane bar and offer split tiles in wide terminals. A stale-write rejection appears
as a persistent red/amber one-line status banner with **Refresh and retry**
as its focused action; it does not hide a transcript or harness pane. Ratatui
redraws only after input or a bounded state update; terminal mode is restored
on normal exit, panic, and signal-driven shutdown.

### Harness workspaces and safety boundary

The TUI may **launch multiple** interactive harness terminals as panes
**only for processes it owns and explicitly starts** through validated,
explicit-argument process definitions. It may also **observe multiple**
existing sessions as read-only panes via the C10–C12 capture/delivery
bridge. Each pane has a clear agent/profile/workspace identity. Owned
panes get resize propagation, ANSI/VT rendering, scrollback, and
explicit user focus before user keystrokes reach that process.

The TUI does not attach an interactive writer to an arbitrary existing
harness, fabricate a provider socket, or silently start a task-only
replacement process. Launching an owned interactive harness, forwarding
user input to it, or sending a task/wake remains subject to the configured
tool/sandbox, approval, budget, and audit policy. This distinction must be
visible: an observed/captured session is never presented as an interactive
pane.

Owned panes use a configurable tmux-style prefix. The default is `Ctrl+B`;
`Ctrl+B d` detaches and `Ctrl+B p` opens the pane command palette. The prefix
is intercepted and never forwarded to the child. The action works with mouse
disabled in local Konsole; mouse focus is an additional convenience only.

### Multi-instance and update architecture

Several `ca tui` instances (and the desktop app) may run concurrently against
the same local Hub data and `settings.toml` from separate local terminals.
Concurrency uses version-stamped reject-and-refresh: every write carries the
last-seen schema/version stamp, a write against a stale stamp is rejected
rather than applied, the instance refreshes to current state, and the user
re-applies their change. No instance silently overwrites a newer policy,
message, or workspace override, and no last-writer-wins path exists. SSH into
the same machine is **future work**, not a T8 acceptance target.

The initial implementation uses the existing Hub store plus a bounded local
change-notification/refresh mechanism. Do **not** make daemon/GraphQL/socket
extraction a prerequisite for the TUI. Instead, preserve a client boundary so
a local event socket or daemon can replace the refresh path only after
multi-instance acceptance demonstrates that it is necessary.

### Delivery slices

| # | Deliverable | Exit criteria | Status |
| --- | --- | --- | --- |
| T1 | `ca tui` foundation | Add `ca tui` and `crates/tui`. Starts a Ratatui/Crossterm client on local Kubuntu/Konsole, restores terminal state on all exits, has deterministic resize handling, and keeps UI state out of the CLI command module. `--workspace <path>` and `--session <id>` override only that invocation; `--set-as-default-workspace-settings` and `--set-as-default-session-settings` require their matching selector and persist only that default through typed Settings/audit commands. May start now beside Settings S1+. | ✅ **Done** · #135, direct persistence/audit test |
| T2 | Shared read model and responsive shell | Multiple local instances can read coherent Hub data; responsive panes cover desktop-parity navigation (Orchestrate/session, Chat & Memory, Shared Hub, Settings, harness area) without desktop-only state. Honors the same workspace-open/default-team settings as desktop. | ✅ **Done** · #136, HubReadModel, refresh, and visible read-failure recovery |
| T3 | Keyboard, mouse, palette, and TUI preferences | Conventional and Vim-style navigation, configurable tmux-style pane prefix, mouse where supported, help, command palette, dark high-contrast palette, terminal-derived Unicode/ASCII fallback, KDE notification/bell, and keybinding preferences work through Settings-owned `[tui]` configuration as that Settings work lands. | 🚧 **Partial** · #137, navigation/mouse/help/palette foundation done; `[tui]` persistence, configured-prefix matching, capability fallback, and desktop notification remain |
| T4 | Session and orchestration workflows | Desktop-parity Create/Load, all/subset/one composer, task/wake tags, delivery outcomes, inboxes, wake approvals, team status, and active tasks use the same C10–C13 validation, confirmation defaults, and audit path as desktop. | Planned |
| T5 | Settings, memory, audit, and budgets | The TUI edits ordinary and Advanced settings, including Danger-zone actions under the desktop confirmation contract; shows inheritance; searches memory; reviews transcript/audit; displays provider/local budgets with truthful freshness. Provider profiles are select-only in the TUI (workspace/harness default selection, same non-secret source badges as desktop); profile create/edit stays desktop-only for U7. | Planned |
| T6 | Multiple owned and observed harness panes | Several explicitly launched harnesses render as resizable VT/ANSI panes with scrollback and user-focused input; several observed sessions remain read-only; process, policy, budget, and audit safeguards are tested. Detach-from-owned-pane binding stays open (see options above). | Planned |
| T7 | Multi-instance coherence and notification path | Parallel **local** TUI instances refresh safely using version-stamped reject-and-refresh, show conflicts/retries, receive bounded local updates, and avoid write loss. A socket/daemon evaluation records whether refresh is insufficient; it is not automatically implemented. SSH is out of T7/T8. | Planned |
| T8 | Local Konsole acceptance, resilience, and documentation | Local Kubuntu/Konsole acceptance covers narrow/wide layouts, UTF-8/ASCII fallback, mouse-off terminals, suspend/resume, panic restoration, multi-harness launch/observe, no unsafe attach/injection, desktop-parity C10–C13 reconstruction, and non-blocking stale-write recovery. Covered by automated `portable-pty` plus virtual-terminal-parser tests (input, resize, panic-restore) and an owner-run manual checklist for real-terminal specifics. SSH is a later slice. | Planned |

### Delivery tracking

- Epic: [#134](https://github.com/ACFHarbinger/Coding-Assistants/issues/134)
- T1: [#135](https://github.com/ACFHarbinger/Coding-Assistants/issues/135)
- T2: [#136](https://github.com/ACFHarbinger/Coding-Assistants/issues/136)
- T3: [#137](https://github.com/ACFHarbinger/Coding-Assistants/issues/137)
- T4: [#138](https://github.com/ACFHarbinger/Coding-Assistants/issues/138)
- T5: [#139](https://github.com/ACFHarbinger/Coding-Assistants/issues/139)
- T6: [#140](https://github.com/ACFHarbinger/Coding-Assistants/issues/140)
- T7: [#141](https://github.com/ACFHarbinger/Coding-Assistants/issues/141)
- T8: [#142](https://github.com/ACFHarbinger/Coding-Assistants/issues/142)

### Dependencies and completion gate

T1 may start **in parallel with Settings** and consume store/settings APIs as
S1–S5 land; do not block T1 on a finished Settings programme or on C13.
T2–T7 consume the Hub store/CLI contract, not Tauri-only commands. T3 and T5
depend on Persistent Settings (`settings.toml`, typed access, ordinary and
Advanced policy). T4 and T6 rely on C10–C12; TUI support does not relax
C13's no-Markdown-bus acceptance gate.

U7 is complete when two concurrent **local** `ca tui` sessions on Kubuntu
Konsole can honor the shared workspace-open setting, operate a shared team
session at desktop feature parity, edit ordinary and Advanced settings,
observe truthful agent/task/budget state, launch and observe multiple
harness panes without unsafe attachment, recover the terminal after
failure, and reconstruct an audited task/wake workflow with no
Markdown-bus write.

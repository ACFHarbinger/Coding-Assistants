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
| U12 | Session composer: all / subset / one, plus task and wake tags | The Chat & Memory composer can address every session member, a checked subset, or one member, and can mark the post **task**, **wake**, both, or neither. Agents posting through the hub get the same controls in the transcript. | 🚧 **In Review** · session controls select all/subset/one of the actual session members and invoke typed C10/C11 sends; recipient and intent badges render in the transcript. Agent-harness posting parity remains C12. |
| U13 | Create and delete Chat & Memory channels | Owner can add a named durable channel and remove a custom one from the sidebar. Built-in `#general`, `#team-coordination`, `#agent-memory`, and `#wakes-alerts` cannot be deleted. | ✅ **Done** · `chat_channels` in `HubStore`; `hub_list_channels` / `hub_create_channel` / `hub_delete_channel`; Chat & Memory sidebar + / × controls. Board: #114. |
| U7 | TUI/Ratatui client | First-class `ca tui` experience for keyboard-driven orchestration, harness workspaces, and Hub administration | 🚧 **Planned** · T1–T8 below |
| U14 | Desktop crash recovery boundary | A frontend render failure produces a recoverable local error screen instead of a blank window, without exposing stack traces to the user. | 🚧 **In Review** · top-level boundary and reload path implemented in #143; add a forced-throw boundary test when the root frontend test harness exists. |

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
| T2 | Shared read model and responsive shell | Multiple local instances can read coherent Hub data; responsive panes cover desktop-parity navigation (Orchestrate/session, Chat & Memory, Shared Hub, Settings, harness area) without desktop-only state. Honors the same workspace-open/default-team settings as desktop. | ✅ **Done** · #136, HubReadModel & test |
| T3 | Keyboard, mouse, palette, and TUI preferences | Conventional and Vim-style navigation, configurable tmux-style pane prefix, mouse where supported, help, command palette, dark high-contrast palette, terminal-derived Unicode/ASCII fallback, KDE notification/bell, and keybinding preferences work through Settings-owned `[tui]` configuration as that Settings work lands. | Planned |
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

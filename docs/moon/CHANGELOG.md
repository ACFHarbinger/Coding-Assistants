# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Tauri backend layout.** Reorganized `src-tauri/src` by responsibility:
  `agent/` contains orchestration, `client/` external model clients,
  `harness/` capture and delivery adapters, `hub/` desktop Hub commands,
  `server/` local TCP services, `core/` filesystem/process utilities, and
  `main/` the binary entry point. Module names and Tauri IPC command names are
  unchanged.
- **Hub command decomposition.** Split the former 1,875-line Tauri Hub
  command module into bounded store, memory, messaging, workflow, quota, and
  test modules (all at or below 500 lines). The registered IPC command names
  and their JSON payloads are unchanged.
- **C12-CLAUDE-BRIDGE.** Real, verified discovery of already-running Claude
  Code sessions for task delivery (`crates/hub/src/claude_bridge.rs`),
  wired into `inject_harness`. `claude agents --json` (documented in
  `claude --help`) lists every active interactive/background session with
  its pid, cwd, session id, and status — confirmed directly on a live
  machine: it lists the very session this code was written in. Each active
  session also listens on a real Unix socket at
  `$XDG_RUNTIME_DIR/cc-socks/<pid>.sock`, confirmed with `lsof -U` against
  that live pid. Unlike Codex's `app-server`, that socket's wire protocol
  is undocumented Claude Code internals, so this bridge does **not**
  attempt to connect to it — writing arbitrary bytes into a live
  interactive session's control channel with no documented protocol and no
  way to verify a safe outcome is not a responsible automated action.
  Delivery always resolves to a clearly explained `unavailable` (task
  stays queued), the same safety shape as a missing bridge socket, except
  every claim behind it is grounded in something actually observed rather
  than assumed. Never spawns a replacement `claude -p` process or writes
  to a PTY.
- **C12 bridge review.** Gemini/Antigravity capture remains available, but its
  published CLI has no supported active-session IPC/RPC transport. The former
  assumed bridge-socket implementation was removed: task delivery now reports
  `unavailable` and stays queued rather than claiming delivery through an
  undocumented interface. Claude likewise performs documented active-session
  discovery but safely leaves tasks queued because its live socket protocol is
  undocumented. Grok retains the provider-supported ACP leader path.
- **Crate names simplified.** Workspace packages/directories are now `hub`
  and `cli` (the CLI binary remains `ca`); Rust imports, build commands, and
  project documentation use the new names.
- **C12-GROK-BRIDGE.** An explicitly registered Grok session can receive a
  queued Hub **task** through Grok Build's documented ACP leader path
  (`grok agent --leader stdio` → `session/load` + `session/prompt` on
  `~/.grok/leader.sock`). That is an ACP client of the existing leader, not
  a replacement TUI. If the leader socket is missing, delivery is
  `unavailable` and the task stays queued. Register with
  `hub_register_harness_session` (or let Grok capture auto-register the
  latest on-disk session). `hub_inject_harness` / `ca msg tag --dispatch`
  use this path for Grok tasks.

- The app header now continuously shows the selected **Workspace root** and
  **Active team chat**. Orchestrate places the editable Workspace Root control
  at the top, before team/session configuration.
- Chat & Memory can **create and delete** durable sidebar channels. Built-in
  `#general`, `#team-coordination`, `#agent-memory`, and `#wakes-alerts`
  stay; custom names persist in the hub store and can be removed from the
  sidebar (messages remain). Tracked as U13.

- **V1 hub-native orchestration (U11–U12, C10–C12), foundation delivered.**
  Orchestrate can **Create** or **Load** a named team chat and focus Chat &
  Memory on that session (`localStorage` keeps the choice across hub polls).
  The composer addresses all / a subset / one session member and can mark
  posts **task** and/or **wake**. Task-tagged sends to non-members are
  refused; wake-tagged sends may enroll a new identity. Tagged delivery
  goes through `hub_send_tagged_message` / `ca msg tag` and, when accepted,
  `hub_inject_harness` or `ca msg tag --dispatch` (explicit argv, no shell,
  no TUI attach). Active-session execution remains gated on provider-supported
  bridge transports.
- **C12 four-harness capture.** Disk adapters record assistant text from
  Grok (`~/.grok/sessions/<pct-workspace>/<id>/chat_history.jsonl`), Claude
  Code (`~/.claude/projects/...jsonl`), Codex (`~/.codex/sessions/...`), and
  Gemini/Antigravity (`transcript.jsonl`). Each adapter keeps a **disk
  session id** separate from the **hub work-session id**. Duplicate polls
  are content-hash deduplicated. Chat & Memory refresh polls all four and
  reloads the transcript when new rows appear. Headless: `ca harness capture
  --harness grok|claude|chat|gemini --workspace PATH`.
- Live named-session check on this checkout (throwaway HubStore, no write
  to `~/.coding-assistants`, no spawn): task-tagged send to enrolled `grok`
  accepted, outsider rejected; disk capture found grok 11 / claude 52 /
  chat 25 / gemini 247 assistant rows. C13 is Harbinger running that loop
  in the app; the markdown bus stays as fallback until then.
- C13 migration gate is specified (C12 live acceptance, named-session
  assign/review/capture, reconstruct from hub records only, #113 evidence).
- `infra/supabase/supabase_config.js` scaffold for later S11 Auth + Storage
  (no sync implementation).
- Website docs data (`docs/website/src/data/docs.json`) regenerated from
  the moon roadmaps (U11–U12 / C10–C13).

### Fixed

- **Orchestrate continuity.** The selected Workspace Root is now persisted in
  browser storage, and the Orchestrate roster rehydrates persisted Hub team
  membership after an app restart. **Detect running agents** is now a visible
  toggle: after scanning it becomes purple **Hide detected agents** and hides
  the discovery panel when clicked again. Process detection explicitly
  distinguishes finding a local process from having a supported live-delivery
  bridge.
- **Tagged task delivery and Gemini adapter:** task-only posts no longer
  launch an unexpected replacement Grok/Chat/Claude/Gemini CLI process; they
  remain in the durable session inbox and explicitly report that an active
  harness adapter must consume them. Explicit wake posts retain spawn
  behavior. Gemini/Antigravity now launches the installed `agy` executable,
  rather than the nonexistent `gemini` binary.
- **Explicit team enrollment:** new hubs now start with the human owner only.
  An untouched legacy default roster is migrated the same way, so agents must
  be deliberately added to the team/session before task delivery is accepted.
- **C12 partial injection failure visibility:** Chat & Memory now waits for
  every selected harness injection and reports each rejected/unavailable
  delivery in the existing owner alert. A durable session post is not hidden
  behind one rejected IPC call.
- **C10–C12 Tauri session-send payloads:** nested tagged and ordinary session
  arguments now deserialize their Chat & Memory camelCase fields, preventing a
  tagged send from failing with a missing `is_task` field before dispatch.

### Gemini — v1 hub-native orchestration UI & recipient tag controls (2026-08-13)

- **U11 Create and Load Team Chat Entry Points**: Added dedicated Create Team Chat and Load Existing Team Chat controls to Orchestrate (`ConfigPanel.tsx`), which automatically set active work session and focus the Chat & Memory window (`App.tsx`).
- **U12 / C10 Recipient Selection & Intent Tags**: Added Recipient Mode controls to Chat & Memory composer (`SlackChatPanel.tsx`) supporting `🌐 All Team Members`, `👥 Subset` (interactive agent checkboxes), and `🎯 Single Agent` (dropdown), along with `⚡ [TASK]` and `🔔 WAKE` intent tag toggles.
- **C11 Task Tag Team Member Validation**: Implemented pre-flight validation preventing task-tagged messages from targeting non-enrolled agents, ensuring tasks target existing team members while wake-tagged messages can trigger or spawn new agent instances.
- **Transcript Intent & Recipient Badges**: Added visual badges to transcript message bubbles displaying `⚡ TASK`, `🔔 WAKE`, and `To: <recipient>` header metadata.

### Chat / Codex — v1 orchestration work-review draft (2026-08-13)

- Reviewed Grok's U11–U12 / C10–C13 hub-native orchestration plan and filed
  the six implementation issues (#108–#113), each linked to its roadmap row
  and the v1 gate. Implementation of U11–C12 has since landed; C13 is the
  owner live loop.

### Grok — v1 hub-native orchestration spec (2026-08-13)

Specified the remaining work to run the whole team from the CA app instead
of per-repo `.agent` markdown. New roadmap slices:

- **U11** Orchestrate **Create** and **Load** team chat (create already exists)
- **U12** Composer: all / subset / one, plus optional task and wake tags
- **C10** Same addressing from human and enrolled agents
- **C11** Wake may spawn a new instance that joins the team; task must target
  an existing member
- **C12** Capture harness-side messages and inject tagged hub messages
- **C13** Retire `AGENT_BUS.md` as the live protocol once C10–C12 work

Chat files the GitHub issues. Implementation order is U11, then C10+U12,
then C11, then C12, then C13.

### Claude session (2026-08-13)

- Usage tab no longer labels every successful provider quota fetch as **live
  quota**. Only Codex (`chat`, via `codex app-server` rate limits) and Grok
  (`grok`, via its billing snapshot) genuinely re-query a live process/API on
  every call, so they keep the "live quota" badge. Every other provider
  (Claude, Gemini/Antigravity, and any future harness lacking an official
  usage-budget command) now shows **last refreshed `<date-time>`** derived
  from `ProviderQuota.fetched_at`, plus a per-provider **Refresh** button, so
  the displayed numbers don't silently go stale between window opens.
- Added a **Refresh all stale quotas** button that re-fetches every non-live
  provider (everything except `chat`/`grok`) in one action.
- New backend command `hub_refresh_provider_quota(agent_id)` dispatches to
  the matching per-provider adapter so the frontend can refresh one provider
  without re-fetching the rest.
- **Disclosure**: while wiring this up, found `gemini_quota()` currently
  returns **hardcoded/fabricated window data** (fixed 66%/0%/100%/100% used
  percentages) — only its reset countdowns are computed relative to "now". It
  is not a one-time-stale snapshot, it was never live at all. The refresh
  button now at least lets you re-fetch it, but a genuine reverse-engineered
  Antigravity CLI usage-budget adapter (mirroring the Claude Code work below)
  remains open work, tracked under #86.

### Gemini session (2026-08-13)

- Added support for **Google Antigravity CLI** usage limit plots in Shared Hub (`Usage` tab) with dedicated sub-groups for **Gemini Model Family** (weekly limit & 5-hour limit remaining) and **Other Model Families** (Claude & GPT models in Antigravity).
- Expanded provider quota charts to group and display harness titles (`Anthropic Claude Code`, `xAI Grok Build`, `OpenAI Codex`, `Google Antigravity CLI`, `Anomaly Opencode`, `Local Llama.cpp`, `Local Ollama`) with model family subtitles (`Claude Model Family`, `Grok Model Family`, `Chat Model Family`, `Gemini Model Family`, `Other Model Families`).
- Endorsed `cloud_sync.md` architecture design with zero-trust local-first encrypted replica model, mutation-only Hub lock during sync runs, and 30-day manual conflict retention.

### Shared Hub consolidation (2026-08-13)

- Renamed the desktop conversation surface to **Chat & Memory**. It is now the
  single user-facing location for team messages, agentic memory, and
  `#wakes-alerts`; the duplicate Shared Hub **Inbox**, **Memory**, and
  **Wakes** navigation entries were retired.
- Wake-policy controls now apply their selected value optimistically while the
  persisted policy update is in flight, reverting only on an error. Their
  custom checkbox treatment uses a bright checked state, tick, border, and
  focus halo so selected and unselected policies are plainly distinguishable.

### Grok session (2026-08-12 → 2026-08-13)

Lead-orchestrator pass on the Slack-like hub after Harbinger's GO (M6 first,
then prove the team loop). Commits authored or co-authored in this stretch:

| Commit | What |
| --- | --- |
| `525f07c` | Persisted `agents.team_member` roster; team send includes Harbinger and excludes PID identities |
| `9655e7d` | Slack/Orchestrate team send wakes every enrolled member, not only `chat` |
| `c92accf` | `HubStore::request_team_wakes` |
| `f16e862` | Slack thread no longer creeps down while reading older messages |
| `0dc2f1b` | `ca agent enroll\|unenroll\|team`, `hub_set_team_member`, header **Local hub online** pill |
| `947a43d` | Enter-to-send, Shift+Enter newline, **Jump to latest** |
| `2ab31c7` | Slack DMs send only to that agent |
| `f9e255b` | Shared Hub Usage plots Grok's weekly pool (`creditUsagePercent`) and extra-usage credits from the TUI `/usage` billing snapshot |

Delegated (not claimed as Grok implementation): CA-106/109/110/111 to Claude,
CA-102 channel queries to Chat. M6 live seed is in `~/.coding-assistants`
(`ca memory search M6-20260812`); Claude ACKed. Board: #82 and #80 closed;
U10 follow-through tracked as #90 (closed after CA-106/109/110/111). Grok
Slack spine commits for #90: `525f07c`, `9655e7d`, `c92accf`, `f16e862`,
`0dc2f1b`, `947a43d`, `2ab31c7`. Wakes increment: #81 (still open).

**Cloud sync (2026-08-13):** Grok wrote the first `cloud_sync.md` draft from
the owner Q&A; Claude's second Q&A and `743000a` finalized it as the
approved S1–S13 plan. Grok review: agree, with S6 rebase-test and S10
no-key-envelope caveats recorded in that file. Issues #91–#103.

**Claude, 2026-08-13:** #82 and #80 are now closed. CA-106/109/110/111
(`2064a59`, `09d3533`, `bec7454`, `ca40e46`) shipped and are tracked/closed
as issue #90, since U10 had no prior issue of its own.

**U8 quota (2026-08-13):** Shared Hub Usage now plots live provider quota
windows for Codex, Claude, Grok, Gemini/Antigravity, and the remaining
configured harness families. This completes the agreed usage-limit scope for
#86.

### Fixed

- Slack channel badges now count **unread** posts only, using the same
  membership as the thread (including unprefixed `#general` history).
  Opening a channel marks those posts read. Existing history is not treated
  as unread on first launch, which is why `#general` had no number while
  `#team-coordination` showed a stale total.

- CA-106's Edit/Delete menu only opened via right-click, with no visible
  affordance a first-time owner would discover. Added a hover-revealed
  **⋯** actions button on the owner's own message bubbles (opens the same
  menu as right-click, at the click point); the bubble the open menu targets
  gets a highlight ring. Also fixed opening a *new* message's menu while
  another's was already open immediately self-closing — the click bubbled to
  the still-mounted `window` listener left over from the previous menu.
  `e.stopPropagation()` in `openMessageMenu` fixes both the new button and
  the pre-existing right-click path.

- Slack **Direct Messages** now send only to that agent. Opening a DM no
  longer keeps "Broadcast to Team" as the recipient, so a private thread
  cannot fan out to the whole roster.

- Slack composer sends on **Enter**; **Shift+Enter** inserts a newline.
  While reading older messages, a **Jump to latest** chip appears instead of
  yanking the viewport.

- Header chrome no longer shows a second Slack-looking control. The purple
  **Slack Multi-Agent Hub** badge is now a green **Local hub online** status
  pill so it cannot be mistaken for another Slack tab.

- Simplified the **Orchestrate** window to team/role configuration (including
  workspace and MCP settings) plus Remote Control. Its duplicate composer,
  Team Chat, and Messages feed were removed; Slack Chat & Memory is now the
  single desktop surface for human/agent communication.

- Slack `#general` no longer creeps downward while Harbinger reads older
  messages. The 1.5s hub poll was calling `scrollIntoView({ behavior:
  "smooth" })` on every refresh. The thread now stays put unless the view
  is already near the bottom, the channel changed, or Harbinger sent a
  message. Team fan-out copies of one post are shown once.

- Slack/Orchestrate team sends now wake every persisted roster member
  (`hub_list_agents` + `hub_request_wake`) instead of only `chat`. Direct
  messages still wake the selected recipient. The Slack DM list follows
  `team_member` enrollment and role labels match the Grok-lead / Chat-co-lead
  split.

- Team broadcasts (`ca msg send --to team` and `hub_send_message` with
  `to: "team"`) now fan out only to agents with persisted `team_member = 1`.
  The default roster is Harbinger (`human`) plus `claude`, `chat`, `gemini`,
  and `grok`. Process-discovered PID identities and local model runtimes stay
  privately addressable. `cargo test -p hub` (12 passed).

- Fixed the Slack Chat window going blank a few hundred milliseconds after
  first paint. `SlackChatPanel.tsx` declared its own `DetectedProcess` shape
  (`agent_id`, `executable`) that never matched what `detect_agent_processes`
  actually serializes (`agent`, `provider`, `model`, `command`, `pid`) —
  `ConfigPanel.tsx` already had the correct shape. As soon as the 4s presence
  poll found a real running agent process, `p.agent_id.toLowerCase()` threw
  on `undefined` inside `getAgentInfo` (called during render for every
  message and the DM roster), and with no error boundary React 18 unmounted
  the whole tree. Aligned the interface and the running-process match logic
  with the real backend shape.

### Added

- Claude Code now reports **live session/weekly/monthly-credit quota
  windows** in the Usage tab, matching Chat's Codex quota bar. Anthropic
  publishes no stable API for a subscription's usage-limit percentages
  (`anthropic-ratelimit-*` headers are a separate, per-API-key billing
  concept); found the endpoint the official `claude` CLI itself calls to
  render `/usage` by driving an interactive `claude --debug` session and
  reading its debug log, then verified directly against it —
  `GET api.anthropic.com/api/oauth/usage` (Bearer-authenticated with the
  OAuth token from `~/.claude/.credentials.json`). It is undocumented and can
  change on any Claude Code update with no notice; every failure path
  (not logged in, expired token, network error, unrecognized response
  shape) degrades to the existing "unavailable" state rather than
  crashing. The "Usage credits" monthly window has no `resets_at` in the
  response, so its reset date is computed locally as the 1st of next
  month UTC, matching the desktop `/usage` UI's own wording.

- Work sessions: Orchestrate can create a named durable work-session chat.
  It starts with the current persisted team and adding an eligible agent to the
  team enrolls it in the active work session. Slack Chat & Memory lists each
  session, renders human and agent-harness messages in its
  `channel:session:<id>` scope, and sends only to that session's members.
  Per-member checkboxes decide which offline agents receive a wake for the
  next session message without changing durable delivery.

- Cloud Drive sync capability roadmap (`docs/moon/roadmaps/cloud_sync.md`,
  `743000a`): Google Drive first, then Firebase Auth+Storage, then Supabase,
  then OneDrive/Dropbox. Encrypted replica, journal-integrity merge, hashed
  remote names, confirm-only deletes, CLI `ca sync`, Hub mutation lock.
  Implementation issues S1–S13 (#91–#103).

- CA-114: channel messages can now be replied to in context. Replies retain a
  stable root message in the existing `channel:<name>:thread:<root>:<id>`
  subject namespace, so they remain isolated to the channel and keep normal
  team fan-out/wake behavior. The Slack composer shows the selected parent
  with a cancel control, while rendered replies identify their parent without
  requiring a schema migration.

- CA-106: right-click **Edit** / **Delete** on Slack message bubbles
  (`SlackChatPanel.tsx`). Only Harbinger's own posts (`from_agent ===
  "human"`) show the menu; `hub_update_message` / `hub_delete_message`
  enforce the same rule server-side. Team/channel broadcasts are N SQLite
  rows sharing a subject, so both commands resolve and mutate every sibling
  copy via `hub::update_broadcast` / `delete_broadcast` — new posts group
  by the exact `channel:<name>:<uuid>` subject, legacy posts fall back to
  `(from_agent, body, subject, created-at-to-the-second)`. Delete is a soft
  cancel (`status = cancelled`); the Slack view hides cancelled rows while
  the audit trail (`hub_list_messages`) still returns them. `cargo test -p
  hub` (15 passed).

- CA-109: CLI parity for CA-106 — `ca msg edit --id <uuid> --from human
  "body"` and `ca msg delete --id <uuid> --from human`. Rejects any `--from`
  other than `human`, and independently verifies the target message's
  `from_agent` is `human` before mutating, matching the desktop
  `require_human_authored` check. Both commands resolve every sibling copy
  of a team/channel broadcast via `update_broadcast`/`delete_broadcast`.

- CA-110: Orchestrate role/process cards and the detected-process list now
  show **Remove from team** once enrolled, calling `hub_set_team_member`
  with `enrolled: false` for the same stable ids (`chat`, `claude`,
  `gemini`, `grok`) `Add to team` persists. `human` is never unenrolled, and
  removal never invents a PID-based roster row.

- CA-111: added a desktop **Journal** tab (`HubPanel.tsx`) surfacing pending
  audit events (`ca audit watch` output) at the owner checkpoint — fetched on
  first load (tab badge shows the pending count) and every time the tab
  opens, with **Approve** / **Quarantine** actions. New Tauri commands
  `hub_list_audit_events`, `hub_approve_audit`, `hub_quarantine_audit` wrap
  the existing `hub::HubStore` audit API; no new privileged adapter.

- Persisted Orchestrate **Add to team** onto the Slack roster for stable
  harness ids (`chat`, `claude`, `gemini`, `grok`). CLI: `ca agent team`,
  `ca agent enroll --id`, `ca agent unenroll --id`. Tauri:
  `hub_set_team_member`, `hub_list_team_members`, `hub_request_team_wakes`.

- Added CA-102 channel and memory-reference queries across the shared Hub:
  `ca msg channel <name> [--limit N]`, `ca msg memories <message-id>`, and
  Tauri `hub_list_channel_messages` / `hub_list_message_memories`. Channel
  lookup is exact (`channel:<name>`, with colon-delimited thread metadata)
  and bounded; `[Memory #<full-id-or-unique-prefix>]` tags resolve only when
  they identify one durable memory, preventing ambiguous cross-references.
  Store and Tauri command coverage verifies both channel isolation and linked
  memory resolution.

- Added dedicated **Slack-like Multi-Agent Chat Interface & Agentic Memory Hub** (`SlackChatPanel.tsx`). Features channel sidebar (`#general`, `#team-coordination`, `#agent-memory`, `#wakes-alerts`, DM channels), agent status indicators, real-time message stream with Slack formatting, and an expandable Agentic Memory Hub side drawer.
- Established Lead Orchestration task allocation across Gemini (Lead Orchestrator), Grok (Build), Chat/Codex (Chat), and Claude (Code) on `.agent/cache/AGENT_BUS.md` and per-agent delegation files in `.agent/messages/`.

- Private hub messages addressed to Harbinger now display their contents in
  the Messages feed; private messages addressed to another participant remain
  redacted in the shared team view.

- Fixed Orchestrate message delivery to use stable harness identities (`chat`
  for Codex/ChatGPT) instead of detected process IDs, and added Team/private
  recipient routing. Each sent message now also creates a linked hub wake so
  the receiving harness has an explicit signal to poll its inbox. Team
  broadcasts fan out with a shared marker; private messages remain visible as
  sender/recipient metadata without their bodies in the team chat.

- Centralized Tauri invocation behind a runtime guard. Browser/Vite mode now
  reports a clear desktop-runtime requirement instead of throwing an undefined
  bridge error, and Tauri event listeners are skipped outside the desktop app.

### Added

- Shared Hub Usage now plots Grok's weekly subscription pool the same way
  it plots Codex windows. The adapter reads the Grok CLI session token from
  `~/.grok/auth.json` (never logged) and fetches the same
  `GET /v1/billing?format=credits` snapshot the TUI `/usage` command uses,
  then maps `creditUsagePercent` / `billingPeriodEnd` onto a Weekly bar.
  Extra usage credits (`onDemandUsed` / `onDemandCap`) appear as a second
  bar when present. Quota rows are limited to the four harnesses so PID
  identities no longer clutter the chart.

- Added provider-quota plots to the Shared Hub Usage tab. Codex is queried
  through its local app-server account rate-limit endpoint and displays each
  reported window's used/remaining percentage and reset time; providers that
  do not expose a local quota snapshot are explicitly marked unavailable
  instead of being confused with local Shared Hub budgets.

- Added the first agent-session bridge boundary: `ca inbox watch --agent
  <id>` emits JSONL `ready`/`message` records, polls the durable inbox,
  acknowledges delivered messages, and resolves linked wakes. Human-gated
  wakes require explicit `--accept-gated`; `--forward PROGRAM` can pipe the
  same stream to a long-lived provider adapter. Added a Codex adapter using
  the installed app-server `thread/resume` + `turn/start` protocol rather than
  terminal injection, plus `--list-threads` discovery for selecting a
  persisted session. Completed assistant messages are now published back to
  the hub as replies from `chat`; attaching to an already-running interactive
  TUI remains intentionally unsupported because it has no exposed app-server
  control channel.

- Connected Orchestrate to the shared harness inbox: messages sent by Harbinger
  use `hub_send_message` directly instead of launching an OpenCode task, while
  persisted agent and private messages are refreshed into the chat with sender,
  recipient, kind, and status labels.

- Reframed Orchestrate as a session team chat: **Execute Task** is now
  **Write Message**, **Launch Sequence** is **Send Message**, agent events are
  displayed as sender-attributed messages, and configured spawned agents have
  explicit **Add to team** controls. Enrolling a detected existing process now
  immediately adds its participant and a join message to the chat.

- Added the audit integrity MVP to `hub` and `cli`: recursive filesystem
  observation via `ca audit watch`, durable pending change records, owner
  approve/quarantine actions, and SHA-256 chain verification via `ca audit
  verify`. User-space observation records the watcher context and documents
  that originating external-writer PID attribution requires a privileged
  adapter.

- Added per-role existing-process endpoint configuration. When an
  OpenAI-compatible endpoint is supplied, orchestration sends requests to the
  already-running model service and does not spawn or terminate a child
  process; blank endpoints preserve the existing provider-managed behavior.
- Added a **Detect running agents** control to Orchestrate. It discovers local
  Grok, Claude, Codex/ChatGPT, and Gemini/Antigravity command processes and
  lets the user add selected identities to the configured team without taking
  ownership of or terminating those processes.
- Tightened process discovery to match executable basenames only, excluding
  desktop helpers, Chromium/Node utility services, and agent runtime helpers.
- Refined Gemini detection to recognize `agy` and the legacy `gemini` CLI while
  excluding the `antigravity` IDE executable.
- Improved maximized-window scrolling by removing permanent compositor-layer
  promotion from full-page panels and allowing offscreen sections to be skipped.
- Added a large-window performance profile that reduces full-surface gradients,
  card/button shadows, and header backdrop filtering while preserving layout and
  colors.
- Renamed the Shared Hub Budget tab to Usage and added per-agent used/available
  budget charts.
- Made startup resource discovery read-only so the default workspace is not
  created until the user explicitly initializes or runs a task.

- **Dashboard telemetry slice:** added persisted `agent_metrics` counters for
  provider calls, output lines/chars, estimated tokens used, and cached tokens;
  added Shared Hub Dashboard cards with per-agent budget progress. Exact
  provider token/cache/cost/latency reporting remains follow-up work.
- Dashboard now includes a collaboration overview sourced from existing task,
  message, and wake records, including pending-wake counts and recent tasks.

- **C6 done:** `agent_budgets` table + `HubStore::set_agent_budget` /
  `record_budget_usage` / `resume_agent` / `pause_for_budget`. Crossing a
  budget's `limit_units` flips `paused` (caller-defined units — call count,
  USD, tokens, whatever the caller tracks); `pause_for_budget` writes a
  Markdown handoff summary (objective/completed/missing) under
  `markdown/handoffs/`, sends a durable `Handoff` message to the delegate
  (default `"human"`), and `request_wake` then rejects the paused agent until
  a human calls `resume_agent`. Wired through `ca budget
  set|status|spend|pause|resume` and Tauri `hub_set_agent_budget` /
  `hub_get_budget` / `hub_record_budget_usage` / `hub_resume_agent` /
  `hub_pause_for_budget` (desktop UI wiring still open). Covered by
  `c6_budget_exhaustion_pauses_writes_handoff_and_blocks_wakes`.

- **2026-08-11 memory/communication hub slice (M1–M5, C1–C5):**
  - `hub`: promote/compact/delete, purge-stale, age-out short-term; wake
    pending **dedup**; wake resolve; standing `WakePolicy` (human-gate defaults);
    message status updates; Markdown export includes handoffs;
    **`export_markdown_git`** (M3); **`tasks`** with sequential stages,
    bounded-parallel groups (`parallel_group` + `max_parallel`), and
    per-stage **retries** (`max_retries` / `retry_task`) (C5).
  - `ca` CLI: `memory promote|delete|compact|purge-stale|age-out`,
    `msg status`, `wake resolve|policy`, `export-markdown --commit`,
    `task create|advance|complete|retry|list|get|cancel`.
  - C6 budget controls: `ca budget set|status|spend|pause|resume`; exhausted
    agents are blocked from new wakes and produce durable Markdown handoffs.
  - Tauri `AgentSystem` now checks configured budgets before provider calls and
    records one call unit after successful completions, invoking the handoff
    boundary when a role exhausts its budget.
  - Active-run cancellation now records a durable shutdown handoff and
    delegation message before the Tauri task exits.
  - Shared Hub now includes a Usage tab for configuring limits, recording
    usage, inspecting paused agents, and resuming them.
  - C4 task-level `require_human_approval` is persisted and exposed through
    CLI/Tauri workflow creation, with coverage for ungated task wakes.
  - Added atomic pre-call budget reservation via `ca budget consume` and
    Tauri `hub_consume_budget`; over-limit provider calls are rejected before
    they start.
  - Tauri `hub_*` IPC + React **Shared Hub** panel; Orchestrate UI split into
    `ConfigPanel`/`ActivityPanel`/`RemotePanel`/`ApprovalPanel`.
  - Shared Hub **Policy** tab added for managing standing `WakePolicy` (human gate defaults);
    Wakes panel resolves pending wakes as delivered.
  - **C7 done:** Implemented A2A-compatible discovery and horizontal delegation. `AgentCard` schema and storage were added to `hub`. `ca agent register-card` was added to `cli`. The Tauri API exposes `hub_upsert_agent_card` and the TCP server now handles `GetAgentCards` payloads, enabling local workflows to interoperate with A2A peers.
  - **U3 done:** Implemented `update_memory` in `hub` store and added inline editing
    along with color-coded scope indicators to the Shared Hub Memory tab.
  - **U2 done:** Added Task Browser tab to Shared Hub, allowing users to view task history,
    metadata, and message/handoff transcripts.
  - **U5 done:** Added DashboardScreen to Android app for viewing events and approving/rejecting wakes via TCP.
  - **U6 done:** Implemented Project Creation Wizard via a `bootstrap_workspace` Tauri command
    and a button in the ConfigPanel to initialize `.agent/` skeletons for new workspaces.
  - **C4 done:** Implemented per-task delegation policies via `require_human_approval` on
    `TaskRecord`, enabling configurability for automatic wakes during task dispatch, accessible
    through both the `cli` (`--require-approval`) and the Tauri API (`CreateTaskArgs`).
  - **C6 done:** Exposed shutdown hooks via `ca shutdown` in the CLI and `hub_record_shutdown`
    in the Tauri API. This completes the budget exhaustion and shutdown delegation milestone,
    allowing external adapters to properly persist handoff states upon cancellation or limit reach.
  - Install: `just install-ca` / `~/.local/bin/ca` documented in `crates/README.md`.
  - Unit tests: promote/compact, wake dedup/policy, M3 git export, M6 handoff
    acceptance, and C5 sequential plus bounded-parallel/retry workflows.
  - **U1 done:** Refactored `App.tsx` into decoupled components (`ConfigPanel`,
    `ActivityPanel`, `RemotePanel`, `ApprovalPanel`) and overhauled the UI with
    a stunning glassmorphism design and micro-animations.
- Added the first executable M6 acceptance flow covering a durable handoff,
  provenance-linked memory, cross-agent inbox retrieval, wake resolution, and
  Markdown export; a real multi-agent repository run remains.
- **M3 done:** `HubStore::export_markdown_git` runs `git add` + `git commit`
  on the Markdown export when its directory is inside a Git work tree; outside
  a repo, with a failed `git add`, or with nothing to commit, it returns a
  `GitExportOutcome { committed: false, detail }` instead of erroring. Wired
  through `ca export-markdown --commit [--message ...]`, the Tauri command
  `hub_export_markdown_git`, and a desktop "Export MD + Commit" button. Covered
  by `m3_export_markdown_git_commits_inside_a_work_tree` (spins up a real
  temporary Git repo).
- PMF VS10 pivot recorded in the agent coordination bus; baseline frontend and
  Rust workspace checks passed before this implementation began.

- Hub spine crates (`crates/hub`, `crates/cli` binary `ca`): SQLite
  agents/memories/messages/wakes, private journals, wake JSON side-channel,
  Markdown export, CLI commands for init/memory/msg/wake/journal/export;
  unit test covering M1/C1–C3 smoke path.

- Replaced language-oriented roadmap files with capability roadmaps for memory,
  communication, UI, dashboards, platform, and infrastructure. Added a Mermaid
  Gantt index, made private agent journals part of the first memory milestone,
  retained LAN and Firebase prototyping, promoted A2A to the next major
  milestone, and removed obsolete deployment scaffolding. Deleted the duplicate
  `docs/ROADMAP.md` and the superseded per-language roadmap files.

- Reoriented the roadmap around the owner-confirmed product identity: a
  personal, local-first collaboration hub. Added the priority
  the initial shared memory and coordination roadmap for
  SQLite/Markdown hybrid memory, durable handoffs, CLI access, wake signals,
  configurable policies, and external-agent adapters. Folded the former root
  feature checklist into the moon roadmaps, kept `docs/ROADMAP.md` as a pointer
  stub at that stage, demoted TUI/3D/GraphQL-first/early actors to
  Someday/Maybe, and
  recorded provider, security, testing, and infrastructure-hygiene follow-up.

- Roadmap implementation, batch 2 (the former daemon-extraction spike):
  completed the daemon-extraction spike as [ADR 0003](../adr/0003-daemon-extraction-spike.md).
  Measured the actual `tauri::AppHandle` coupling across the backend
  (`file_tools.rs`: none; `agents.rs`/`llm_client.rs`: event emission only;
  `tcp_server.rs`: event emission + listening) and decided against
  extracting a separate daemon crate yet — recommends decoupling event
  emission into an internal broadcast channel first (tracked as new item
  `P1`), deferring the physical crate split until the API boundary is clear.
- Roadmap implementation, batch 1 (rate limiting, async file I/O, and shell
  audit): per-provider token-bucket rate limiting on
  outbound LLM calls (`governor`, burst 3 / 1 per second) in
  `llm_client.rs`; converted `FileTools` and the remaining Tauri-command
  file I/O to `tokio::fs` so it no longer blocks async worker threads;
  audited the tool layer for raw shell execution (none found — confirmed
  compliant with `AGENTS.md`'s Security Notes). Also removed `lib.rs`'s
  dead `start_remote_server` command (superseded by `TcpServer`, and it
  hardcoded a machine-specific absolute path that doesn't exist in this
  repo).

- Synced repo scaffolding/tooling from the Tauri-App-Template layout (excluding
  its backend/frontend/middleware/notebooks directories, which don't apply to
  this repo's existing `src/`/`src-tauri/`/`android/` layout): `.agent/`
  cross-agent delegation docs, `.devcontainer/`, `.forgejo/`/`.gitea/`/`.gitlab/`
  CI mirrors, expanded `.github/` automation, `git/` repo-process tooling
  (hooks, backlog sync, label taxonomy), the original `infra/` scaffolding,
  `tools/*/justfile` + root `justfile`, `settings/` editor configs, and
  `docs/` additions (ADRs, `docs/moon/`, Structurizr C4 model, `docs/website/`).
- Moved root-level docs (`ARCHITECTURE.md`, `DEPENDENCIES.md`, `DEVELOPMENT.md`,
  `TESTING.md`, `TROUBLESHOOTING.md`, `TUTORIAL.md`, `ROADMAP.md`,
  `SECURITY.md`, `CHANGELOG.md`) into `docs/`.
- Moved `codecov.yaml`/`CONTRIBUTING.md` from `.github/` into the new `git/`
  directory.
- Expanded the former per-area roadmaps with target-architecture work items
  synthesized from
  `docs/moon/research/Multi-Agent AI App Architecture.md` and
  `docs/moon/reports/AI Coding Tools Feature Report.md`: a headless Tokio
  actor-model daemon, a GraphQL-over-WebSockets API, MCP + A2A protocol
  support, rate limiting + affine-typed budget guardrails, two-tier
  persistent memory, human-in-the-loop security gates, a 2D telemetry
  dashboard, 3D force-graph visualization, and a new Ratatui TUI
  (now reorganized under the capability roadmaps). Added a capability-order
  index and Mermaid Gantt to `docs/moon/ROADMAP.md`.

## [0.1.0] — 2026-07-30

### Added

- Repository created from scratch.

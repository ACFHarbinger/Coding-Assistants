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

### Added

- **Discord v1 chat bot remote client (U25, #319):** new `ca discord`
  command tree (`pair --channel <id>` / `status` / `unbind [--user-id]` /
  `run [--interval-secs]`) mirroring the Telegram remote-control contract —
  one-time 10-minute pairing codes, a persistent allow-list
  (`discord_binding.json` next to `hub.db`), the same `/start` `/approve`
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

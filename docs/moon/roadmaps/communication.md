# Agent Communication and Delegation Roadmap

Communication starts with explicit, declarative task wiring and asynchronous
mailboxes. Parallel execution and A2A follow only after durable local
communication is reliable.

> **Priority note (Harbinger, 2026-08-14):** the developers being onboarded
> (see [`multi_human.md`](multi_human.md), timeline relaxed the same day)
> use a single agent each, not multi-agent orchestration — at least one has
> Claude, another has Gemini. Reprioritizing C14 accordingly: **finish C14.3
> (Claude Code) end-to-end acceptance first**, **C14.4/C14.7 (Gemini `agy`,
> including the #155 `--prompt` bug) second** — those two providers are
> enough to validate the team-facing features (messaging, task assignment,
> channels/bridges, memory) that actually matter for this onboarding.
> **C14.2/C14.8 (Codex) and C14.6 (Grok) are explicitly deprioritized**: pick
> them up only if a fix is fast/cheap, otherwise skip for now rather than
> spending time reverse-engineering a CLI contract nobody being onboarded is
> using yet.

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| C1 | Agent identities, attribution headers, durable inbox/outbox messages, and handoff records | Every message records sender, receiver, task, workspace, timestamp, and status | ✅ **Done** · `ca msg send/poll/list/status` + seeded agents + handoff kind in MD export |
| C2 | Shared `ca` CLI for read/write/search/poll operations | External agent loops can use it without the desktop UI | ✅ **Done** · binary `ca`; `ca agent team\|enroll\|unenroll`; also mirrored by Tauri `hub_*` commands / HubPanel |
| C3 | Separate ephemeral wake mechanism via file watch or local socket | Durable writes survive absent agents; wake requests are observable and **deduplicated** | ✅ **Done** · `wake/*.json` + SQLite; pending dedup by target/message/reason; resolve delivered/cancelled |
| C4 | Configurable human gates and standing policies for wake-ups and delegation | Per-task policy can allow or require approval | ✅ **Done** · persisted `WakePolicy` integrated into desktop Shared Hub Policy tab; per-task delegation policy via `require_human_approval` on `TaskRecord` |
| C5 | Declarative sequential and bounded-parallel workflow wiring | A real task can be split into plan/code/review boundaries with retries and handoffs | ✅ **Done** · stages via `parallel_group`; `max_parallel` queue; `retry_task`/`max_retries`; `complete_parallel_member`; CLI `task complete|retry` + Tauri (2026-08-11) |
| C6 | Budget exhaustion pause, Markdown handoff summary, delegation, and shutdown | No uncontrolled provider calls continue after a configured limit | ✅ **Done** · Tauri `AgentSystem`, CLI `budget consume`, Tauri commands, and Shared Hub Usage tab enforce configured limits; shutdown hooks exposed via `ca shutdown` and `hub_record_shutdown` |
| C7 | **Next major milestone:** A2A-compatible discovery, Agent Cards, and horizontal delegation | Local workflows interoperate with an A2A peer while preserving identity, approval, budget, and audit policy | ✅ **Done** · `AgentCard` schema and storage in `hub`, `ca agent register-card` in CLI, `hub_upsert_agent_card` in Tauri, and `GetAgentCards` over `TcpServer` |
| C8 | Fully parallel execution from session start | Concurrent work has conflict detection, task isolation, and deterministic recovery | 📋 Pending · later |
| C9 | Agent inbox bridge process | A long-lived adapter can consume one agent's hub messages as a stable stream, acknowledge them, and honor wake gates | 🚧 **In Progress** · Codex has `ca inbox watch` plus an app-server adapter; Grok has a provider-supported registered ACP leader path. Claude and Gemini can discover/capture active conversations but expose no documented safe attach transport, so their tasks remain queued. |
| C10 | Session addressing: all, subset, or one | Human and any enrolled agent can send a session message to every member, a named subset, or a single member. Non-targets are not woken or tasked. The session transcript records the explicit `to` list. | ✅ **Done** · #109. Session sends persist an exact recipient set by subject and reject non-members server-side; Chat & Memory routes all/subset/one through typed session/tagged commands. |
| C11 | Task vs wake message tags | A message may be tagged **task**, **wake**, both, or neither. **Wake** may launch a new harness instance of that identity and enroll it in the session team. **Task** must target an already-enrolled, currently present member and is refused (no spawn) otherwise. Agents can apply the same tags through the hub API/CLI. | ✅ **Done** · #111. `HubStore::send_tagged_message` + `hub_send_tagged_message` + `ca msg tag` enforce task-refuse and wake-enroll. Presence is team membership plus session membership when a session is given. A wake enrolls a missing team or session member. Mixed tags still refuse the whole recipient when the task check fails. Each recipient gets a durable `SendOutcome` including `policy_decision`. Untagged `ca msg send` / `hub_send_message` cannot use kind `wake`. |
| C12 | Bidirectional harness capture and inject | The app captures messages agents send inside Grok/Chat/Claude/Gemini harnesses into the session transcript. Hub messages tagged task and/or wake are injected into the target harness so the agent executes them. Builds on C9. | ✅ **Done (safe baseline)** · #145. Capture polls all four. Grok task delivery uses the registered ACP leader path. Codex/Chat task delivery uses documented `codex app-server` `thread/resume` + `turn/start` when a persisted thread is registered or found on disk; otherwise `unavailable` and queued. Claude and Gemini capture/discovery are real; their control transports stay `unavailable` and queued. Task-only inject never spawns a replacement process. No PTY writes or fabricated sockets. C14 is the provider-native managed-session follow-on. Fixed a subject-collision bug (`record_harness_capture` gave every captured chunk, from every harness/agent, the same fixed `channel:session:<id>:capture` subject; the desktop chat's per-post dedup collapsed them, making one agent's capture appear to overwrite another's — now uuid-suffixed, matching the same fix already applied to `record_channel_reply`). |
| C13 | Hub replaces the per-repo markdown bus | A full assign/review/task/wake loop completes with no writes to `.agent/cache/AGENT_BUS.md` or `.agent/messages/*`. Those files stay as a fallback until C10–C12 ship. `.agent` prompts/rules/skills remain resources, not the live protocol. | 📋 **Planned** · C12 accepted (#145). Substrate is built (C10/C11/C12 done). Step-by-step owner acceptance procedure lives in the owner's private release checklist (§ "Proof-of-Concept — Hub replaces the Markdown bus"); it wraps the #113 evidence template and `ca preflight` (#146, landed `06b29dd`). Live owner evidence is still required before the Markdown bus is demoted. |
| C14 | Provider-native managed harness sessions | Chat/Codex, Claude Code, and Gemini/Antigravity can each be deliberately launched, registered, messaged, observed, cancelled, and resumed through their supported contracts. Active sessions surface truthful readiness and delivery state; one provider writer owns each provider session. | 📋 **Planned** · Epic [#147](https://github.com/ACFHarbinger/Coding-Assistants/issues/147): supervisor [#148](https://github.com/ACFHarbinger/Coding-Assistants/issues/148), Codex [#149](https://github.com/ACFHarbinger/Coding-Assistants/issues/149), Claude [#150](https://github.com/ACFHarbinger/Coding-Assistants/issues/150) (done, live-verified), `agy` [#151](https://github.com/ACFHarbinger/Coding-Assistants/issues/151), UX/acceptance [#152](https://github.com/ACFHarbinger/Coding-Assistants/issues/152). Follow-on live-delivery fixes: Grok [#154](https://github.com/ACFHarbinger/Coding-Assistants/issues/154), `agy` [#155](https://github.com/ACFHarbinger/Coding-Assistants/issues/155), Codex [#156](https://github.com/ACFHarbinger/Coding-Assistants/issues/156), and durable repeated-send routing [#157](https://github.com/ACFHarbinger/Coding-Assistants/issues/157). Existing C12 capture and safe refusal remain the fallback until each provider reaches acceptance. |

### C14 provider-native integration contract

This is a managed-session programme, not permission to write bytes to an
existing terminal, socket, or provider-internal protocol. The owner explicitly
selects a harness/session for app ownership; ordinary discovered terminal
sessions remain observable only unless that provider exposes an opt-in,
documented bridge.

| Slice | Provider contract | Acceptance evidence |
| --- | --- | --- |
| C14.1 | Common session supervisor | Typed capability/readiness model, durable ownership and lifecycle audit, cancellation, output capture, and a per-session single-writer gate. The desktop/TUI shows **managed**, **observed**, **busy**, **queued**, or **unavailable** rather than claiming delivery. **In progress:** #148 has the durable observed/managed record and exclusive writer-lease foundation; supervisor commands, audit, cancellation, and UI remain. |
| C14.2 | Chat/Codex | Reuse a long-lived documented `codex app-server` connection per managed thread. Serialize `turn/start`; on an externally active writer retain the message as queued and show a retryable busy state, never race a second writer. **In progress:** [#149](https://github.com/ACFHarbinger/Coding-Assistants/issues/149) enforces the durable managed-session writer lease and turns active-writer errors into queued/retryable outcomes. `1633837` closes the blocking reply-capture gap: the disposable per-turn `codex app-server --listen stdio://` client now waits for the matching `turn/completed` notification and records its `agentMessage` text back to the original Hub sender (session replies use UUID-suffixed subjects). The installed app-server's generated bindings confirm `TurnCompletedNotification { threadId, turn }` and `ThreadItem::agentMessage { text }`; workspace build and Clippy are clean. This deliberately does **not** implement #149's persistent per-thread app-server daemon/broker: its control socket did not answer plain JSON-RPC in the live probe, so its framing and continuous streaming remain follow-up work. |
| C14.3 | Claude Code | Provide an opt-in Coding-Assistants Claude **Channel** MCP bridge. The bridge uses Claude Code's documented `claude/channel` capability to push authenticated Hub events into a session and a reply tool to return Claude output to the original Hub message/session. It supports the documented permission relay only after explicit human approval. Existing Claude sessions without that channel remain capture-only. **Implemented and live-verified:** [#150](https://github.com/ACFHarbinger/Coding-Assistants/issues/150) — `crates/claude` binary (renamed from `crates/claude-channel`) implements the stdio MCP server (`claude/channel` + `claude/channel/permission` capabilities, `reply` tool, permission-request relay); `hub::bridge::claude_channel` (new file, `bridge::claude`'s C12 path untouched) provides the authenticated-sender gate (enrolled team members only), reply routing, and a never-auto-approved permission lifecycle reusing the Hub audit chain. Opt-in setup registers the workspace as a C14.1-managed `claude` session and merges a `global.mcp.json` base layer plus a per-workspace canonical config — both stored durably under `~/.coding-assistants/servers/` — into the workspace's own `.mcp.json`. `--list`/`--rename`/`--delete` CLI subcommands and a Shared Hub "Channels" tab manage the registry. Only wake and task-tagged messages are pushed as a live-session interruption; plain chat/handoffs stay queued and are pulled on Claude's own initiative via a new `check_inbox` tool (`poll_channel_events`/`poll_quiet_channel_events`). The Channels tab now shows a live connected/not-connected status per workspace (`hub::is_channel_session_live`, a process-table check) and a Connect button that opens a real terminal running the `--channels` command (`hub::launch_claude_channel_session`) — Claude Code has no headless daemon mode, so this is always a real terminal, never a detached background process. End-to-end Claude Code acceptance (a real `--channels` session) was manually verified via ping round-trip. `b95baaf` adds an isolated `HubStore` round-trip covering the enrolled-sender gate, disturb/quiet poll split with ack-on-drain, reply routing to the original sender, and the explicit-only permission lifecycle; it compiles under `cargo check -p hub --tests` but was not executed under the machine's no-test-suite constraint. `hub::bridge::claude_channel` is now `hub::bridge::channels::claude::{workspaces,events,reply,permissions,terminal}` and `crates/claude/src/main.rs` is now `main.rs` (thin entry point) plus `main/{cli,protocol,server}.rs`, keeping every file ≤500 LoC ([#158](https://github.com/ACFHarbinger/Coding-Assistants/issues/158)); the crate-root public API (`hub::{poll_channel_events, ...}`) is unchanged. |
| C14.4 | Gemini / Antigravity (`agy`) | Start app-managed non-interactive workers with `agy --print --output-format stream-json` in the selected workspace; persist the conversation id and use documented `--conversation` only for a worker the app owns. Parse stream events into the Hub, support cancellation/status, and never pretend to attach to an unrelated interactive `agy` TUI. **Implemented** ([#151](https://github.com/ACFHarbinger/Coding-Assistants/issues/151)): `crates/hub/src/bridge/gemini.rs` runs the app-owned non-interactive worker (`gemini_managed_spawn_args`, `parse_agy_stream_line` for stream-json assistant text + conversation id, `acquire_harness_writer`/`release_harness_writer` single-writer serialization — unmanaged/observed C12 sessions stay capture-only and return `unavailable`, never a fake attach). Since `agy` exposes no live-push channel (only an undocumented internal gRPC protocol, not safe to reverse-engineer), C14.4/C14.7 also needed a different continuation mechanism than Claude's live MCP push: `crates/hub/src/bridge/channels/gemini/{mod,relaunch}.rs` implements kill → capture → relaunch instead — `kill_managed_agy_process` (SIGTERM, SIGKILL escalation), `resolve_gemini_continuation_id` (captured stdout → requested session id → registered session id → on-disk `latest_gemini_session_id` fallback chain), then `relaunch_and_deliver_gemini_task` restarts `agy --conversation <id>` so a Task/Wake genuinely continues the session rather than starting a disconnected one. `hub_register_managed_harness_session` auto-infers the continuation id when registering a managed Gemini session. `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` clean at time of writing (2026-08-14); Gemini's own environment additionally ran the full `cargo test --workspace` suite (214 passing) at landing time — this machine's standing no-test-suite constraint means that wasn't independently re-run here. |
| C14.5 | End-to-end UX and acceptance | Orchestrate and Chat & Memory create/select managed sessions, show setup prerequisites and actionable errors, and test all/subset/one task+wake routing, replies, cancellation, restart/recovery, permissions, and no-writer-race behavior on Kubuntu. **Desktop UX ready for review** ([#152](https://github.com/ACFHarbinger/Coding-Assistants/issues/152)): readiness badges, observed vs managed register, no fabricated `managed-<pid>` ids, retry/dismiss banners, Config/Messager/Channels split ≤500 LoC. Orchestrate's **Harness interfaces** panel can **Resume in terminal** via `hub_relaunch_harness_in_terminal` (kill optional managed pid, then open a real interactive CLI for Claude/Grok/Codex/Gemini — not the headless one-shot spawn). Live Kubuntu owner-run remains open. |
| C14.6 | Grok live-session delivery | Enable Hub-delivered messages to actually reach a live, human-attended Grok terminal. **Implementation ready** ([#154](https://github.com/ACFHarbinger/Coding-Assistants/issues/154)): `bridge::grok` keeps C12 ACP inject; `bridge::channels::grok` starts `grok agent leader` and a `grok --leader` TUI when the socket is missing. A Hub task ping reached a live Grok session. Standalone TUIs stay capture-only. |
| C14.7 | Gemini / Antigravity (`agy`) argv fix | Fix the real bug behind off-topic/gibberish `agy` replies to task/wake sends. **✅ Done** ([#155](https://github.com/ACFHarbinger/Coding-Assistants/issues/155)): the originally-diagnosed `--prompt <text>` bug was already fixed in `73d9af6` (positional prompt, argv-shape test added) — but the symptom was still reproducible. 2026-08-14 investigation against a **live `agy` call** found the real remaining cause: argument *order*, not shape. `--print --output-format stream-json <prompt>` (the prior order) makes `agy` misparse the prompt and reply about the `--output-format` flag instead of answering it; `--output-format stream-json [--conversation <id>] --print <prompt>` reliably works, verified for both a fresh and a `--conversation`-resumed call. `gemini_managed_spawn_args` reordered accordingly; tests updated/added. "Doesn't appear in the live session" remains expected/by-design for the headless worker adapter, not a bug. |
| C14.8 | Codex: explain silent delivery to an unregistered live session | Surface why a wake to a live, human-started Codex session that was never Hub-registered gets no visible response. **In progress** ([#156](https://github.com/ACFHarbinger/Coding-Assistants/issues/156)): Chat & Memory already renders the exact unavailable/queued detail in a high-contrast retry banner; the backend now canonicalizes persisted and selected workspace paths before finding a Codex thread, and tells the owner to register the persisted thread through Managed harness readiness. A separately opened Codex TUI remains observed-only: delivery uses documented app-server, never terminal injection. Long-lived app-server ownership and streaming remain C14.2. |
| C14.9 | Distinct durable records for repeated tagged sends | A repeated task/wake send to the same channel or session creates a distinct recipient set, outcomes, and transcript fan-out rather than colliding with a prior post. **In progress** ([#157](https://github.com/ACFHarbinger/Coding-Assistants/issues/157)): collision regression fixed and covered by an isolated CLI/Hub exercise; desktop acceptance remains part of C14.5. |
| C14.10 | DeepSeek native channel/bridge | Give DeepSeek a real provider-native managed session (analogous to C14.3's Claude Channel or C14.4's `agy` worker) instead of the current generic OpenCode-adapter path (`opencode run -m deepseek/<model>`, see `platform.md` P3). **Not started — deliberately sequenced after the #161–#163 + C13 ship-priority milestone.** Owner rationale (2026-08-15): DeepSeek is presently a trial implementer (see the standing DeepSeek-trial ruling above) with no seeded `HubStore` roster identity and no documented CLI/session contract; a native channel is real-integration work, appropriate only once the trial earns it. Deliberately double-purposed: beyond giving DeepSeek a native session, standing up a brand-new provider bridge from scratch — this time with C14.1–C14.9's accumulated lessons already written down (silent no-op launches, blocking synchronous calls freezing the UI thread, resize/DOM lifecycle bugs, single-writer races) — is itself the test of whether this team's workflow has actually improved at avoiding those recurring bug classes, or whether the same mistakes resurface on a fresh integration. |
| C14.11 | Meta **Muse Code** provider-native harness | Onboard Meta's `Muse Code` terminal coding-agent (Muse Spark 1.3-backed) as a managed harness: `HarnessId::Muse`, headless one-shot spawn argv, capture from Muse Code's local append-only event log, interactive resume off that log, and a `("muse", "Muse Code")` roster identity + `git/messages/muse_coauthor.msg` trailer. Model inference lands separately as `platform.md` P4a (Meta Model API). **Not started — [#273](https://github.com/ACFHarbinger/Coding-Assistants/issues/273).** Owner rationale (2026-09-08): Muse is now a full team member; assigned to Muse itself (self-integration, domain expert on its own CLI), Codex reviewing. First step is a spike on the exact non-interactive `muse-code`/`muse` invocation and event-log format — Meta's public posts do not confirm a batch/print flag, so no guessed `muse -p …` argv. |
| C14.12 | **Cursor `agent`** harness | Onboard the Cursor local CLI agent (`agent`, some installs alias `cursor-agent` — resolve once at setup). Headless one-shot via `agent -p "<prompt>" [-m <model>] [--output-format stream-json]`, working dir by `cd` (no verified `--cwd` flag), interactive resume via `agent --resume "<chat-id>" --print`. `agent ls` is **not** a reliable machine-readable session enumerator, so the integration must capture and persist the chat id when it creates a job rather than discovering it later. Cursor supplies its own account model config — this is harness-only, no `platform.md` provider row. `HarnessId::Cursor` + `("cursor", "Cursor Agent")` identity + `git/messages/cursor_coauthor.msg`. **Not started — [#275](https://github.com/ACFHarbinger/Coding-Assistants/issues/275).** Assigned to Cursor itself, Codex reviewing. |
| C14.13 | Alibaba **Qwen Code** harness | Onboard the Qwen Code CLI (`qwen`, `0.23.2`), a Gemini-CLI fork, as a managed harness. **CLI contract captured live 2026-09-09 — see the dated note below.** Its transcript is the **Claude-Code JSONL format** (`~/.qwen/projects/<sanitised-cwd>/chats/<sessionId>.jsonl`, `uuid`/`parentUuid`/`sessionId`/`timestamp`/`type`/`provenance`/`message.parts`), so the adapter mirrors `src-tauri/src/harness/claude.rs` / `gemini.rs`, **not** the Muse/Vibe event-log pattern. `--session-id` lets the caller pre-assign the id (Muse-style managed capture, no discover-then-register). `/coordinate` sub-agent team-runner + git-worktree isolation still need capture that does not collapse concurrent sub-agent output. Harness-only — Qwen Code carries its own OpenAI-compatible model config, no `platform.md` provider row. `HarnessId::Qwen` + `("qwen", "Qwen Code")` identity + `git/messages/qwen_coauthor.msg`. **Not started.** Owner rationale (2026-09-09 provider-menagerie review): a `/coordinate`-capable in-harness multi-agent runner to benchmark against CA's own orchestration. |
| C14.14 | Moonshot **Kimi** swarm harness | Onboard the Kimi Code CLI (`kimi`, `0.42.0`) as a managed harness. **CLI contract captured live 2026-09-09 — see the dated note below.** Unlike Qwen it has its **own event-log transcript** (`~/.kimi-code/sessions/wd_<slug>/session_<uuid>/agents/main/wire.jsonl` + `state.json`), closer to the Muse pattern but with distinct event types, and a real machine-readable session enumerator (`kimi session list --json`). `kimi acp` (stdio ACP server) is a clean managed-delivery transport, like Grok's leader ACP. The reason to onboard is the capability, not the model: swarm-native execution (`KIMI_CODE_AGENT_SWARM_MAX_CONCURRENCY`, `--max-subagent-depth`) is a different in-harness multi-agent design than Qwen's `/coordinate` or CA's stage graph. Harness-only, no `platform.md` provider row. `HarnessId::Kimi` + `("kimi", "Kimi")` identity + `git/messages/kimi_coauthor.msg`. **Not started.** Owner rationale (2026-09-09): passed on Kimi as a plain model on cost grounds but wants the swarm-native harness as a benchmark data point. |
| C14.15 | **Mistral Vibe** provider-native harness | Onboard the Mistral Vibe CLI (`vibe` 2.25.1) as a managed harness. **CLI contract captured live 2026-09-09.** Vibe has **no `--session-id` flag** — the caller cannot pre-assign an id — so managed start follows the Gemini pattern: run one turn with no resume flag, then register the sibling `meta.json.session_id` UUID from the newest matching `~/.vibe/logs/session/session_*/` directory (the directory name is a timestamp+short-id, *not* the resume token). `--resume <uuid>` for a real disk id only; `managed-*` / `pending` / empty stay a fresh spawn. Argv uses `--output streaming`. Identity split: harness key `"vibe"`, roster/agent id `"mistral"`. **Landed & Codex-PASS'd 2026-09-09** (`main` merges `3d13979` + `3aa3600`): S2 auth health probe reading `$VIBE_HOME/whoami_cache.json` (#304, `health/probes.rs`); S3 Mistral **Admin API** quota adapter (`GET /v1/admin/{usage,spend-limit}`, `x-api-key`, `MISTRAL_ADMIN_API_KEY` — the repo's first non-bearer credential header) + S4/S5 `ProviderQuotaLocalUsage` from `meta.json` `stats` and the unmetered `LocalUsageMeter` UI (#306); S6 `crates/hub/src/bridge/vibe.rs` — managed task delivery, writer lease, `Acked`, `latest_vibe_session_id` returning the UUID; S8 `crates/hub/src/harness/vibe_spawn.rs` — managed spawn + `--resume` + Gemini-pattern start (#305). **Landed & verified 2026-09-09 (S7):** S7 transcript capture adapter in `src-tauri/src/harness/vibe.rs` + `hub_capture_vibe_session` command + `App.tsx` 1.5s poll (with `hub_capture_muse_session` gap resolved), pulling assistant text from `messages.jsonl` into Chat & Memory with injected prompts and reasoning content filtered. Live acceptance verified against `vibe 2.25.1` (`vibe -p --output streaming`). C14.15 is now complete. |
| C15 | Markdown coordination files become journals, not source of truth | `AGENT_BUS.md`'s dated log entries and task-board assignment rows move into a queryable `HubStore` table, while Markdown remains human-readable narrative output rather than the record agents parse to coordinate. **Not started:** defer until C14, #161–#163, and C13 are settled; do not introduce another coordination substrate while the current one is under active acceptance. |
| C16 | Agent-invoked specialised sub-orchestrator models | A role running inside a harness can call a small, specialised model **directly** (in-process HTTP, not by spawning a CLI) to act as a swarm sub-orchestrator: plan and dispatch a bounded fan-out of sub-agents, or classify/route work, without spending a full frontier-model turn. Distinct from C5/C8 (declarative workflow wiring) and C14 (managed CLI sessions) — this is a role type backed by a direct model provider. Exit criteria: a role can be configured with a direct-call model, invoke it within a bounded fan-out, have those calls budgeted (C6) and audited like any other provider call, and degrade to `unavailable` when unconfigured. Builds on `platform.md` **P4** (direct HTTP providers); `platform.md` **P13** (optional routing gateway) is its natural companion, making the sub-orchestrator model cheap to swap. **Not started — owner-described 2026-09-09.** Scope after C13/C14/C15 land and P4 exists. |

**2026-09-09 (Harbinger) — enabler for C13/C15.** Owner is beginning the
migration off external Konsole tabs + the markdown bus. The first step is a
**UI** capability, tracked as `ui.md` **U15** / epic
[#295](https://github.com/ACFHarbinger/Coding-Assistants/issues/295): run every
harness CLI inside the app's own interactive terminals, tiled in a hand-rolled
resizable grid in Orchestrate (drag splitters to push/pull, drag panes to
re-arrange, layout persisted per workspace). This does not itself move any
coordination record — C13 (Hub replaces the markdown bus) and C15 (markdown
becomes journal) remain the substrate change, still gated on their existing
prerequisites. U15 just removes the reason the owner keeps six external
terminals open.

**2026-09-09 (Harbinger) — provider-menagerie review.** After a survey of the
current agentic-coding landscape (frontier labs + non-US + open-weight +
harness/routing services), the owner settled on a bounded set of additions
rather than an open-ended roster. Landed as three roadmap entries:
**C14.13** (Alibaba Qwen Code, a `/coordinate`-capable in-harness multi-agent
runner), **C14.14** (Moonshot Kimi swarm harness, agentic-swarm-native), and
**C16** (agent-invoked specialised sub-orchestrator models — the app's own
direct-model-call path). A model-routing gateway (OpenRouter / Requesty) is
`platform.md` **P13**, deliberately deferred until C16 exists and the
provider/credential/health surface is mature. Held for now: GLM-5.x and
MiniMax M3 hosted providers, and OpenHands/Aider as research baselines
(revisit if/when formal baseline benchmarking starts).

**2026-09-09 (Harbinger) — Qwen Code + Kimi Code CLI contracts captured live.**
Both installed and probed on the dev box (Claude, non-interactive one-shot
runs). Verified facts for C14.13 / C14.14 — replaces the "spike first, no
guessed argv" placeholder in those rows:

*Qwen Code — `qwen` 0.23.2, `~/.local/bin/qwen` (on PATH).*
- Non-interactive: positional prompt (or deprecated `-p`); `-o/--output-format
  {text,json,stream-json}`; `-y/--yolo` or `--approval-mode {plan,default,
  auto-edit,auto,yolo}`; `--include-directories`; `--session-id <id>` **pre-
  assigns the id** (so managed capture is Muse-style: register the id you
  passed, no discover-then-register); `-r/--resume <id>`, `-c/--continue`,
  `--fork-session`. `--chat-recording` must be set or `-c`/`-r` do nothing.
- Transcript: `~/.qwen/projects/<sanitised-cwd>/chats/<sessionId>.jsonl`,
  **Claude-Code JSONL** — one object per line with `uuid`, `parentUuid`,
  `sessionId`, `timestamp` (ISO-8601), `type` (`user`/`system`/`assistant`),
  `provenance` (`real_user` vs `system` — the injected-vs-real signal),
  `cwd`, `version`, `message: {role, parts:[{text}]}`. Adapter mirrors
  `src-tauri/src/harness/claude.rs` / `gemini.rs`; filter `type=="assistant"`
  text parts, skip `type=="system"`.
- stream-json `result` event carries `stats.{models,tools,files,skills}` +
  `usage.{input_tokens,output_tokens,cache_read_input_tokens}` — a
  `local_usage` source in the shape S4 already added for Vibe.
- `qwen sessions list|ps|controllers`; `qwen serve` (HTTP daemon, Stage-1
  experimental `--http-bridge`); `--acp` (ACP stdio mode); `--json-file` /
  `--json-fd` emit structured events alongside the TUI; `--max-subagent-depth`
  (default 5); `/coordinate` is a live slash command.
- `--auth-type {openai,anthropic,qwen-oauth,gemini,vertex-ai}`; provider
  config in `~/.qwen/settings.json` `modelProviders.openai[]` (`baseUrl`,
  `envKey`). **Caveat: `QWEN_CODE_HOME` relocates config/sessions but detaches
  auth** — a run under an overridden home 401s. Not the hermetic-test lever
  `VIBE_HOME` was; tests must stub the transcript dir directly.
- **Auth state 2026-09-09: the OAuth token is expired (`401 invalid access
  token`).** `qwen` re-login needed before any live delivery test.

*Kimi Code CLI — `kimi` 0.42.0, `~/.kimi-code/bin/kimi` (**NOT on PATH**).*
- `resolve_binary("kimi")` fails as-is — the health/spawn path needs the
  `~/.kimi-code/bin/kimi` fallback, same shape as Cursor's `agent` /
  `cursor-agent` dual-name handling.
- Non-interactive: `-p/--prompt`; `--output-format {text,stream-json}`;
  `-y/--yolo` / `--auto` / `--plan` (**`-p` cannot combine with `--auto`** —
  `-p` already runs headless). `-S/--session [id]`, `-c/--continue`;
  `kimi fork`, `kimi export` (ZIP).
- stream-json stdout is minimal: `{role:"meta",type:"system.version"}`,
  `{role:"assistant",content:"…"}`, `{role:"meta",type:"session.resume_hint",
  session_id, command}` — the resume-hint line hands you the id + exact
  `kimi -r <id>` command.
- Session store: `~/.kimi-code/sessions/wd_<slug>_<hash>/session_<uuid>/` with
  `state.json` (`{id, version:2, cwd, createdAt, updatedAt (ms epoch),
  archived, agents:{main:{homedir,type}}, lastTurnReason}`) and the real
  transcript at `agents/main/wire.jsonl` — an **event log**, per-line `type`:
  `metadata`, `profile.bind`, `turn.prompt`, `context.append_message`
  (`message.role` + `message.origin.kind`), `llm.request`, `usage.record`
  (`usage.{inputOther,output,inputCacheRead,inputCacheCreation}`,
  `usageScope:"turn"` — the `local_usage` source), `context.append_loop_event`
  (`event.type=="content.part"` → `part.text` is assistant output;
  `event.type=="step.end"`), `turn.ended`, `prompt.completed`. Closer to the
  Muse event-log filter pattern than to Claude's format.
- `kimi session list --json [--all] [--cwd <path>]` → `[{id, workDir,
  sessionDir, createdAt, updatedAt, archived, lastTurnReason}]` — a genuine
  machine-readable enumerator (unlike Cursor's `agent ls`).
- `kimi acp` (stdio ACP server) — the clean managed-delivery transport, in
  the same family as Grok's leader ACP; prefer it over parsing the minimal
  stream-json for task delivery.
- Config: `~/.kimi-code/config.toml` (TOML). Provider `managed:kimi-code`,
  `base_url = "https://api.kimi.ai/coding/v1"` (**`.ai`, not the `.com` some
  third-party guides show**), OAuth under `~/.kimi-code/{oauth,credentials}/`.
  Models `kimi-code/{kimi-for-coding, kimi-for-coding-highspeed, k3,
  k3-256k}`; `k3` is 1M context with `support_efforts=[low,high,max]`.
- **`KIMI_CODE_HOME` is honored** (creates a full scratch home) — the
  hermetic-test lever, like `VIBE_HOME`. Also `KIMI_CODE_DATA_DIR_NAME`,
  `KIMI_CODE_AGENT_SWARM_MAX_CONCURRENCY`, `KIMI_CODE_SWARM_TIMEOUT_MS`,
  `KIMI_CODE_EXPERIMENTAL_SUBAGENT_FORK`.
- Auth state 2026-09-09: working (round-trip returned a completion).

#### C14.5 desktop acceptance matrix

This is the Orchestrate / Chat & Memory surface. It reads existing Hub
session records and reuses `hub_start_harness`, `hub_register_harness_session`,
`hub_register_managed_harness_session`, `hub_inject_harness`, and
`hub_relaunch_harness_in_terminal`. It does
**not** change provider transports, writer leases, or the
`harness_session_registrations` schema. Dismiss is UI-only and must never
steal a writer lease. TUI coverage stays with the TUI owner.

| Surface | Action / state | Expected result |
| --- | --- | --- |
| Orchestrate readiness | No row for the workspace | Empty list plus the selected provider's setup prerequisite |
| Orchestrate | **Register observed** | Capture-only `observed` / `ready`. No process spawn |
| Orchestrate | **Start managed** | Provider-specific: Grok uses Connect / resume (real session id). Other harnesses require a real thread/conversation id — never `managed-<pid>`. Failed start stays unregistered |
| Orchestrate | **Resume in terminal** | Kill the optional registered managed pid, then open a real terminal running that harness's interactive CLI (resume latest on-disk session when one exists). Not the headless `hub_start_harness` spawn; does not attach to an undocumented socket or TTY |
| Chat session strip | Registered rows | High-contrast **managed**, **observed**, **busy**, **queued**, **unavailable** (and `stopped` when recorded) |
| Chat send | all / subset / one + task and/or wake | Existing C10/C11 tagged send; inject outcomes appear as a banner, not a browser `alert` |
| Chat banner | `queued` / `busy` / `unavailable` | Retry re-calls `hub_inject_harness` for that message. Dismiss hides the notice only |
| Codex / `agy` busy writer | Second task while leased | Truthful queued/retryable outcome; no second writer is started |
| Claude without Channel | Task inject | `unavailable` or queued; session remains capture-only |
| Observed Gemini / interactive TUI | Task inject | `unavailable`; UI must not claim attach |
| Kubuntu live | Replies, cancel, restart, permissions, no-writer-race | Owner-run evidence on #152. Implementation alone is not acceptance |

**Provider facts verified on 2026-08-13:** Codex CLI 0.147.0 supplies the
experimental documented app-server; Claude Code 2.1.231 supplies documented
Channels (research preview) for pushed events into a running session and
two-way reply tools; Antigravity CLI (`agy`) 1.1.12 supplies `--print`,
`--output-format stream-json`, and `--conversation`, but no active-session
IPC/RPC. `agy` has no `--cwd` argument, so its workspace is the child process
working directory. These facts replace the prior incorrect `agy --cwd` wake
argv; the basic wake now uses the documented one-shot stream contract.

### C13 migration gate

The five completion conditions are unchanged:

1. **Preflight:** C10–C12 have passed their live acceptance checks; create or
   load a named work session with a recorded workspace and enrolled team.
2. **Hub-native run:** the owner assigns a bounded repository task through
   Chat & Memory to all, a subset, and one agent. At least two agents must
   acknowledge, execute/review, and publish their harness-originated result
   into the same session transcript; include one audited task or wake delivery.
3. **Reconstruction:** the session transcript, recipient/outcome records, and
   audit trail independently show assignment, delivery, execution, review,
   final decision, and handoff. No `.agent/cache/AGENT_BUS.md` or
   `.agent/messages/*` write is permitted during the run.
4. **Recovery:** if delivery, capture, or review fails, record the failure in
   the Hub and resume only through the existing Markdown bus. Do not delete,
   rewrite, or silently import historical bus/message files.
5. **Completion:** attach the acceptance evidence to #113, update the
   changelog/roadmaps and Project 21, then demote the Markdown bus to
   documented read-only fallback rather than removing it.

#### Owner-run checklist (2026-08-13)

Use this on Kubuntu against a real repository (for example this checkout or
Project-Mobile-Fortress). Do **not** treat automated C12 fixture tests as
this gate. Record every answer on #113. Stop and use the Recovery step if a
required delivery path is missing.

**Known transport truth (C12 accepted):** Grok task inject uses a registered
ACP leader socket. Chat/Codex task inject uses documented `codex app-server`
`thread/resume` + `turn/start` when a persisted thread is registered or found
on disk. Claude and Gemini **capture** from disk; their **task inject** stays
`unavailable` and queued. A **wake** may spawn via explicit argv. A **task**
never spawns a replacement process.

##### A. Preflight (gate 1)

1. Confirm C12 #145 is accepted. Do not start if adapters were reopened.
2. Snapshot the Markdown fallback (do not edit these files during the run):
   ```bash
   sha256sum .agent/cache/AGENT_BUS.md
   find .agent/messages -type f -print0 | sort -z | xargs -0 sha256sum
   ```
   Attach the hashes to #113 as **before**.
3. Desktop: Orchestrate → set an **absolute Workspace Root** → enroll at least
   `human`, `grok`, and `chat` (plus Claude/Gemini if they will only capture).
4. **Create team chat** or **Load team chat**. Confirm the header shows that
   named session and workspace.
5. Optional but recommended for inject: register live sessions
   (`hub_register_harness_session` / Orchestrate discovery) so Grok has a
   leader socket and Codex has a `diskSessionId` thread id.

##### B. Hub-native run (gate 2)

Use Chat & Memory on the named session. Composer: all / subset / one, plus
**task**, **wake**, both, or neither. Send is explicit.

| Step | Address | Tags | Expected durable result |
| --- | --- | --- | --- |
| B1 | **all** enrolled members | neither | One recipient set; no spawn; non-targets not tasked |
| B2 | **subset** (two members) | **task** | Each present member accepted; absent refused; `policy_decision` recorded; **no spawn** |
| B3 | **one** (a present member) | **wake** or task+wake | Wake may enroll/spawn per policy; task still refuses if not present |
| B4 | **one** unsupported inject (Claude or Gemini) | **task** | Inject status `unavailable` or `queued`; message remains in the session inbox |

Then:

6. From at least **two** harnesses, produce a real assistant reply in that
   workspace (Grok and Codex are the supported inject pair; Claude/Gemini
   count if their on-disk transcript is captured).
7. Refresh/capture into the same session until two harness-originated
   messages appear in the session channel.
8. Confirm one B2/B3 delivery left a `SendOutcome` (`accepted` /
   `wake_enrolled` / `wake_denied_*` / `task_refused_not_present`) and, if
   injected, a `HarnessInjectResult` of `delivered` or truthful
   `unavailable`.

##### C. Reconstruction (gate 3)

Independently, without opening `AGENT_BUS.md` as the source of truth:

9. Session transcript shows B1–B3 assignment text, recipient badges, and the
   two harness results.
10. `tagged_send_outcomes` / UI outcome list matches the intended `to` set.
11. Audit / journal shows the human send and any wake-policy decision.
12. Re-hash the fallback files from step 2. **Pass only if every hash is
    unchanged.** A write to `.agent/cache/AGENT_BUS.md` or `.agent/messages/*`
    fails the gate.

##### D. Recovery (gate 4) — only if B or C fails

13. Record the failure in the Hub (outcome reason, inject `unavailable`
    detail, or a human note in the session). Do not invent a delivered
    harness result.
14. Resume coordination on the existing Markdown bus. Do **not** delete,
    rewrite, or silently import historical `.agent/messages/*` or
    `AGENT_BUS.md` into the Hub.

##### E. Completion (gate 5)

15. Attach to #113: before/after hashes, session id, recipient lists for
    B1–B3, two harness capture message ids, one inject/outcome record,
    and a short reconstruction narrative.
16. Chat/Codex updates changelog, this roadmap, and Project 21, then
    demotes the Markdown bus to documented **read-only fallback**. Do not
    remove the files.

**Pass:** steps A–C and E complete, hashes unchanged, two harness results
in the named session, one audited task or wake delivery.

**Fail:** any Markdown-bus write during the run; task-only spawn; fabricated
delivery; fewer than two harness-originated session messages; missing
all/subset/one coverage.

#### Preflight helper

Preferred, non-mutating inspector (does not call `HubStore::open`, so it will
not create `hub.db` or `.agent/**`):

```bash
ca preflight --workspace /absolute/path/to/repo
# optional: --session <work-session-id>   --json
```

It prints a paste-ready #113 block: Hub home, team, requested session,
registered harness readiness (no start/inject), and fallback file hashes.
Run it again after the live loop; hashes must be unchanged.

Shell fallback if `ca` is not on PATH:

```bash
sha256sum .agent/cache/AGENT_BUS.md
find .agent/messages -type f -print0 2>/dev/null | sort -z | xargs -0 -r sha256sum
```

#### Evidence template (paste into a #113 comment)

Do not submit this template until the live run is finished. Leave unused
rows as `not run` rather than inventing ids.

~~~~markdown
## C13 live run — owner evidence

- **Date (UTC):**
- **Machine / OS:**
- **Workspace root (absolute):**
- **Named session id / title:**
- **Enrolled team:**
- **Result:** pass / fail / recovered-via-markdown-bus

### A. Preflight hashes

**Before**

    (paste sha256 lines)

**After**

    (paste sha256 lines)

Hashes unchanged? yes / no

### B. Addressing and tags

| Step | Recipients | Tags | Outcome ids / policy_decision | Inject status |
| --- | --- | --- | --- | --- |
| B1 all |  | neither |  | n/a |
| B2 subset |  | task |  |  |
| B3 one |  | wake or both |  |  |
| B4 unsupported task |  | task |  | unavailable/queued |

### C. Harness results (need two)

| Harness | Capture or inject | Message id | Notes |
| --- | --- | --- | --- |
| grok / chat / … |  |  |  |
| grok / chat / … |  |  |  |

### D. Reconstruction (no Markdown bus as source)

- Transcript shows B1–B3 and both harness results? yes / no
- Recipient sets match the table? yes / no
- Audit/journal shows the human send and any wake decision? yes / no

### E. If failed

- Hub failure record (outcome / inject detail / session note):
- Resumed on existing Markdown bus without rewriting history? yes / no / n/a

This comment is **not** a C13 pass by itself. Chat/Codex closes #113 only
after reviewing these fields.
~~~~

**2026-08-12:** CA-102 adds bounded, exact channel queries to the shared
store, CLI, and Tauri API (`channel:<name>` plus colon-delimited metadata).
Chat messages can embed `[Memory #<full-id-or-unique-prefix>]`; the Hub
resolves only unique references, retaining isolation and avoiding accidental
links to similarly prefixed memories. CA-106/109 add owner-only edit/delete
parity across the desktop and CLI. CA-114 adds contextual replies using the
same subject namespace (`channel:<name>:thread:<root>:<id>`), preserving
channel isolation and existing roster wake behavior without a migration.

**2026-08-13:** Named work sessions are durable `work_sessions` plus
membership records. A session initializes from the persisted team; an agent
added to the team is also enrolled in the active session. Its chat uses an
isolated `channel:session:<id>` subject namespace, so messages emitted from a
human or agent harness render together while per-member wake selection stays
an explicit delivery decision.

The `.agent/reports`, `.agent/messages`, and `.agent/cache/AGENT_BUS.md`
conventions are temporary process artifacts, not the long-term communication
protocol. Until C10–C13 ship, Grok and Chat still coordinate sub-task
allocation on `AGENT_BUS.md`.

**2026-08-13 (Grok, v1 hub-native orchestration):** Harbinger's remaining
workload is to run the team from the CA app instead of per-repo markdown.
C10–C13 plus U11–U12 are that delivery. Order: U11 load/create, then C10+U12
addressing and tags, then C11 spawn-vs-existing semantics, then C12
four-harness capture/inject, then C13 retire the markdown bus.

**2026-08-13 (Chat/Codex, migration intake):** Grok is the task-assignment
lead and Chat/Codex is the review/governance lead. The active streams are
session lifecycle, all/subset/one tagged composer UX, durable task-vs-wake
semantics, provider-safe harness capture/injection, and the C13 owner-run
acceptance checklist. `AGENT_BUS.md` remains the temporary allocation fallback
only. A startup regression in the Chat & Memory message stream was fixed before
the programme begins; the app once again reaches its initial empty state.

**2026-08-13 (Grok, U13):** Chat & Memory channels are durable `chat_channels`
rows. Custom channels can be created and soft-deleted; built-in four remain.

**2026-08-11:** The desktop Shared Hub originally exposed Inbox/Wakes panels
over the same store as the CLI. Those duplicate surfaces are now retired:
messages and memory belong to **Chat & Memory**, while wake events belong in
`#wakes-alerts`. Persisted `WakePolicy` remains in the Shared Hub Policy tab;
per-task policy is available through CLI/Tauri task creation, while desktop
task-creation controls remain open. A2A
Agent Card discovery and delegation payloads are implemented via `AgentCard` in `hub`, exposed via `ca agent register-card` and `GetAgentCards` in the `TcpServer`.

The C5 task schema and dispatch path now persist retry counters,
parallel-stage queues, and a maximum concurrency bound.

The Tauri execution path now performs call-count accounting around `LLMClient`
completions and invokes the existing handoff flow on exhaustion. Cancellation
now also writes a durable shutdown handoff before the active run exits.
Provider automatic spend reporting, external-adapter adoption, and shutdown
hooks remain open. External agents can now reserve units atomically with
`ca budget consume` immediately before a provider request.

**2026-08-11:** C6 first boundary implemented — per-agent budgets are caller-defined units
(call count, USD, tokens, ...); the store only compares totals, so the
provider-cost mapping is a caller concern. `pause_for_budget` is explicit
(distinct from the automatic `paused` flip in `record_budget_usage`) so a
caller can keep working briefly after crossing the limit if it chooses, but
is expected to call it before stopping, per the owner's original answer (a
persistent summary + delegation + shutdown, not a hard kill). Provider
automatic spend reporting and shutdown hooks remain open, as does desktop
The remaining workflow gap is fully parallel session startup under C8.

**2026-08-12:** Team fan-out now uses an explicit persisted roster
(`agents.team_member`) instead of every row in `agents`. Default members:
`human`, `claude`, `chat`, `gemini`, `grok`. Harbinger is included so
`#general` is visible to the owner. Chat & Memory/Orchestrate team sends wake that
roster with `hub_request_wake` per enrolled member (`HubStore::request_team_wakes`,
`hub_request_team_wakes`). Enrollment: `ca agent enroll\|unenroll\|team` and
`hub_set_team_member`. Chat's CA-102 channel-query work owns
`list_channel_messages` in the same store.

**2026-08-13:** Chat & Memory DMs no longer inherit the team-broadcast recipient
(`2ab31c7`). Composer is Enter-to-send with a jump-to-latest chip while
reading history (`947a43d`). Thread replies (CA-114, Chat) stay in the
`channel:<name>:thread:` subject namespace.

**2026-08-13 (cloud):** Multi-device replica of `.coding-assistants` is specified
in [`cloud_sync.md`](cloud_sync.md) (S1–S13, issues #91–#103). Drive first;
journal-integrity merge is S6 after the S5 snapshot gate. Not implemented.

**2026-08-15 (DeepSeek, #161):** the Orchestrate "Resume in terminal"
surface now reports truthfully instead of silently no-oping. Embedded PTY
sessions retain a bounded output tail plus the real exit status
(`pty_session_status`), so a fast-failing harness CLI (e.g. `codex resume
<stale-id>`) shows its error output and an "exited" chip rather than a
blank terminal; a missing session renders an explicit error state. Session
discovery during a launch is bounded (3 s) with a truthful "fresh session,
not a resume" note on timeout, and both relaunch commands run their
blocking resolve off the Tokio worker. The external terminal path is
preserved. Verification: build + clippy + targeted tests (per the standing
hardware constraint).

**2026-09-08 (Harbinger):** Two new harnesses onboarding as C14.11 (Meta
**Muse Code**, #273) and C14.12 (**Cursor `agent`**, #275), plus their team
identities (`muse`, `cursor`). Model inference for Muse Spark 1.3 is
tracked separately as `platform.md` P4a (#274, Meta Model API). The two
Perplexity MCP servers (official `@perplexity-ai/mcp-server` #276, and the
subscription-session `perplexity-web-mcp-cli` #277) are external-server
integrations under `platform.md` P9. The shared registry (#272) is on
`main`; #276/#277 (Grok) are ready for review. Settings toggle is #278
(Gemini). Parent tracking issue: #270. Team now: Claude (lead),
Chat/Codex (review), Grok (impl), Gemini (UI), Muse, Cursor.

**2026-09-08 (Harbinger) — credentials, account connections & provider
quotas batch (parent #279).** #273/#274/#276/#277/#278 landed on `main`
(merge train `ec837cd`/`03ba888`/`ee3ef0b`); **#275 held** on three open
Codex session-integrity findings. Follow-on: **C14.11** — a Muse Code
plan-quota surface belongs on the harness readiness panel once
`platform.md` #280 lands the Muse Spark adapter (the Meta Model API key is
shared). **C14.12** — a Cursor `agent` plan-quota adapter is
`platform.md` #281 (owner: Cursor; owner overrode the #275 sequencing
hold). The CLI has no usage subcommand; the adapter reads the same
dashboard JSON the app `/usage` panel loads (`GetCurrentPeriodUsage`)
and degrades to `unavailable` when not logged in. #280 remains
spike-gated. Secret storage for these harnesses' keys/tokens (Meta
`MODEL_API_KEY`, Cursor login) moves to the `platform.md` P12 vault
(#282) and `settings.md` S8 (#283/#284/#285). Per-user account links in
the shared Hub are `multi_human.md` **H7** (#286), blocked on H2.
DeepSeek and OpenCode re-join the active implementer roster this batch;
see the agent bus team table.

**2026-09-08 (Harbinger) — complete the P3 status surface (parent
`platform.md` #291).** The C14.11/C14.12 readiness rows for **Muse Code**
and **Cursor `agent`** (and the existing Claude Code / Gemini `agy` rows)
currently show only process discovery. A typed cheap `ProviderHealth`
probe (#291) and its readiness-panel surface (#292) add
installed / authenticated / auth-expiry per harness without a usage call;
#293 reconciles `managed_pid` against `hub::proc` and surfaces the writer
lease so a dead managed session is visible without a relaunch attempt.
Muse/Cursor self-integrate their health branches (#294).

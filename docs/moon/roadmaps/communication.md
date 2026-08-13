# Agent Communication and Delegation Roadmap

Communication starts with explicit, declarative task wiring and asynchronous
mailboxes. Parallel execution and A2A follow only after durable local
communication is reliable.

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
| C12 | Bidirectional harness capture and inject | The app captures messages agents send inside Grok/Chat/Claude/Gemini harnesses into the session transcript. Hub messages tagged task and/or wake are injected into the target harness so the agent executes them. Builds on C9. | ✅ **Done (safe baseline)** · #145. Capture polls all four. Grok task delivery uses the registered ACP leader path. Codex/Chat task delivery uses documented `codex app-server` `thread/resume` + `turn/start` when a persisted thread is registered or found on disk; otherwise `unavailable` and queued. Claude and Gemini capture/discovery are real; their control transports stay `unavailable` and queued. Task-only inject never spawns a replacement process. No PTY writes or fabricated sockets. C14 is the provider-native managed-session follow-on. |
| C13 | Hub replaces the per-repo markdown bus | A full assign/review/task/wake loop completes with no writes to `.agent/cache/AGENT_BUS.md` or `.agent/messages/*`. Those files stay as a fallback until C10–C12 ship. `.agent` prompts/rules/skills remain resources, not the live protocol. | 📋 **Planned** · C12 accepted (#145). Owner-run checklist, #113 evidence template, and `ca preflight` (#146) are ready. Live owner evidence is still required before the Markdown bus is demoted. |
| C14 | Provider-native managed harness sessions | Chat/Codex, Claude Code, and Gemini/Antigravity can each be deliberately launched, registered, messaged, observed, cancelled, and resumed through their supported contracts. Active sessions surface truthful readiness and delivery state; one provider writer owns each provider session. | 📋 **Planned** · Epic [#147](https://github.com/ACFHarbinger/Coding-Assistants/issues/147): supervisor [#148](https://github.com/ACFHarbinger/Coding-Assistants/issues/148), Codex [#149](https://github.com/ACFHarbinger/Coding-Assistants/issues/149), Claude [#150](https://github.com/ACFHarbinger/Coding-Assistants/issues/150) (done, live-verified), `agy` [#151](https://github.com/ACFHarbinger/Coding-Assistants/issues/151), UX/acceptance [#152](https://github.com/ACFHarbinger/Coding-Assistants/issues/152). Follow-on live-delivery fixes from the C14.3 acceptance audit: Grok [#154](https://github.com/ACFHarbinger/Coding-Assistants/issues/154), `agy` [#155](https://github.com/ACFHarbinger/Coding-Assistants/issues/155), Codex [#156](https://github.com/ACFHarbinger/Coding-Assistants/issues/156). Existing C12 capture and safe refusal remain the fallback until each provider reaches acceptance. |

### C14 provider-native integration contract

This is a managed-session programme, not permission to write bytes to an
existing terminal, socket, or provider-internal protocol. The owner explicitly
selects a harness/session for app ownership; ordinary discovered terminal
sessions remain observable only unless that provider exposes an opt-in,
documented bridge.

| Slice | Provider contract | Acceptance evidence |
| --- | --- | --- |
| C14.1 | Common session supervisor | Typed capability/readiness model, durable ownership and lifecycle audit, cancellation, output capture, and a per-session single-writer gate. The desktop/TUI shows **managed**, **observed**, **busy**, **queued**, or **unavailable** rather than claiming delivery. **In progress:** #148 has the durable observed/managed record and exclusive writer-lease foundation; supervisor commands, audit, cancellation, and UI remain. |
| C14.2 | Chat/Codex | Reuse a long-lived documented `codex app-server` connection per managed thread. Serialize `turn/start`; on an externally active writer retain the message as queued and show a retryable busy state, never race a second writer. **In progress:** #149 now enforces the durable managed-session writer lease and turns app-server active-writer errors into queued/retryable outcomes; the long-lived streaming broker remains. |
| C14.3 | Claude Code | Provide an opt-in Coding-Assistants Claude **Channel** MCP bridge. The bridge uses Claude Code's documented `claude/channel` capability to push authenticated Hub events into a session and a reply tool to return Claude output to the original Hub message/session. It supports the documented permission relay only after explicit human approval. Existing Claude sessions without that channel remain capture-only. **In progress:** [#150](https://github.com/ACFHarbinger/Coding-Assistants/issues/150) — `crates/claude` binary (renamed from `crates/claude-channel`) implements the stdio MCP server (`claude/channel` + `claude/channel/permission` capabilities, `reply` tool, permission-request relay); `hub::bridge::claude_channel` (new file, `bridge::claude`'s C12 path untouched) provides the authenticated-sender gate (enrolled team members only), reply routing, and a never-auto-approved permission lifecycle reusing the Hub audit chain. Opt-in setup registers the workspace as a C14.1-managed `claude` session and merges a `global.mcp.json` base layer plus a per-workspace canonical config — both stored durably under `~/.coding-assistants/servers/` — into the workspace's own `.mcp.json`. `--list`/`--rename`/`--delete` CLI subcommands and a Shared Hub "Channels" tab manage the registry. Only wake and task-tagged messages are pushed as a live-session interruption; plain chat/handoffs stay queued and are pulled on Claude's own initiative via a new `check_inbox` tool (`poll_channel_events`/`poll_quiet_channel_events`). The Channels tab now shows a live connected/not-connected status per workspace (`hub::is_channel_session_live`, a process-table check) and a Connect button that opens a real terminal running the `--channels` command (`hub::launch_claude_channel_session`) — Claude Code has no headless daemon mode, so this is always a real terminal, never a detached background process. End-to-end Claude Code acceptance (a real `--channels` session) manually verified via ping round-trip; automated acceptance coverage is still open. |
| C14.4 | Gemini / Antigravity (`agy`) | Start app-managed non-interactive workers with `agy --print --output-format stream-json` in the selected workspace; persist the conversation id and use documented `--conversation` only for a worker the app owns. Parse stream events into the Hub, support cancellation/status, and never pretend to attach to an unrelated interactive `agy` TUI. |
| C14.5 | End-to-end UX and acceptance | Orchestrate and Chat & Memory create/select managed sessions, show setup prerequisites and actionable errors, and test all/subset/one task+wake routing, replies, cancellation, restart/recovery, permissions, and no-writer-race behavior on Kubuntu. **Desktop UX ready for review** ([#152](https://github.com/ACFHarbinger/Coding-Assistants/issues/152)): readiness badges, observed vs managed register, retry/dismiss banners. Live Kubuntu owner-run remains open. |
| C14.6 | Grok live-session delivery | Enable Hub-delivered messages to actually reach a live, human-attended Grok terminal. **Diagnosed, unclaimed** ([#154](https://github.com/ACFHarbinger/Coding-Assistants/issues/154)): `deliver_grok_task` already implements the documented `--leader`/`--leader-socket` ACP path correctly; delivery is `unavailable` only because no leader process/socket exists by default. Not a code bug — needs owner-facing setup documentation and optionally a connect/spawn helper mirroring `hub::launch_claude_channel_session`. |
| C14.7 | Gemini / Antigravity (`agy`) `--prompt` argument fix | Fix the real bug behind off-topic/gibberish `agy` replies to task/wake sends. **Diagnosed, unclaimed** ([#155](https://github.com/ACFHarbinger/Coding-Assistants/issues/155)): `gemini_managed_spawn_args` passes the message body as `--prompt <text>`, but `agy --help` documents `--prompt` as a bare `--print` alias, not a value-taking flag — the real prompt is silently dropped. "Doesn't appear in the live session" is expected/by-design for the headless worker adapter, not a bug. |
| C14.8 | Codex: explain silent delivery to an unregistered live session | Surface why a wake to a live, human-started Codex session that was never Hub-registered gets no visible response. **Diagnosed, unclaimed** ([#156](https://github.com/ACFHarbinger/Coding-Assistants/issues/156)): nothing auto-registers a manually-started Codex session with the Hub, so delivery silently resolves `unavailable`/`queued`; even a resolved thread is only ever turned via a disposable headless `app-server` client, never the visible TUI directly. |

#### C14.5 desktop acceptance matrix

This is the Orchestrate / Chat & Memory surface. It reads existing Hub
session records and reuses `hub_start_harness`, `hub_register_harness_session`,
`hub_register_managed_harness_session`, and `hub_inject_harness`. It does
**not** change provider transports, writer leases, or the
`harness_session_registrations` schema. Dismiss is UI-only and must never
steal a writer lease. TUI coverage stays with the TUI owner.

| Surface | Action / state | Expected result |
| --- | --- | --- |
| Orchestrate readiness | No row for the workspace | Empty list plus the selected provider's setup prerequisite |
| Orchestrate | **Register observed** | Capture-only `observed` / `ready`. No process spawn |
| Orchestrate | **Start managed** | Documented wake spawn; on pid, row becomes `managed` / `ready`. Failed start stays unregistered |
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

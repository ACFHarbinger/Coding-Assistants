# Scheduled Goals / Gate-Driven Prompting Roadmap

Codex follow-up (2026-10-20): #351/#355 original recovery/pause findings addressed; attempt-aware completion corrected by Codex. Two crash-recovery gaps remain (wake replay and one-shot finalization); integration stays blocked.

> **Status:** Approved design (owner answered the four open questions
> 2026-10-19) — issue decomposition below is the implementation plan.
> Acting owner-directed handoff: Codex coordinates/integrates; Grok reviews.
> Cursor and OpenCode repair the core; implementation assignments are #351–#358.

**Goal:** a new **Schedule** tab in Orchestrate where a user creates a
task/goal, targets one agent or the standing team roster, picks a
date/time to fire it, and optionally keeps it **continuously re-prompting**
until either its **gate criteria** are met or its **budget** is exhausted.
Gate criteria are enforced two ways: a sandboxed command/tool call (P5
`os_tool`, e.g. `wc -l` on each touched file) or a built-in app metric
(e.g. each target agent's lifetime tokens-written counter).

## Owner decisions (2026-10-19)

| Question | Decision |
| --- | --- |
| **Team target** | "Full team" means the **standing roster** (`agents.team_member`), the same set `send_message_to_team` already broadcasts to — not a `WorkSessionRecord`'s members. |
| **Usage source** | **Both.** The per-provider quota probe (session 5h / weekly 7d % windows, already fetched in `src-tauri/src/commands/quota/*`) is the hard stop. The app's own cumulative `agent_metrics` counter is shown for reference only, and separately powers the `MinTokensWritten` gate criterion (a different thing from the budget stop condition). |
| **Scheduling reliability** | **OS-level integration** so a fire time is honored even if the app is closed. The `ca` CLI (`crates/cli`) already opens the same `hub.db` the GUI does and has a command-dispatch surface (`ca tool run\|approve\|pending` is the P5 precedent) — `ca schedule tick` is the natural headless entry point for cron/systemd/launchd/Task Scheduler to invoke. |
| **Delivery approach** | Mini-epic: this roadmap + GitHub issues, Claude implements the core directly. |

## Known cross-layer limitation (stated up front, not hidden in a footnote)

`crates/hub` cannot see `src-tauri`'s quota adapters (`ProviderQuota`,
session/weekly %) — `hub` is the lower layer; `src-tauri` depends on it,
not the reverse. That means:

- **When the app is open**, the in-process scheduler poller (owned by
  `src-tauri`, which already imports both `hub` and the quota adapters)
  can check live session/weekly % before each re-prompt and pause on
  exhaustion. Full fidelity.
- **When the app is closed**, `ca schedule tick` (pure `hub` + `cli`, no
  quota-adapter access) can only enforce the in-hub `credit_allowance`
  budget, not the live provider quota window. This is a known, documented
  gap for S1, not a silent one — extracting the quota-probe logic into a
  shared crate so `cli` can call it too is explicitly out of scope here
  and tracked as a follow-up (S5b).

## Known gap: usage accounting does not cover harness-CLI agents (verified, not hidden)

Checked directly: `gate_provider_call` is called from exactly one place,
`src-tauri/src/agent/budget.rs`, itself only used by `AgentSystem`'s
in-app role/`chat_completion` pipeline. `record_agent_metrics` is called
from exactly one place, `src-tauri/src/agent/orchestrator.rs:181`,
same pipeline — and what it records as `tokens_used` is
`output.split_whitespace().count()` (a word count, not a real token
count; the code's own comment says exact values are a later provider-
adapter job). **Neither path is touched by the managed/observed
harness-CLI agents this app actually orchestrates day to day** (Claude
Code, Cursor, Grok, etc. — `crates/hub/src/bridge/*`, driven over
PTY/messages, not `AgentSystem::chat_completion`). Retrofitting usage
accounting onto every harness bridge is its own multi-slice epic and is
**not** in scope here.

Consequence for this epic's design: `BudgetPolicy.credit_allowance` and
`GateCriterion::MinTokensWritten` are real and will work correctly for
targets running through `AgentSystem` (if/when that path is used), but
are **not reliable stop conditions for the harness-CLI agents this
Schedule tab will realistically target**. Two independent stop
conditions that work unconditionally, regardless of which pipeline the
target runs through, are therefore **mandatory, not optional**, on every
`ScheduledTask`:

- `max_attempts: u32` (required, sane default — mirrors `retry_task`'s
  existing `max_retries` precedent in `tasks/runtime.rs`).
- `deadline: Option<DateTime<Utc>>` (optional wall-clock cutoff).

A task is `Paused`/`Failed` the moment either trips, independent of
whatever `BudgetPolicy`/gate-criteria state is — with one agreed
boundary: `max_attempts` stops **new** dispatches, but the attempt
already in flight may still collect a response, evaluate gates, or time
out. `deadline` does not wait: it fails the task even mid-attempt.
`GateCriterion::Command`
and `GateCriterion::MaxLinesPerFile` are unaffected by this gap — they
read the filesystem/process state directly, not an accounting table —
and remain the primary, trustworthy gate mechanism for harness-CLI
targets. The Schedule UI (S8) must say this plainly next to the
tokens-written criterion and the quota-based budget toggle, not bury it.

## Wake policy: reused, not bypassed

Dispatch calls `request_wake` exactly the way `dispatch_step`
(`crates/hub/src/store/tasks/mod.rs:163-210`) already does for task-stage
advancement. It is governed by the existing `WakePolicy`
(`default_requires_human_gate` / `allow_auto_wake`) like every other
automated wake in the app — Schedule adds no bypass field and no new gate
class. If a user wants a schedule's re-prompts to proceed without a human
approving each wake, that is the existing, already-global
`allow_auto_wake` setting, not something this epic introduces or
special-cases per task.

## Response detection and quorum (S3)

A re-prompt only fires after the previous dispatch's response has either
arrived or timed out — never purely on the repeat interval. "Responded"
is a `messages` row from the target agent with `created_at` after
`last_fired_at`. For a `Team` target, the default quorum is **All**
(every current standing-roster member must have responded) to match the
"each agent" framing in the motivating example; a per-attempt
**response timeout** (separate from `repeat_interval_secs`) converts a
non-responding agent into a failed attempt counted against
`max_attempts`, so a dead/closed harness session cannot hang a task
forever.

## Build order

**S1 → S2 → S3 → S7 → S8 → S6 first.** That sequence alone is a complete,
testable feature with the app open — including the only path where the
owner's "Both" usage-source answer is actually honored (S6 is the
in-process poller with live quota access). S5a (the OS-level trigger) is
a separate follow-up commit once that's solid, not the foundation; a
timer that can fire `ca schedule tick` headlessly is worthless until the
tick logic it drives (S3) is itself correct and tested.

## Roadmap

| # | Slice | Summary | Status |
| --- | --- | --- | --- |
| S1 | Schema, types, and CRUD | `scheduled_tasks` table (new `crates/hub/src/store/schedule/` module, mirrors `scorecard/`'s split-file pattern to respect the 500-LoC cap) + `ScheduledTask`, `ScheduleTarget`, `GateCriterion`, `GateCombinator`, `BudgetPolicy`, `ScheduleStatus` types, **plus mandatory `max_attempts: u32` and optional `deadline`** (see gap note below — these are the only stop conditions guaranteed to work for harness-CLI targets). Plain CRUD: create/get/list/cancel. | 🔍 Ready for review · Cursor · `agent/cursor-351` |
| S2 | Gate evaluator | `GateCriterion::Command` (arbitrary program+args via P5 `os_tool`, pass = exit 0) is the general mechanism; `GateCriterion::MaxLinesPerFile` (glob-matched files, one `wc -l <file>` per match through the same `os_tool` sandbox, no shell/no `awk`/no multi-file `wc` total-line footgun) and `GateCriterion::MinTokensWritten` (reads `agent_metrics.tokens_used` deltas against a baseline snapshotted at schedule-creation time — see the usage-accounting gap note) are built on top of it. `GateCombinator::All\|Any` across a task's criteria list. | 🚧 Code on main; OpenCode gate corrections #352 |
| S3 | Dispatch/tick state machine | `HubStore::tick_scheduled_tasks(now)`: for each due task, dispatch (send to agent or `send_message_to_team`, plus `request_wake` — see wake-policy note) → wait for a response or response-timeout (quorum: All for `Team`, excluding `system`/`human`) → evaluate gates → Completed \| re-prompt (with a generated note of what's still unmet) \| Paused on budget \| Failed on `max_attempts`/`deadline`. One-shot tasks (`repeat_interval_secs: None`) fire once and complete, no gate loop. **Attempt boundary:** `max_attempts` caps new dispatches; the final in-flight attempt may complete, miss its gate, or time out; `deadline` is a hard cutoff even mid-attempt. Outbox recovery re-checks those stops before send; each recipient is claimed atomically; recovered one-shots complete on drain. Tick CAS never matches `paused`. | 🔍 Ready for review · Cursor · `agent/cursor-351` |
| S4 | `ca schedule` CLI + headless tick | `ca schedule list\|create\|cancel\|tick`. `tick` is what an OS trigger invokes. Credit-allowance budget only (see cross-layer limitation above). | 📋 Kimi #354 · queued after #347 + core acceptance |
| S5a | Linux OS-level trigger | systemd user timer (`~/.config/systemd/user/ca-schedule.timer` + `.service` invoking `ca schedule tick`), installed/managed from Settings. Built and tested on this dev machine. | 📋 OpenCode #356 · queued after CLI + in-app validation |
| S5b | macOS/Windows OS-level trigger | launchd plist / Windows Task Scheduler (`schtasks`) equivalents. **Owner VM verify owed**, same pattern as prior Windows-only work (#288) — no macOS/Windows box available here. | 💤 Deferred #357 · unassigned / owner VM verify |
| S5c | Quota-probe bridge (follow-up) | Extract `src-tauri`'s session/weekly quota-probe logic into a shape `crates/cli` can also call, so headless `ca schedule tick` gets full budget fidelity, not just `credit_allowance`. Explicitly deferred out of S1–S5a. | 💤 Deferred #358 · unassigned |
| S6 | In-app poller + budget bridge | `src-tauri`-side timer loop while the app is open: calls the same tick logic as S3/S4 but additionally checks live quota % before each re-prompt (the "Both" decision above) and pauses on exhaustion, same as `gate_provider_call`'s existing pause behavior. | 📋 Cursor #355 · queued after core + S8 |
| S7 | Tauri commands | `hub_schedule_create\|list\|cancel\|get\|tick`, registered in `invoke.rs`, thin wrappers over S1/S3. | 🔍 Ready for review · Cursor · `agent/cursor-351` (no IPC shape change) |
| S8 | Frontend: Schedule tab | New Orchestrate sub-tab (`orchestrateSubView` gains `"schedule"`, alongside `"setup"`/`"terminals"` in `src/App.tsx` — that file is at 492/500 lines, so the new tab's content is its own component, `App.tsx` only gains the nav button + a thin mount point). Task/goal builder: instruction text, target picker (agent list + "Full team"), date/time picker, repeat-interval toggle (off = one-shot), gate-criteria builder (command / max-lines-per-file / min-tokens-written, combinator), budget picker (credit allowance amount, "stop on quota exhaustion" toggle). List view with status, next-fire time, attempt count, manual cancel. | 📋 Gemini #353 · scaffold ready; final integration after #351/#352 |

## Non-goals / explicitly deferred

- No new approval gate class — `os_tool`'s existing `SandboxStrictness`
  policy governs every `Command`/`MaxLinesPerFile` gate check exactly as
  it governs any other `os_tool` call. Scheduling does not get a bypass.
- No shell scripts as gate criteria — `os_tool`'s `HARD_DENY` already
  blocks `sh`/`bash`/etc.; a gate criterion is explicit argv, same as
  every other `os_tool` call.
- No cron-equivalent for arbitrary app actions — this epic's OS-level
  trigger exists only to drive `ca schedule tick`, not a general-purpose
  scheduler for other app features.
- No composite "readiness score" across criteria — `GateCombinator` is
  `All`/`Any` over independent pass/fail facts, same discipline as the
  Team Scorecard epic's "never a composite score" constraint.

## GitHub issue plan

Epic: [#350](https://github.com/ACFHarbinger/Coding-Assistants/issues/350).
Children: #351 (S1/S3/S7 corrections, Cursor), #352 (S2 gates, OpenCode),
#353 (S8 UI, Gemini), #354 (S4 CLI, Kimi), #355 (S6 poller, Cursor),
#356 (S5a Linux trigger, OpenCode), #357 (S5b deferred), #358 (S5c deferred).
All are native sub-issues of #350. #351/#352 agree their shared gate-result
API first; #353 may scaffold while final integration waits for core PASS.
Code on main is not review acceptance: the six core findings remain tracked
on the bus and correction issues. The dated owner handoff supersedes the
original Claude-only delivery assignment while active.

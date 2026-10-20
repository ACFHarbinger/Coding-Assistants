# Scheduled Goals / Gate-Driven Prompting Roadmap

**S1-S3, S6, S7, S8 integrated into `main` (2026-10-20)**: the reconciled
`agent/grok-352` core (Codex-reviewed engine correction + pending-gate
resume), **S6** in-app poller (#355), and **S8** the Orchestrate Schedule
tab (#353, Codex-reviewed gate-editor fixes). S4/S5a next.

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
whatever `BudgetPolicy`/gate-criteria state is. `GateCriterion::Command`
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
| S1 | Schema, types, and CRUD | `scheduled_tasks` table (new `crates/hub/src/store/schedule/` module, mirrors `scorecard/`'s split-file pattern to respect the 500-LoC cap) + `ScheduledTask`, `ScheduleTarget`, `GateCriterion`, `GateCombinator`, `BudgetPolicy`, `ScheduleStatus` types, **plus mandatory `max_attempts: u32` and optional `deadline`** (see gap note below — these are the only stop conditions guaranteed to work for harness-CLI targets). Plain CRUD: create/get/list/cancel. | ✅ Landed in `main` |
| S2 | Gate evaluator | `GateCriterion::Command` (arbitrary program+args via P5 `os_tool`, pass = exit 0) is the general mechanism; `GateCriterion::MaxLinesPerFile` (glob-matched files, one `wc -l <file>` per match through the same `os_tool` sandbox, no shell/no `awk`/no multi-file `wc` total-line footgun) and `GateCriterion::MinTokensWritten` (reads `agent_metrics.tokens_used` deltas against a baseline snapshotted at schedule-creation time — see the usage-accounting gap note) are built on top of it. `GateCombinator::All\|Any` across a task's criteria list. P5 approval is a distinct `pending` outcome: the proposal is linked in the audit row, reused across ticks, and its approved `os_tool.ran` result is consumed. Missing workspaces and unavailable token measurements fail closed. | ✅ Landed in `main` |
| S3 | Dispatch/tick state machine | `HubStore::tick_scheduled_tasks(now)`: for each due task, dispatch (send to agent or `send_message_to_team`, plus `request_wake` — see wake-policy note) → wait for a response or response-timeout (quorum: All for `Team`, excluding `system`/`human`) → evaluate gates → Completed \| re-prompt (with a generated note of what's still unmet) \| Paused on budget \| Failed on `max_attempts`/`deadline`. One-shot tasks (`repeat_interval_secs: None`) fire once and complete, no gate loop. **Attempt boundary:** `max_attempts` caps new dispatches; the final in-flight attempt may complete, miss its gate, or time out; `deadline` is a hard cutoff even mid-attempt. Pending gate waits use the same attempt CAS as other live transitions. Outbox recovery re-checks those stops before send; each recipient is claimed atomically; recovered one-shots complete on drain or on the next tick if `delivered_at` landed first. Wake obligations replay idempotently for committed messages without repeating a finished wake. Tick CAS never matches `paused`. | ✅ Landed in `main` |
| S4 | `ca schedule` CLI + headless tick | `ca schedule list\|create\|get\|cancel\|tick`. `tick` is what an OS trigger invokes. Credit-allowance budget only (see cross-layer limitation above). | ✅ Landed in `main` (#354, Kimi). |
| S5a | Linux OS-level trigger | systemd user timer (`~/.config/systemd/user/ca-schedule.timer` + `.service` invoking `ca schedule tick`), installed/managed from Settings. Built and tested on this dev machine. | ✅ Landed in `main` (#356, OpenCode). |
| S5b | macOS/Windows OS-level trigger | launchd plist / Windows Task Scheduler (`schtasks`) equivalents. **Owner VM verify owed**, same pattern as prior Windows-only work (#288) — no macOS/Windows box available here. | 💤 Deferred #357 · unassigned / owner VM verify |
| S5c | Quota-probe bridge (follow-up) | Extract `src-tauri`'s session/weekly quota-probe logic into a shape `crates/cli` can also call, so headless `ca schedule tick` gets full budget fidelity, not just `credit_allowance`. Was deferred out of S1–S5a; now unblocked since the core stack is landed. | 🚧 Delegated · Kimi · #358 |
| S6 | In-app poller + budget bridge | `src-tauri`-side timer loop while the app is open: calls the same tick logic as S3/S4 but additionally checks live quota % before each re-prompt (the "Both" decision above) and pauses on exhaustion, same as `gate_provider_call`'s existing pause behavior. **Policy:** pause only on a measurable session/weekly `remaining_percent <= 0`; `unavailable` / empty windows / balance-only / local-usage-only do not invent a stop. Interval 15s; overlapping ticks skipped. Headless tick stays credit-only (S5c). Generic tick transitions do not match `paused`, so a stale active snapshot cannot resurrect a quota-paused row; pending outbox is not sent after pause. | ✅ Landed in `main` (#355, Cursor). |
| S7 | Tauri commands | `hub_schedule_create\|list\|cancel\|get\|tick`, registered in `invoke.rs`, thin wrappers over S1/S3. | ✅ Landed in `main` |
| S8 | Frontend: Schedule tab | New Orchestrate sub-tab (`orchestrateSubView` gains `"schedule"`, alongside `"setup"`/`"terminals"` in `src/App.tsx` — that file is at 492/500 lines, so the new tab's content is its own component, `App.tsx` only gains the nav button + a thin mount point). Task/goal builder: instruction text, target picker (agent list + "Full team"), date/time picker, repeat-interval toggle (off = one-shot), gate-criteria builder (command / max-lines-per-file / min-tokens-written, combinator), budget picker (credit allowance amount, "stop on quota exhaustion" toggle). List view with status, next-fire time, attempt count, manual cancel. | ✅ Landed in `main` (#353, Gemini). |

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

### Codex review checkpoint — 2026-10-20

#351/#355 require outbox atomicity/recovery and quota-pause race corrections;
#353 requires configurable gate inputs/one-shot clarity; #356 requires
concurrent command-pipe draining. Scoped review fixes are committed on
agent branches. #352 gate slice (`c92e4b4`) and #354 CLI (`c9c9f74`) accepted
conditionally, not merged: scheduler enablement/integration remains blocked
on the shared engine. Exact findings and verification are recorded in the
agent bus and issue comments. Older queued/RFR rows below are historical
where they conflict with this dated checkpoint.

Schedule review 2026-10-20: Codex corrected partial-team token-baseline false passes; gate tests and Hub lib suite pass. Those gate commits are rebased onto `971f999` on `agent/grok-352`.
Schedule review 2026-10-20: tick-summary IPC mismatch fixed by Codex. Changes still requested: editable command argv/check, globs and numeric gate thresholds; current builder only inserts hard-coded presets. Do not mark S8 accepted yet.
Schedule review 2026-10-20: Codex reviewed CLI wrapper and clarified --now help; 25 CLI tests pass. Slice accepted conditionally on corrected #351/#352 integration; no claim that the old scheduler core is safe.

Schedule review 2026-10-20: Codex fixed path directive injection, synchronous Settings blocking, and swallowed removal failure/reload ordering. OpenCode then fixed runner pipe draining: stdout/stderr are consumed on reader threads while the child runs (16 KiB bounded capture, excess drained and discarded), with the 15s timeout preserved and a 2s post-exit collection grace.

**All of the above is historical review commentary; S1-S8 are now landed in `main` (2026-10-20) — see the roadmap table. S5b/S5c remain deferred (#357/#358).**

Epic: [#350](https://github.com/ACFHarbinger/Coding-Assistants/issues/350).
Children: #351 (S1/S3/S7 corrections, Cursor), #352 (S2 gates, OpenCode),
#353 (S8 UI, Gemini), #354 (S4 CLI, Kimi), #355 (S6 poller, Cursor),
#356 (S5a Linux trigger, OpenCode), #357 (S5b deferred), #358 (S5c deferred).
All are native sub-issues of #350. #351/#352 agree their shared gate-result
API first; #353 may scaffold while final integration waits for core PASS.
Code on main is not review acceptance: the six core findings remain tracked
on the bus and correction issues. The dated owner handoff supersedes the
original Claude-only delivery assignment while active.

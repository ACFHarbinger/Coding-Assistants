# Team Scorecard / HITL Oversight Roadmap

> **Status:** Approved design — issue decomposition below is the
> implementation plan.
>
> **Owner lock (2026-10-19):** Grounded in Mitchell/Ghosh/Passi, "AI Agents
> Push Humans Out of the Loop" (`assets/bibliography/2608.23642v3.pdf`):
> sustained routine approval erodes the overseer's own vigilance, domain
> skill, and situational awareness ("oversight degrades the overseer").
> Brainstorm round ran in `docs/moon/proposals/team-scorecard-brainstorm.md`
> (2026-10-18/19): Codex, Grok, Kimi, Cursor, and Mistral each proposed
> entries and the owner answered each agent's open questions directly in
> their own session. Those recorded selections are taken as final (owner
> confirmed 2026-10-19). This file is the synthesis.
>
> **Goal:** give the owner (and, narrowly, reviewer agents) a trustworthy,
> advisory "Team Scorecard" — per-agent cost, what each agent actually
> touched, and a signal turned back on the owner's own review behavior —
> without adding new approval friction and without any agent being handed
> a league table to optimize against.

## Scope locked 2026-10-19

| Topic | Owner decision |
| --- | --- |
| **Priority categories** | Decision design, behavioral monitoring, role design. **Strategic friction is explicitly out** — no new approval gates in this epic. |
| **Metrics purpose** | All three: per-agent cost/performance accounting, security/trust surface (what an agent touched), and an oversight-quality signal turned back on the owner's own review behavior. |
| **Enforcement** | Opt-in / advisory only. A hard gate for a new security-relevant action class is a separate owner decision outside this epic. |
| **Network capture** | Process-level, best-effort, Linux-first. `/proc/<pid>/fd` socket-inode matching against `/proc/<pid>/net/tcp[6]` on the existing U23 `system_snapshot` cadence — not a new timer, not eBPF (needs root), not `LD_PRELOAD` (misses static binaries). |
| **Surface** | A new dedicated **Hub tab** (`scorecard`, sibling of `dashboard` / `usage` / `system`), not a D1–D4 subtab. |
| **Roles** | Formalized as a schema fact: `oversight_role` column on `agents`, separate from U21's free-text display `role` column and separate from the permission-roles store (`roles/gate.rs`). An audited `oversight.exception` row covers temporary swaps (the S11 Codex-implements precedent). |
| **Access model** | `scorecard_visibility` persisted owner setting (`owner_only` default \| `agents_own_card` \| `all_agents`), enforced at the query layer. Nothing from the scorecard auto-posts to the bus in any mode — a visible league table is exactly the optimization-target failure mode the paper warns about. |
| **Cost truth** | Per-request `provider.usage` rows are `measured`; account-level quota deltas are `estimated` context only and must never masquerade as task cost. A disagreement between them is surfaced, not silently resolved. |
| **Attribution basis** | Every stamped row carries `registered \| observed \| external \| inferred \| unknown` beside its `task_id`/`work_session_id`, so "measured" never silently includes "guessed" (the #165 capture-misattribution lesson). |
| **Sync boundary** | Settings-controlled sync level, default **local-only**: owner digest, opt-in evidence/review history, and stamped audit rows stay local by default. Audit-grade facts (role exceptions, trial graduation, classified findings) always sync in `hub.db`; network samples and derived trend projections are always local-only. |
| **Row identity** | Three row kinds — `agent` \| `harness` \| `provider` — never silently joined. An agent, its managed harness session, and the provider account it bills to are different facts (Cursor is a roster agent, a managed `agent`/`cursor-agent` harness, and a `CURSOR_TOKEN` quota provider, all at once). |
| **No new gates** | The scorecard only labels actions that already passed through existing gates (`os_tool.*`, vault, `tcp.auth_rejected`, provider budget). A new deny path is a separate owner decision. |

## Non-goals / explicitly deferred

Unanimous across all five proposing agents — do not re-propose these inside
this epic:

- **Canaries** (known-answer tasks to catch fabricated success) — blocked on
  stamped rows being able to record `declined`/`blocked` as distinct from
  `ran`. Revisit after T1/T7 land.
- **A second event-store / activity-ledger table** — `hub_get_scorecard`
  projects the existing hash-chained `audit_events` chain; a dedicated
  ledger table is a possible *later* evolution, not a first slice, and would
  give cloud sync (S1–S13) another table to learn.
- **eBPF / privileged network capture** — needs root/capabilities; out of
  scope while process-level `/proc` polling is sufficient.
- **New approval gates** — strategic friction is explicitly deprioritized.
- **Assignment recommendations** (recommend an agent for a task from
  history) — needs enough reliable observations first; revisit once T1's
  stamps have real data.

## Roadmap

| # | Slice | Summary | Status |
| --- | --- | --- | --- |
| T1 | Slice stamps, attribution, and measured cost | Nullable `task_id` / `work_session_id` / `attribution` written into `process_json` (not new `audit_events` columns — `ProcessJsonMeta` already parses these keys) on `os_tool` audit rows, the provider budget gate (new `budget.reserved` / `budget.stopped` ops), and tagged message sends. New `provider.usage` audit op records measured per-request cost (tokens now, currency once priced) so unbudgeted agents aren't stuck `unmetered`; gateway-routed calls (OpenRouter, OpenCode) cite the upstream provider plus a `via_gateway` field. Unstamped rows read `unknown`, never zero. | 🔍 Ready for review · Cursor · `agent/cursor-341` |
| T2 | Oversight role, trial, and exception authority | `oversight_role` (`implementer \| reviewer \| lead \| observer \| trial`) column beside `agents.role`; hash-chained `oversight.exception` audit op (agent, from/to, scope, granted_by, started_at, expires_at, reason); `oversight.trial_graduated` op. Authority setting: owner grants by default, a delegated lead is an explicit alternate setting a lead cannot self-enable. An exception grants no P5/vault/network rights and does not authorize self-review. Expiry: task completion or stated deadline, whichever first, plus explicit revocation. | 📋 Planned · delegated |
| T3 | `hub_get_scorecard` query, Hub `scorecard` tab, and visibility setting | One read-only Tauri command projecting stamped audit rows, tasks, work sessions, budgets, and quota snapshots into three row kinds (agent/harness/provider) with `measured`/`estimated`/`unknown` per figure, plus `coverage: managed \| observed \| unregistered`. New Hub tab (`HubTab` union) reusing the panel pattern beside `dashboard`/`usage`/`system`. `scorecard_visibility` setting enforced at read time. Default sort: open security-class rows, then oldest pending `human` decision, then measured cost — never a composite score. | 📋 Planned · delegated |
| T4 | Owner digest and work-session batch review | Private, owner-only digest card: queue age, outstanding decisions, review volume, unresolved findings — grouped by active/recent **work session** (U24), with Hub task id as a row column, GitHub issue as an optional label. Remote HITL (Telegram/TCP `/approve`/`/reject`) uses the same in-app `human` clock, tagged `via: desktop \| telegram \| tcp`, so it isn't double-counted. No new approve-all, no new deny path. | 📋 Planned · blocked on T1/T3 |
| T5 | `mcp.invoke` on the audit chain | `hub_mcp_client_call` (P14-A) currently writes no audit row. Record `mcp.invoke` (server, tool, workspace, actor, outcome, duration) on the existing chain, same security-class list as `os_tool.*`. Never copies arguments or tool payloads. Visibility only — no change to Tools-tab UX or a new gate. | 📋 Planned · blocked on T1 |
| T6 | Network samples on the U23 cadence | On the existing `system_snapshot` timer, for managed harness PIDs and registered observed sessions whose PID start time still matches: walk `/proc/<pid>/fd`, match socket inodes against `/proc/<pid>/net/tcp[6]`, record destination IP/port/pid/start-time/session id and whether the fd table was fully readable (`fds_unreadable` is a field, not a silent empty list). Capped ring: last 24h or a few hundred rows per session, whichever is smaller — fixed internal bound, not a setting. No reverse DNS, no payloads, no byte counts. | 📋 Planned · blocked on T1 |
| T7 | Consistency probes and reviewer-side signature | Three advisory probes computed in `hub_get_scorecard` (quota delta vs. claimed slice cost; git-diff scope vs. recorded touch-set; network observations vs. provider call log; writer-lease owner vs. slice claimant) — a disagreement writes one `scorecard.disagreement` audit row linking both sources; shown as a card badge. Reviewer-side oversight signature (dwell time, zero-evidence approvals, fix-find rate, owner-override-reversal rate) derived from T1/classified-findings data, same descriptive-only, no-single-score discipline as the owner's own signal. | 📋 Planned · blocked on T1/T3 |

Individual trends and classified review findings (severity/origin tagging
on reviewer feedback) are a T1/T3 extension, not a separate numbered slice —
fold them into T1's stamping work and T3's row detail view once the base
projection exists; they do not need their own schema beyond what T1/T2
already add.

## GitHub issue plan

Parent epic [#340](https://github.com/ACFHarbinger/Coding-Assistants/issues/340)
+ seven child issues, same pattern as #279/#295/#270:

| Slice | Issue | Owner |
| --- | --- | --- |
| T1 | [#341](https://github.com/ACFHarbinger/Coding-Assistants/issues/341) | Cursor |
| T2 | [#342](https://github.com/ACFHarbinger/Coding-Assistants/issues/342) | Grok |
| T3 | [#343](https://github.com/ACFHarbinger/Coding-Assistants/issues/343) | Gemini |
| T4 | [#344](https://github.com/ACFHarbinger/Coding-Assistants/issues/344) | *(next batch)* |
| T5 | [#345](https://github.com/ACFHarbinger/Coding-Assistants/issues/345) | *(next batch)* |
| T6 | [#346](https://github.com/ACFHarbinger/Coding-Assistants/issues/346) | *(next batch)* |
| T7 | [#347](https://github.com/ACFHarbinger/Coding-Assistants/issues/347) | *(next batch)* |

T1–T3 delegated now (can start in parallel — T3's query/panel scaffolding
does not need every T1 producer finished, it just reads `unknown` for
anything not yet stamped). T4–T7 opened for visibility, explicitly
blocked, queued for the next batch.

## Review notes

Full per-agent proposals, including every rejected option and the owner's
answers to each agent's open questions, live in
`docs/moon/proposals/team-scorecard-brainstorm.md` — that file is the
record of *why*; this roadmap is the record of *what ships*. Do not
duplicate that detail here; link back to it from PR descriptions and bus
notes instead.

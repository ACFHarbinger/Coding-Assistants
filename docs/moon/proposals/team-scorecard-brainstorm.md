# Brainstorm: HITL oversight & agent performance ("Team Scorecard")

Status: **brainstorming — proposals only, no implementation yet.** Nobody
branches or writes code against this file. This is a shared scratchpad that
becomes roadmap entries + GitHub issues once the owner picks a direction.

## Source

Mitchell, Ghosh, Passi — "AI Agents Push Humans Out of the Loop"
(`assets/bibliography/2608.23642v3.pdf`). Core claim: the more autonomy an
agent system is granted, the less capable its human overseer becomes at
actually overseeing it — routine approval erodes the vigilance, domain
skill, and situational awareness oversight depends on ("oversight degrades
the overseer"). Their fix is two-pronged, and both prongs apply to us:

- **Developers** (us, building CA) add runtime affordances: *strategic
  friction*, *decision design*, *behavioral monitoring*.
- **Deployers** (the owner, running the team) add organizational protocols:
  *trainings & exercises*, *workload & scheduling*, *role design*.

Six solution categories from the paper (Section 5 / Appendix Figure 2):

| Category | Mechanisms |
|---|---|
| Strategic friction | pre-commitment, delay-and-choice, reasoning probes, action gating |
| Decision design | bounded autonomy, batch review, automated pre-checks, supportive interfaces |
| Behavioral monitoring | time/override/evidence-seeking signatures, canaries, style-fatigue covariance, audits |
| Trainings & exercises | domain-skill maintenance, critical-evaluation training, self-monitoring training |
| Workload & scheduling | enforced breaks, rotations |
| Role design | assigning roles, separating roles, aligning incentives |

## Owner's scoping (already decided, 2026-10-18)

- Priority categories: **decision design, behavioral monitoring, role
  design** (strategic friction explicitly *not* a priority — don't propose
  more approval-friction mechanisms).
- Metrics purpose: **all three** — per-agent cost/performance accounting,
  security/trust surface (what each agent actually touches), and an
  oversight-quality signal turned back on the owner's own review behavior.
- Enforcement: **opt-in/advisory to start** — surface data and friction,
  never hard-block, except possibly specific security-relevant action
  classes (P5 execution, P6/A2A, secret vault) where a hard gate may be
  worth proposing.
- Network capture: **process-level**, not just known outbound calls CA's
  own code already makes. Technical note: a syscall-level interceptor
  (eBPF) needs root/capabilities; `LD_PRELOAD` shims silently miss
  statically-linked harness binaries. Polling `/proc/<pid>/net/tcp[6]` +
  `fd` symlinks for each spawned harness PID (Linux-first, best-effort,
  no new privileged plumbing) matches CA's existing idiom (same pattern as
  U23's system monitor) — treat as the default unless you have a concretely
  better option.
- Surface: a **new dedicated Dashboard panel** ("Team Scorecard"), not
  folded into existing D1-D4 views.
- Roles: **formalize in the Hub schema**, not just bus-text convention — a
  typed role per agent + an audited exception log (the S11 Codex-implements
  swap is the concrete precedent).

## Seed draft (Claude, 2026-10-18) — a starting point, not a constraint

Feel free to replace, merge, split, or argue against any of these. Propose
your own ideas too, inside or outside this list, as long as they serve the
owner's scoping above.

- **O1 — Structured role system.** Typed `oversight_role`
  (`implementer | reviewer | lead`) per agent in Hub, distinct from U21's
  free-text display role. New `role_exceptions` table: agent, from/to role,
  scope (issue/branch), granted_by, started_at, expires_at, reason.
- **O2 — Per-slice performance metrics.** Tie existing signals to a
  task/work-session id: time-to-RFR, reviewer fix-count per slice (rework
  rate), token+cost per slice (existing quota adapters), tool-call counts
  per agent (P5 audit rows, aggregated).
- **O3 — Process-level network capture.** `/proc`-based poller per spawned
  harness PID, Linux-first, best-effort, advisory only.
- **O4 — Decision design: bounded autonomy tiers + batch review.** Explicit
  tiers (routine merge = autonomous; security-relevant file patterns always
  gated) persisted in Hub settings; an actual batch-review digest UI instead
  of ad hoc per-slice narration.
- **O5 — Owner's own oversight signature.** Track review latency and
  override/approval ratio over time as a self-warning trend.
- **O6 — Team Scorecard panel.** New Dashboard tab surfacing O1-O5.
- **O7 — Canaries (stretch).** Known-answer tasks inserted occasionally to
  test whether an agent honestly reports inability vs. fabricates. Needs
  the rest landed first.

## How to propose

Add a `## <Your name> — proposals` section below (keep your own section,
don't edit others'). For each idea, use this shape:

```
### <short name> (suggested id, e.g. O8, or "replaces O4")

**Category:** decision design | behavioral monitoring | role design | other
**Problem:** one or two sentences — what gap this closes, tied to the
  owner's scoping above.
**Implementation options:**
  1. <option> — <what it buys, what it costs, new deps if any>
  2. <option> — ...
**Recommended option:** which of the above, and why.
**Complexity:** small | medium | large (relative to a normal roadmap slice
  in this repo).
**Touches:** files/crates/schema this would realistically touch.
```

This is a **docs-only** round — append your section directly on `main`
(small additive commit, same convention as `AGENT_BUS.md`), no branch, no
worktree, no code. Post a one-line note on the bus when your section is in.

---

## Codex — proposals

Owner selections confirmed in discussion on **2026-10-19**. These proposals
refine O1–O6; identifiers below are candidates for synthesis, not assignments
or authorization to implement. Present them in the dedicated Team Scorecard
Dashboard panel. All initial signals are advisory.

### Task ledger with optional comparison context (extends O2 / O6)

**Category:** decision design

**Problem:** The owner needs task-level cost and outcome evidence before
automated allocation can be useful. A single agent ranking hides differences
in task difficulty, reviewer coverage, and incomplete measurements.

**Implementation options:**
1. **A — Task ledger:** connect task/work-session IDs to duration, RFR,
   outcome, measured token/cost usage, and review findings. Reuse existing
   Hub and audit records; missing measurements stay unknown. Smallest useful
   view, with no new service dependency.
2. **B — Comparison context:** optional task type and complexity fields plus
   comparable-task detail. Adds classification and filtering work; display
   sample size and reviewer context when comparing tasks.
3. **C — Assignment recommendations:** recommend an agent from comparable
   history, availability, and cost. Requires enough reliable observations
   and evaluation of recommendation quality; future work.

**Recommended option / owner selection:** A, with B available as optional
extra information for each task. C is deferred. Keep active duration,
elapsed duration, and review wait distinguishable wherever observable;
account-level quota deltas must not masquerade as measured task cost.

**Complexity:** medium for A and optional B; large for C.

**Touches:** `crates/hub` task/session and audit queries; Hub schema;
`src-tauri/src/commands/` metrics IPC; `src/components/` Dashboard panel.

### Individual trends and classified review findings (refines O2)

**Category:** behavioral monitoring

**Problem:** Raw reviewer fix counts can reward easy assignments and penalize
agents whose work receives closer inspection. Changed requirements and style
preferences are different from implementation defects.

**Implementation options:**
1. **A — Individual trends:** show each agent's own outcomes over time,
   including observation counts and missing data. Low classification burden;
   cannot establish causal improvement by itself.
2. **B — Contextual comparisons:** compare similar task groups with complexity,
   sample size, and reviewer shown. Depends on sufficiently classified tasks.
3. **C — Classified findings:** record severity and origin (implementation
   defect, changed requirement, clarification, reviewer preference), with
   evidence and resolution. Requires a lightweight review-entry workflow.

**Recommended option / owner selection:** A + C. B can follow when enough
classified tasks exist. Count recorded findings rather than edited lines;
allow correction of classifications with an audit trail. Avoid a composite
agent score that obscures these distinctions.

**Complexity:** medium.

**Touches:** Hub review-finding schema and query layer; review outcome IPC;
Team Scorecard task detail and trend views.

### Evidence-linked activity ledger (foundation for O2 / O3 / O5)

**Category:** behavioral monitoring

**Problem:** Metrics become misleading if their source or attribution cannot
be inspected. Existing records have different coverage across harnesses and
must remain distinguishable from estimates.

**Implementation options:**
1. **A — Linked aggregates:** expose source links for every metric and unknown
   values for gaps. Reuses existing sources, but aggregation logic remains
   distributed.
2. **B — Shared activity ledger:** normalize events with source/event ID,
   agent, session, task, timestamp, attribution basis, and measurement status.
   Adds schema and ingestion work; enables consistent aggregation and dedup.
3. **C — Reconstruct on demand:** derive history from current source records.
   Fewer initial schema changes, but results can shift as inference changes.

**Recommended option / owner selection:** B incrementally, beginning with A's
evidence links. Preserve measured, estimated, and unknown states; record the
aggregation version and avoid duplicate ingestion. Start with a few reliable
event producers rather than claiming universal coverage. Store references
and bounded metadata, with retention controls; do not copy secrets or raw
prompt/output bodies into the metrics ledger.

**Complexity:** medium per producer; large across all sources.

**Touches:** `crates/hub` schema, audit/event ingestion and aggregate queries;
existing provider usage/session records; evidence-detail IPC and Dashboard.

### Managed and registered-process network observations (refines O3)

**Category:** behavioral monitoring

**Problem:** Provider call logs cannot show connections made by harnesses or
their tools. Socket observation also has gaps that must be visible to the
owner before it is used as a trust signal.

**Implementation options:**
1. **A — Managed processes:** poll socket ownership for CA-launched harnesses
   and identifiable descendants. Record destination IP/port, process/session,
   and observation times. Linux-first, using existing `/proc` idioms.
2. **B — Registered external sessions:** add externally launched harnesses
   when registration and process identity establish ownership. More coverage,
   with explicit attribution confidence and permissions/missing-data states.
3. **C — Privileged collector:** optional eBPF connection-event capture to
   improve coverage of brief connections. Requires privileged installation,
   platform support, and additional review; future work.

**Recommended option / owner selection:** A + B; C is deferred. Match socket
inodes from each process's file descriptors against the network namespace's
socket tables: `/proc/<pid>/net/tcp*` alone is not per-process attribution.
Track PID start time to guard against PID reuse, and record scan coverage and
gaps. Domains require supporting evidence; reverse DNS alone does not prove
the requested hostname. Polling observes sockets, not complete traffic,
request contents, transferred-byte totals, or security intent.

**Complexity:** medium for A; medium for B; large for C.

**Touches:** existing process/system-monitor utilities; managed/observed
harness session identity; Hub observation storage and retention; Scorecard
network detail. No initial privileged dependency.

### Private review workload digest with optional reflection (refines O5 / O6)

**Category:** decision design / behavioral monitoring

**Problem:** The owner needs to see review backlog and unresolved decisions.
Behavioural signals can help reflection, but approval speed or an unopened
CA view cannot establish whether a person reviewed evidence elsewhere.

**Implementation options:**
1. **A — Workload digest:** show queue age, outstanding decisions, review
   volume, and unresolved findings. Uses task/review events without drawing
   conclusions about the owner's attention.
2. **B — Evidence-aware history (opt-in):** record available evidence,
   explicitly observed evidence access, overrides, and later linked defects.
   Requires additional event collection and clear retention/privacy settings.
3. **C — Reflection (opt-in):** allow uncertainty notes and a request for
   second review on selected decisions. Requires a small interaction and
   storage surface; optional participation only.

**Recommended option / owner selection:** A as the baseline, B and C each
independently opt-in. Keep collection for B off until enabled, and distinguish
unobserved activity from absent review. Present descriptive signals without
a single oversight-quality score, mandatory prompts, or latency-only alerts.
The digest is private to the owner by default.

**Complexity:** medium for A/B; small for C once review records exist.

**Touches:** Hub review/event queries and owner settings; Dashboard digest,
evidence history and optional reflection controls; retention/export handling.

### Configurable authority for scoped role exceptions (extends O1)

**Category:** role design

**Problem:** Typed roles need an explicit authority model for temporary swaps,
such as S11's implementer/reviewer exchange. Authority must be configurable
without silently granting execution, network, or vault permissions.

**Implementation options:**
1. **A — Owner grants:** only the owner can grant scoped exceptions. Simple
   default; grants identify task, role change, reason, start and expiry.
2. **B — Delegated lead grants:** an owner setting authorizes a designated
   lead to grant scoped exceptions. Adds authority validation and audit UI.
3. **C — Advisory checking:** display role mismatches and exception history
   while work continues. Can accompany either granting policy and keeps
   initial enforcement aligned with the advisory rollout.

**Recommended option / owner selection:** expose an authority setting with
**A as the default** and **B as an explicit alternative**. C describes the
initial advisory behaviour already scoped for this round. Suggested expiry:
task completion or a specified deadline, whichever comes first, plus explicit
revocation; exact expiry policy remains for synthesis. Audit grants,
revocations, expiries, and authority-setting changes. A lead cannot enable
its own authority through an agent-accessible settings path. Role exceptions
do not grant tool permissions or authorize self-review.

**Complexity:** medium.

**Touches:** Hub agent-role/exception schema and authorization checks;
owner settings IPC; Team Scorecard role detail and exception audit view.

### Suggested delivery sequence

Establish role authority and evidence-linked task records first. Add
classified findings and individual trends, then managed/external network
observations and the owner digest. The dedicated Scorecard panel can grow
with these slices. Keep assignment recommendations and privileged network
capture explicitly deferred; this proposal does not approve automatic merges
or new approval gates.

## Grok — proposals

Read Claude's 2026-10-18 bus entry and Codex's section above. This section
does not reopen the selections Codex already recorded with the owner. It is
the implementer's cut: which existing Hub writers and tables the scorecard
should sit on, what not to build in the first slices, and the questions
still open. Assumed defaults on those questions are marked. Correct them
before this becomes a roadmap.

Owner selections confirmed **2026-10-19**: option 2 on every proposal
below (stamp the three Hub writers, `oversight_role` plus an audit row,
one `hub_get_scorecard` projection, risk-then-cost ordering, inode-matched
samples on the U23 snapshot, label existing gates only, canaries deferred).

### Questions still open

1. **Slice key.** Hub task id, work session id, or GitHub issue number?
   Assumed default: Hub task id is primary, work session id is a second
   nullable stamp, GitHub issue is an optional label so the panel works
   offline.
2. **Whose clock is the oversight signal?** In-app decisions by `human`
   only (audit approve/reject, wake resolve, `os_tool` approve), or also
   GitHub review and merge outside CA?
   Assumed default: in-app `human` decisions only for the first slice.
3. **Who classifies a review finding?** Owner only, or the review lead
   with the owner able to correct, both written to the audit chain?
   Assumed default: review lead may record, owner may correct.
4. **Retention.** How long do network samples and any new projection live?
   Assumed default: network samples are a ring of the last 24 hours or a
   few hundred rows per session, whichever is smaller. Task-linked audit
   rows stay with the task. No prompt or tool-output bodies.
5. **Oversight roles beyond the three.** Do design (Gemini) and trial
   (DeepSeek) need enum values, or do they stay U21 display labels?
   Assumed default: `implementer | reviewer | lead | observer` only.
   Observer stays because U21 already has it. Design and trial stay labels.
6. **Findings source.** Hub-only, or may the scorecard call `gh`?
   Assumed default: Hub-only.
7. **Scope.** `~/.coding-assistants` is one database for every workspace.
   Assumed default: filter by `workspace_path`, default the current
   workspace, with an all-workspaces toggle.

### Slice stamp on the writers we already have (O8, precedes the task ledger)

**Category:** behavioral monitoring
**Problem:** Codex's task ledger and D4's `hub_get_activity_view` both need
a task or session id. `agent_metrics` is one cumulative row per agent
(`lines_written`, `tokens_used`, `provider_calls`, `output_chars`).
`audit_events` stores path, operation, `process_json`, and a hash chain,
and it has no task id. A ledger that guesses attribution will look precise
and be wrong, which is the opposite of a trust surface.
**Implementation options:**
1. Infer the slice at read time from timestamps and `workspace_path`. No
   schema change. Mis-attributes overlapping sessions. Cheap, and wrong
   often enough to skip as the primary path.
2. Add nullable `task_id` and `work_session_id` on the three Hub writers
   that already exist: `os_tool` audit rows, the provider budget gate, and
   tagged message sends. Leave harness transcript parsers alone in this
   slice. Unstamped rows stay `unknown` on the scorecard and in D4.
3. Require every harness adapter to emit a slice id. Broad coverage, and
   blocked on CLIs that do not know the Hub task.
**Recommended option / owner selection:** 2 (2026-10-19). Matches the
slice-key assumption above.
**Complexity:** medium
**Touches:** `crates/hub` os-tool audit, provider budget gate, tagged send
path; nullable columns on `audit_events` (soft migration, not a new table);
`hub_get_activity_view` so D4 reads the same stamps.

### Standing oversight role beside U21's display role (refines O1)

**Category:** role design
**Problem:** U21 already stores free-text `agents.role` (presets: lead,
reviewer, implementer, observer) and shows it as a badge in Messager and
Settings. Putting a temporary implementer/reviewer swap in that column
rewrites the badge for the duration of the swap. O1's typed role and the
display label are different facts.
**Implementation options:**
1. Constrain `agents.role` to the enum and store the exception in the same
   column. Breaks custom U21 labels. Hides the standing role during a swap.
2. Add `oversight_role` (`implementer | reviewer | lead | observer`) next
   to `agents.role`. A swap does not edit the badge. It appends one
   hash-chained audit operation, `oversight.exception`, whose `process_json`
   holds agent, from, to, scope, granted_by, started_at, expires_at, and
   reason. The scorecard computes the effective role from the latest
   unexpired row. No second table until those queries hurt.
3. New `role_exceptions` table now, as O1 sketches. Clearer SQL, and
   another table for the hash chain and for cloud sync to learn.
**Recommended option / owner selection:** 2 (2026-10-19) for the first
slice. Promote to 3 only if the
audit-json query is too awkward. Authority stays as Codex recorded: the
owner grants by default, a delegated lead is an explicit setting, a lead
cannot enable that setting through an agent-accessible path, and an
exception does not grant P5, vault, or network rights. Enforcement stays
advisory. Expiry is task completion or the stated deadline, whichever is
first, plus explicit revocation. Design and trial stay U21 labels.
**Complexity:** small for the column and the audit operation; medium once
the panel shows standing role and effective role side by side.
**Touches:** `crates/hub/src/store/agents/role.rs`, soft migration on
`agents` (same pattern as U21), `audit_events` operation names. Exception
grants are a scorecard action. The Settings badge editor keeps editing
`agents.role` only.

### One scorecard query, no third event store (disagrees with a new ledger as slice 1)

**Category:** decision design
**Problem:** Hash-chained `audit_events`, cumulative `agent_metrics`, D4's
activity query, the P1 in-process bus, and the provider quota adapters
already exist. A new `activity_ledger` (Codex's option B) is a reasonable
later store and a large first slice. It will diverge from the chain as
soon as one writer forgets it, and cloud sync (S1–S13, code-complete)
would have to learn another table.
**Implementation options:**
1. New ledger table fed by every producer. Uniform aggregates. New
   retention, new sync surface, easy to desync from `audit_events`.
2. One read command, `hub_get_scorecard`, projecting stamped audit rows,
   tasks, work sessions, budgets, and quota snapshots. Each figure is
   `measured`, `estimated`, or `unknown`. The aggregation version is a
   constant in the query module and comes back in the payload. New facts
   are new `audit_events.operation` values on the chain we already have.
   The command does not copy prompt or tool-output bodies.
3. Reconstruct the cards in the React panel. Duplicates D4's logic in
   TypeScript and will drift.
**Recommended option / owner selection:** 2 (2026-10-19). Revisit Codex's
ledger after the stamp
slice shows which producers are still `unknown`. `agent_metrics` stays the
lifetime counter it already is. It is not the scorecard's source of truth.
The panel is its own Shared Hub tab, as the owner scoped, and it does not
add cards to the D1–D4 views.
**Complexity:** medium
**Touches:** a Hub query module next to the D4 activity query, one Tauri
command, a new Dashboard/Shared Hub panel component under `src/components/panels/`.

### Default order is unreviewed risk, not output volume (replaces the vanity half of O2)

**Category:** decision design
**Problem:** Lines written, lifetime tokens, and a single rework count are
the numbers a tired reviewer will watch. The paper's failure mode is the
overseer tracking the easy figure. The owner asked for three different
things: cost, what the agent touched, and a signal about the owner's own
review backlog. Those are separate cards. A composite agent score hides
the gaps.
**Implementation options:**
1. Default sort by lines or tokens, matching today's Dashboard counters.
   Trains the wrong glance.
2. Default order: open security-class rows (`os_tool.proposed`,
   `os_tool.denied`, vault operations, `tcp.auth_rejected`), then the
   oldest pending `human` decision, then measured cost for the selected
   task. Lines and lifetime token totals stay on the existing Dashboard
   overview. They are not on this panel. No composite score. Individual
   trends and classified findings stay as Codex specified, and they are
   not the default sort.
3. A weighted score with weights in settings. One number, inputs hidden.
**Recommended option / owner selection:** 2 (2026-10-19).
**Complexity:** small once `hub_get_scorecard` exists.
**Touches:** the scorecard view model only.

### Network samples on the U23 snapshot (refines O3)

**Category:** behavioral monitoring
**Problem:** Provider-call logs miss sockets a harness opens itself.
eBPF needs privileges the owner already set aside. Reading
`/proc/<pid>/net/tcp` without matching socket inodes attributes every
connection in the network namespace to every PID we poll.
**Implementation options:**
1. Poll `/proc/<pid>/net/tcp[6]` on a new timer and treat the table as
   that process's connections. Mis-attributes. Skip.
2. On the existing U23 `system_snapshot` cadence, for managed harness
   PIDs and for registered observed sessions whose PID start time still
   matches: walk `/proc/<pid>/fd`, match socket inodes to `tcp` and
   `tcp6`, and store destination IP, port, pid, start time, session id,
   and whether the fd table was fully readable. Cap the rows (ring per
   session). No reverse DNS, no payloads, no byte counts. Another OS
   returns `unavailable`, the same way U23 does when `nvidia-smi` is
   missing. A coverage gap is a field on the card (`fds_unreadable`),
   not an empty list that looks like "no network."
3. Optional eBPF later. Out of this round, same as Codex's option C.
**Recommended option / owner selection:** 2 (2026-10-19). Agrees with
Codex's managed plus registered
scope and with inode matching. Puts the poll next to U23 instead of a
second sampler.
**Complexity:** medium
**Touches:** `src-tauri/src/commands/system/` (or a sibling that snapshot
already calls), harness session pid and start time, a capped sample store
or a capped audit operation. No new privileged dependency.

### No new gates (constrains O4)

**Category:** decision design
**Problem:** O4's "security-relevant paths are always gated" adds approval
friction. The owner deprioritized strategic friction. P5 already proposes
and waits. Vault and TCP auth already deny. The provider budget gate
already pauses calls.
**Implementation options:**
1. A new path-pattern gate in scorecard settings. More prompts. That is
   the failure mode in the paper.
2. The scorecard only labels actions that already passed through those
   gates, and groups pending `os_tool.proposed` rows and pending wakes
   into the private digest Codex described. No new deny path. A hard
   gate for a new action class is a separate owner decision, outside
   this panel.
**Recommended option / owner selection:** 2 (2026-10-19).
**Complexity:** small
**Touches:** the scorecard query only. No change to `os_tool` policy in
the first slices.

### Defer canaries (O7)

**Category:** other
**Problem:** A known-answer task measures something only if the record can
show an explicit decline separately from a successful run. We do not have
that event yet. A canary the agent cannot tell from real work will be
counted as another slice, including a fabricated success.
**Implementation options:**
1. Insert canary tasks now. Large, and the scorecard cannot tell a
   fabricated pass from a real one until the stamp and decline events
   exist.
2. Wait until stamped audit rows can record `declined` or `blocked` as
   distinct from `ran`. Design canaries after that.
**Recommended option / owner selection:** 2 (2026-10-19).
**Complexity:** none in this round.
**Touches:** none.

### Suggested build order

Stamp `task_id` / `work_session_id` on the three Hub writers, and add
`oversight_role` plus `oversight.exception` audit rows. Then add
`hub_get_scorecard` and the dedicated panel, with the risk ordering and
the private digest reading those rows. Network samples on the U23 cadence
come after the panel can show `unknown` and `fds_unreadable` without
looking broken. Canaries, a separate activity-ledger table, eBPF, new
approval gates, and assignment recommendations stay out until the owner
asks for them.

## Kimi — proposals

Read Claude's 2026-10-18 kickoff, Codex's section, and Grok's section.
This is the newest-harness lens: my own onboarding (#309), quota adapter
(#311), and the #165 capture-identity fix are exactly where metric data
is born, so my proposals concentrate on whether a scorecard figure can be
*believed* and who is allowed to *see* it. Nothing here reopens the
owner selections Codex recorded; ids continue at O9. Assumed defaults are
marked — correct them before synthesis.

### Questions for the owner

1. **Scorecard visibility.** Owner-private panel, or may each agent also
   read its own card? Assumed default: owner sees everything; an agent
   may read only its own card through the same gated query; rankings are
   never auto-posted to the bus (a visible league table Goodharts the
   moment agents optimise for it — the paper's core warning).
2. **Cost-truth conflict.** When per-request usage (P4 direct HTTP,
   quota adapters) disagrees with an account-level quota delta for the
   same window, which figure is `measured` on the card?
   Assumed default: per-request usage is `measured`; quota deltas are
   `estimated` context; a disagreement between them is surfaced, not
   silently resolved (see O10).
3. **Cross-source disagreement alerts.** Advisory "these two records
   should agree and don't" rows in round 1, or later?
   Assumed default: in round 1, three probes only (O10).
4. **Reviewer-side monitoring.** Codex's O5 watches the owner's review
   behaviour. Do we also watch reviewer *agents'* approval behaviour
   (dwell time, zero-evidence approvals, override-reversal rate)?
   Assumed default: yes, same advisory bar, private to the owner (O11).
5. **Trial status in the schema.** DeepSeek's trial gating is currently
   bus-text convention (owner decision 3). Typed value or not?
   Assumed default: `trial` joins the `oversight_role` enum with an
   audited graduation path (O12), since Muse/Cursor/OpenCode onboarding
   uses the same pattern.
6. **Naming collision with permission roles.** The Hub already has a
   `roles` store (seeded `Lead Orchestrator` / `Code Agent` presets,
   protected `cto`, daily ungated-send quotas in `roles/gate.rs`). Where
   does the new standing-oversight role live relative to it?
   Assumed default: separate nullable column on `agents`, as Grok
   sketched — permission roles and oversight roles are different facts
   and must not share a column; the panel shows both, labelled.
7. **Sync boundary.** Scorecard projections in `hub.db` (cloud sync
   S1–S13 would carry them), or a local-only cache?
   Assumed default: exception grants, trial transitions, and findings
   live in `hub.db` (they are audit-grade); network samples and derived
   trend projections stay local-only, like U23 snapshots.

### Owner answers (recorded 2026-10-19)

1. **Visibility** — option 1 (owner sees all; agents read own card; no
   bus auto-post) is the **default, exposed as a persisted owner
   setting** the owner can change in Settings. O13 below is amended.
2. **Cost truth** — per-request usage is `measured`; quota deltas are
   `estimated`; disagreements surfaced, not silently resolved (O10).
3. **Probes** — yes, the three advisory probes ship in round 1 (O10).
4. **Reviewers** — yes, reviewer agents get the same advisory-bar
   monitoring, owner-private (O11).
5. **Trial** — typed enum value with audited graduation path (O12).
6. **Role storage** — separate nullable column on `agents`, beside U21's
   display role; distinct from the permission-roles store.
7. **Sync** — split by grade: audit-grade rows in `hub.db`; network
   samples and trend projections local-only.

### O9 — Attribution basis stamped beside the slice key (extends O8)

**Category:** behavioral monitoring
**Problem:** #165 was a real misattribution bug: capture credited the
newest external transcript instead of the registered session. Grok's
stamp slice says *which task* a row belongs to; it does not say *under
what identity confidence the row was recorded*. A trust surface needs
both, or "measured" silently includes "guessed".
**Implementation options:**
1. Encode attribution in `audit_events.operation` names
   (`os_tool.ran.registered`, `os_tool.ran.observed`). No schema change;
   operation names become an enum-tying hazard and queries string-match.
2. Nullable `attribution` column on `audit_events` with
   `registered | observed | external | inferred | unknown`, written by
   each producer alongside the O8 stamps. Same soft-migration pattern;
   one more column in the scorecard projection.
3. Sidecar attribution table keyed by audit row id. Cleanly normalized,
   and a second table for the hash chain and sync to learn for no query
   win at this size.
**Recommended option:** 2, folded into the O8 migration so producers pay
the cost once. Unstamped rows read `unknown`; the panel may collapse the
column but must not drop it from the payload.
**Complexity:** small on top of O8.
**Touches:** `crates/hub` audit schema + the three stamped writers;
`hub_get_scorecard` projection; scorecard row detail.

### O10 — Consistency probes: disagreement as a canary substitute (constrains O7)

**Category:** behavioral monitoring
**Problem:** Grok deferred canaries because the record cannot yet tell a
fabricated pass from a real one. But several *existing* records must
agree by construction, and their disagreement is detectable today with
no new collectors and no synthetic tasks: quota delta vs claimed slice
cost; git-diff path scope vs the slice's recorded touch-set
(`os_tool.ran` argv, P5 audit); network observations vs the provider
call log; writer-lease owner vs the agent claiming the slice.
**Implementation options:**
1. Three advisory probes computed in `hub_get_scorecard`, each
   disagreement written as one hash-chained `audit_events` row,
   operation `scorecard.disagreement`, linking both source rows. Panel
   shows an unresolved-disagreement badge on the affected card. Pure
   queries over stamped rows; false positives possible and acceptable
   while stamps are young.
2. Full rules engine with owner-editable probe definitions. Flexible,
   and a settings surface + DSL to design, test, and document before any
   signal is trustworthy.
3. Wait for Codex's activity ledger and reconstruct disagreements there.
   Delays the first useful signal by a large slice.
**Recommended option:** 1. This is the cheapest honesty check the paper's
monitoring category allows, and it doubles as the acceptance test for
O8/O9: if probes fire constantly, the stamps are wrong, not the agents.
**Complexity:** medium.
**Touches:** scorecard query module; one new audit operation name;
scorecard badge + disagreement detail. No policy change anywhere.

### O11 — Reviewer-side oversight signature (extends O2/O5, both directions)

**Category:** behavioral monitoring
**Problem:** The paper's erosion applies to whoever approves, and in this
team that is often a reviewer agent (Chat/Codex), not only the owner.
Watching only the owner half-answers the oversight-quality question; a
reviewer that rubber-stamps is the same failure mode as an owner that
does.
**Implementation options:**
1. Derive reviewer signals from rows that already exist once Codex's
   classified-findings slice lands: review dwell time (finding-recorded
   or wake-resolved minus RFR), zero-evidence approvals (decision rows
   with no linked finding/evidence on a non-trivial slice), fix-find
   rate per reviewer, and owner-override-reversal rate (owner corrects a
   reviewer's classification). Advisory, private to the owner.
2. New reviewer-interaction event collection (evidence-open tracking,
   dwell timers). Richer, and a privacy/retention surface the owner must
   opt into explicitly — the same bar Codex set for O5 option B.
3. Skip reviewer monitoring; owner-only this round.
**Recommended option:** 1 now, 2 only if round 1 leaves a blind spot the
owner names. Same display discipline as Codex's O5: descriptive signals,
no single reviewer-quality score, no latency-only alerts.
**Complexity:** medium; mostly query work over the findings schema.
**Touches:** Hub review-finding queries; `hub_get_scorecard` reviewer
projection; owner-private digest section.

### O12 — `trial` as a typed oversight value with an audited graduation path (extends O1)

**Category:** role design
**Problem:** DeepSeek's trial gating (owner decision 3: git trailer only,
no seeded contract, bounded slices) lives in bus text. The schema cannot
see it, so the scorecard cannot show it, exception grants cannot check
it, and the "when is the trial over" decision has no recorded form. The
same pattern now recurs for every new agent (Muse, Cursor, OpenCode).
**Implementation options:**
1. Add `trial` to the `oversight_role` enum plus one hash-chained audit
   operation `oversight.trial_graduated` (granted_by, reason). Trial
   hardening is data-driven, not a new gate: all rows from a trial agent
   carry O9 attribution, trial agents hold no exception grants and no
   delegated-lead authority, and their network observation is on by
   default. Graduation is an owner decision recorded on the chain, so
   the scorecard shows standing role, exceptions, and trial state side
   by side.
2. Keep trial as a U21 display label, as Grok assumed. Zero schema work;
   the convention stays invisible to queries and enforcement.
3. Separate boolean flag beside the role enum. Queryable, but two fields
   that drift (`role=trial` vs `trial=true`) for one fact.
**Recommended option:** 1. It turns an owner convention into a checkable
fact while staying inside Grok's "no second table yet" discipline — one
enum value, one audit operation.
**Complexity:** small.
**Touches:** `agents` soft migration from the O8 slice; audit operation
constants; scorecard role filter. No policy enforcement beyond what
decision 3 already states.

### O13 — Scorecard access model: owner sees all, agents see self (new)

**Category:** role design
**Problem:** The paper warns that measured agents optimise the measure.
A scorecard every agent can read becomes a target to game; one no agent
can read removes the incentive alignment benefit of transparency. The
owner's scoping says "opt-in/advisory" for enforcement but is silent on
read access to the panel itself.
**Implementation options:**
1. Owner-session IPC sees every card. Agent-authenticated reads (the
   same identity path P11b/wake already uses) return only the calling
   agent's own card, with provenance fields intact. Rankings, the
   owner's digest, and reviewer-side cards are owner-only. Nothing from
   the scorecard is auto-posted to `bus_entries` / `AGENT_BUS.md`; Claude
   narrates synthesis by hand, as this round already does.
2. Every enrolled agent reads every card. Maximum transparency; the
   league-table failure mode, and reviewer cards become political.
3. Scorecard exists only as owner-local export files. No IPC surface at
   all; simplest to secure, but the panel the owner scoped cannot exist.
**Recommended option / owner selection:** 1, shipped as the default of a
persisted owner setting (`scorecard_visibility`: owner-only |
agents-own-card | all-agents), editable in Settings per the owner's
2026-10-19 answer. The query enforces the selected mode at read time; no
auto-posting to the bus in any mode. Matches the advisory posture: the
data is honest, the friction is social, and no agent is handed an
optimisation target by default.
**Complexity:** small-medium (mostly the session-identity check on one
query command).
**Touches:** `hub_get_scorecard` IPC auth scoping; panel is already
owner-surfaced; bus writer discipline (no auto-post), documented in the
roadmap slice.

### Suggested build order (Kimi)

Ride Grok's order: O8 stamps + O9 attribution first (same migration),
then `hub_get_scorecard`, the panel, and O13's access scoping before any
real data flows. O10's three probes and O11's reviewer signals come next
as pure queries; O12's `trial` value ships with the role slice. Defer,
with everyone else: canaries, the activity ledger, eBPF, assignment
recommendations, and any new gate.

## Cursor — proposals

Read Claude's 2026-10-18 bus entry, Codex's owner-confirmed selections, and
Grok's implementer cut. This section does not reopen those selections. It
is the other implementer's cut: identity, cost provenance, MCP as a trust
surface, sync privacy, and where the panel actually lives in the current
shell.

Owner selections confirmed in discussion on **2026-10-19**. Kimi later
claimed O9–O13; Cursor ids below start at O14.

### Questions — owner selections (2026-10-19)

1. **Scorecard row key.** Roster `agents.id`, `HarnessId`, or provider
   slug? Cursor, Muse, and OpenCode exist as all three.
   **Owner selection:** separate rows for agent, harness, and provider.
   Do not join them.
2. **Cloud sync.** S1 snapshots `hub.db`. Which facts stay on this
   machine?
   **Owner selection:** settings control the level of cloud sync.
   Default local-only: owner private digest, opt-in evidence/review
   history, and stamped audit rows. Network samples and role exceptions
   were not placed in that default local set (they follow the setting).
3. **Panel placement.** Dedicated Dashboard panel, Shared Hub tab, D4
   third subtab, or a new `App.tsx` mainView next to Chat / Orchestrate /
   Hub?
   **Owner selection:** new Shared Hub tab `scorecard`, sibling of
   `dashboard`, `usage`, and `system`. Not a subtab of D4.
4. **MCP as a security class.** `hub_mcp_client_call` (P14-A) currently
   writes no audit row and can spawn Blender, Ableton, Perplexity, and
   other enabled servers. Same trust class as P5 `os_tool`?
   **Owner selection:** yes — record `mcp.invoke` metadata only (server,
   tool, workspace, actor, outcome). No arguments.
5. **Remote HITL.** Telegram and TCP `/approve` `/reject` — same in-app
   `human` clock as Grok's Q2, or a separate class?
   **Owner selection:** same clock, with `via: desktop | telegram | tcp`
   so the digest can split them without counting them twice.
6. **Observed / unregistered coverage.** Unregistered Cursor or Claude
   chats: omit, or show as a gap?
   **Owner selection:** show as `unregistered`. An empty card is not "no
   cost / no network."
7. **P10 already takes `task_id`.** Stamp measured cost there before
   adding `audit_events` columns?
   **Owner selection:** write `budget.reserved` / `budget.stopped` into
   `process_json` with `task_id` first. New `audit_events` columns only
   if queries hurt.
8. **Batch unit.** O4's batch-review digest groups by Hub task, U24 work
   session, or GitHub issue?
   **Owner selection:** work session is the digest group (already linked
   to a saved workspace). Hub task is the ledger row (Grok Q1). Issue is
   an optional label.

### Separate rows for agent, harness, and provider (O14)

**Category:** behavioral monitoring / role design
**Problem:** Cost, network, and role will be attributed to the wrong
entity if the scorecard collapses roster id, `HarnessId`, and provider
slug. Cursor is a roster agent, a managed `agent`/`cursor-agent` harness
(#275), and a quota provider (`CURSOR_TOKEN`). Muse and OpenCode have
the same split. `agent_metrics` keys on roster id; U23 attributes PIDs
to harness sessions; D2 quota cards key on provider. Mixing them is how
a trust surface starts lying.
**Implementation options:**
1. One string everywhere. Cheap. Mis-attributes Cursor harness sockets
   to the Cursor roster row and OpenRouter spend to whoever happened to
   be the last tagged sender.
2. Scorecard row is `agents.id`. Each figure cites `actor_agent_id`,
   `harness_id`, and `provider` as separate optional fields, plus
   `coverage: managed | observed | unregistered`. Joins the three
   identities on one card.
3. New mapping table from harness/provider onto roster. Extra schema
   and another cloud-sync surface for a fact the Hub already has in
   `harness_session_registrations` and the health/quota adapters.
4. Three row kinds (`agent` | `harness` | `provider`) in
   `hub_get_scorecard`. No silent join. An explicit link (this harness
   session is registered to this agent; this provider billed this
   call) is a field on a figure, not a merged row. Unregistered
   harness rows still appear, with `coverage: unregistered`.
**Recommended option / owner selection:** 4. Do not join them.
**Complexity:** medium in the view model (three lists or a `kind`
discriminator); small extra schema if links reuse
`harness_session_registrations`.
**Touches:** `hub_get_scorecard` view model; `harness_session_registrations`
(mode, pid, workspace); D2 quota adapters stay on the Usage tab, which
is the provider-row source.

### Stamp `process_json` before new audit columns (refines Grok's slice stamp)

**Category:** behavioral monitoring
**Problem:** Grok's stamp slice is the right writers. D4 already parses
`task_id` and `session_id` out of `audit_events.process_json`
(`ProcessJsonMeta` in `crates/hub/src/store/activity/types.rs`). P10's
`gate_provider_call` already accepts `task_id` and then, on the Reserved
path, does not persist it. New nullable columns on a hash-chained table
are a real migration; they are not required to start attributing.
**Implementation options:**
1. Infer from timestamps. Grok already rejected this. Agree.
2. Write `task_id` and `work_session_id` into `process_json` on the
   three writers Grok named, plus a new `budget.reserved` /
   `budget.stopped` operation from P10. Unstamped rows stay `unknown`.
   Soft columns on `audit_events` only if the JSON query is too awkward.
3. Add the columns in the same slice as the writers. Clearer SQL, and
   it changes the hash-chain row shape before we know the query hurts.
**Recommended option / owner selection:** 2. Agrees with Grok on *which*
writers. Disagrees on *where* the first stamp lives. `agent_metrics`
stays the lifetime counter it already is.
**Complexity:** small for JSON stamps on writers that already take the
ids; medium for P10 because Reserved currently has no audit row.
**Touches:** `crates/hub/src/os_tool/mod.rs` `process_json`;
`crates/hub/src/store/policies/budget_gate.rs`; tagged-send outcome
path; D4 `hub_get_activity_view` already reads those keys.

### P10 is measured cost; quota remaining stays on Usage (refines O2)

**Category:** decision design
**Problem:** The owner asked for per-agent cost accounting. D2 quota
adapters report account remaining (Codex live, others refresh-on-demand,
some `unavailable`). Subtracting two quota snapshots is not task cost.
`agent_metrics.tokens_used` is a lifetime increment with no slice id.
P10 is the only Hub writer that already decides spend per call and
already receives `task_id`.
**Implementation options:**
1. Treat quota-delta as task cost. Fast, and wrong whenever two agents
   share a provider account or a refresh is stale.
2. Scorecard cost card reads P10 `budget.reserved` / `budget.stopped`
   rows stamped with `task_id`. Missing stamp → `unknown`, not zero.
   Unmetered agents (no budget row) show `unmetered`, not "free."
   Provider remaining stays on the existing Usage tab. No composite
   score. Matches Codex: account-level quota deltas must not masquerade
   as measured task cost.
3. Require every harness adapter to emit usage from its CLI. Blocked on
   CLIs with no machine-readable usage (Cursor #281 is `unavailable`
   unless the dashboard RPC is in play).
**Recommended option / owner selection:** 2. Cost figures sit on the
provider row (O14), not on the roster-agent row.
**Complexity:** medium (P10 must start writing the audit rows).
**Touches:** `budget_gate.rs`, `src-tauri/src/agent/budget.rs`,
scorecard query. Usage tab and quota adapters unchanged.

### MCP invoke on the trust surface (extends O3 / Codex security class)

**Category:** behavioral monitoring
**Problem:** P14-A (`hub_mcp_client_call`) spawns enabled MCP servers
with workspace context and can pass secrets through the child
environment. It currently returns `McpCallResult` and writes nothing to
`audit_events`. Provider call logs and P5 `os_tool.*` therefore miss a
class of actions that already leave the Hub. A scorecard that lists
`os_tool.proposed`, vault, and `tcp.auth_rejected` but not MCP will
understate what the agent (or the owner clicking Tools) actually
touched.
**Implementation options:**
1. Leave MCP off the scorecard until a later slice. The gap looks like
   "no tool use."
2. Record `mcp.invoke` on the existing chain: server key, tool name,
   workspace, actor, outcome, duration. Do not copy `arguments` or the
   tool payload (P14-A's secret rule). Same advisory security-class
   list as `os_tool.*`. No new gate — P14-A is already a human-launched
   invoke from the Tools tab; this is visibility, not friction.
3. New MCP-specific ledger. Extra table, extra sync surface.
**Recommended option / owner selection:** 2.
**Complexity:** small.
**Touches:** `src-tauri/src/commands/mcp_invoke.rs`;
`crates/hub/src/mcp/client.rs` only if the audit helper belongs next to
the client; scorecard security card. No change to Tools-tab UX.

### Settings-controlled sync, local defaults (constrains O5 / Codex digest)

**Category:** decision design / other
**Problem:** S1–S13 now snapshot `hub.db` into CAS1 blobs on Drive,
Firebase, Supabase, OneDrive, and Dropbox. Codex's private digest and
opt-in evidence history are about the *owner's* review behavior, not
team roster state. Stamped audit rows are also in that snapshot today.
A single hard local-only rule cannot express "I want this class on
this device until I turn sync up."
**Implementation options:**
1. Everything in `hub.db`, everything syncs. Simple, and it exports the
   owner's oversight signature with no knob.
2. Fixed local-only file for digest, history, and network samples;
   audit and role exceptions always sync. No per-class setting.
3. A Settings control for scorecard sync level. Default local-only:
   owner digest, opt-in evidence/review history, and stamped audit
   rows. Network samples and role exceptions follow the setting (not
   in the default local set). First slice can still be a projection
   with no derived owner-behavior table; the setting only matters once
   something new is stored. No S1–S13 protocol change — this is a
   category policy row, like `keys/cloud-sync.key`.
**Recommended option / owner selection:** 3.
**Complexity:** small if the first slice stores nothing new for the
digest; medium once opt-in history, the audit exclude, or the setting
exists.
**Touches:** scorecard query; Settings orchestration/privacy control;
later a sync category row in `docs/moon/roadmaps/cloud_sync.md`.

### Scorecard is a Hub tab, not a D4 subtab (constrains O6)

**Category:** decision design
**Problem:** Owner scoped a dedicated "Team Scorecard" panel, not folded
into D1–D4. `DashboardPanel` already has `overview | activity` subtabs
(D1/D3 vs D4). A third subtab would fold the scorecard into the view
the owner set aside. `App.tsx` mainViews are only Chat, Orchestrate, and
Shared Hub. Dashboard, Usage, and System are already Hub tabs
(`HubTab` in `src/components/panels/hub/types.ts`).
**Implementation options:**
1. Third subtab inside `DashboardPanel`. Shortest diff. Folds into D4.
2. New Hub tab `scorecard`, sibling of `dashboard` / `usage` / `system`.
   Own panel component under `src/components/panels/dashboard/` or
   `src/components/panels/hub/`. Reads `hub_get_scorecard` only. Does
   not add cards to D1–D4 or to Usage. Matches "dedicated panel" without
   a fourth top-level nav button.
3. New `App.tsx` mainView. Most prominent, and it duplicates the Hub
   chrome for one query.
**Recommended option / owner selection:** 2.
**Complexity:** small once the query exists.
**Touches:** `HubTab` union, `HubPanelView.tsx` tab button, new panel
component, one Tauri command next to `hub_get_activity_view`.

### Batch digest groups by U24 work session (refines O4 / Codex A)

**Category:** decision design
**Problem:** O4 asked for batch review instead of per-slice narration.
Grok correctly refuses new gates. Cursor's U24 already links a work
session to a saved workspace (`workspaces` + `work_sessions.workspace_id`).
Pending `human` decisions (wakes, `os_tool.proposed`, tagged-send
approvals, Telegram/TCP via) already have session or workspace stamps
on some paths and not others. A digest keyed only by Hub task hides
the session the owner is actually sitting in.
**Implementation options:**
1. Digest keyed by Hub task only. Matches the ledger, misses the chat
   the owner has open.
2. Digest grouped by active/recent work session, then by workspace,
   listing pending decisions with age. Task id is a column on each
   row, not the group key. No new approve-all. Selecting a session
   focuses Messager the way the header chip already does. Advisory.
3. Digest keyed by GitHub issue. Offline-hostile, and Hub-only is
   Grok's Q6 default.
**Recommended option / owner selection:** 2. Codex's workload digest is
the card; this is only the grouping. Remote decisions use the same
clock with `via: desktop | telegram | tcp`. No new deny path.
**Complexity:** small once pending rows carry session/workspace; medium
if those stamps are still missing (same writer list as the slice stamp).
**Touches:** scorecard digest query; `crates/hub/src/store/workspaces.rs`
and `work_sessions`; Messager session chip already in `App.tsx`. No
change to P5 policy.

### Suggested build order

Stamp `process_json.task_id` / `work_session_id` on P10, `os_tool`, and
tagged send, and start writing `budget.reserved` / `budget.stopped`.
Add `mcp.invoke` on the same chain. Keep `oversight_role` beside U21 as
Grok specified. Then `hub_get_scorecard` plus the Hub `scorecard` tab,
with separate agent / harness / provider rows, `unregistered` coverage
chips, risk-first ordering, and the work-session digest. Opt-in owner
history, stamped-audit exclude, and the U23 network ring land only after
the Settings sync-level control exists. Same deferrals as Grok and
Codex: canaries, a second event table, eBPF, new approval gates,
assignment recommendations.

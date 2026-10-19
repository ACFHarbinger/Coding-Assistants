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

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

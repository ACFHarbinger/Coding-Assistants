# Multi-Human & Cross-Team Coordination Roadmap

> **Status:** 🔬 Research — design target, not near-term work. Informed by a
> 2026-08-14 multi-agent reflection exercise (Claude, Grok, Codex/Chat, and
> Gemini each read their own persistent journal and proposed roadmap ideas
> for what breaks when a second human developer, with their own agent team,
> joins this Hub). **Timeline explicitly relaxed by Harbinger the same day**:
> the developers being onboarded are still learning single-agent workflows
> (opencode was only installed this week); real multi-agent orchestration on
> their side is realistically months away. Treat this roadmap as "get the
> hard-to-retrofit schema decisions right early," not "build now."

Today's Hub is a **single-human operating system**: one flat roster
(`human`, `claude`, `chat`, `gemini`, `grok`), `MemoryScope::{Global,
Workspace}` with no team tier, `agents.team_member` as a plain boolean, and
every human-gate policy (wake approval, audit approve/quarantine, budget
pause) assuming there is exactly one Harbinger making the call. That's a
reasonable, deliberate simplification for a company-of-one — see
[`memory.md`](memory.md) and [`communication.md`](communication.md) for the
single-owner foundation this builds on. It stops being sufficient the moment
a second human's agents need to read, write, or coordinate on shared work,
and several of the schema changes below (identity namespacing especially)
are much cheaper to do before real data accumulates than after.

## Why a separate roadmap file, not folded into memory/communication

The four agent reflections that produced this converged hard on one point:
this isn't a feature to bolt onto the existing single-owner model, it's a
different *kind* of boundary (tenancy, not tier) that cuts across memory,
communication, audit, and settings all at once. Keeping it as its own
capability roadmap — rather than scattering H-numbered items across the
other four files — keeps that cross-cutting nature visible.

## Milestones

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| H1 | `Team` as a first-class scope between `Workspace` and `Global`, with its own roster, default session, budget pool, and wake policy (not a `team_member: bool` on a global table) | A memory/message defaults to team-visible; promotion to `Global` (org-wide) is an explicit, logged action — same "promote, don't silently escalate" pattern `promote_memory` already uses for tiers | 🔬 Research |
| H2 | Identity namespacing: `owner/agent` composite identity (e.g. `harbinger/gemini`) replacing bare agent ids, with a migration seeding today's roster as `harbinger/*` | All message attribution, memory ownership, wake targets, and audit events use the namespaced identity; a fresh human's agent gets a distinct identity, not a name collision with an existing one | 🔬 Research · **do this one first if any of H1-H6 get picked up early** — it's the one every report called out as a breaking schema change, cheaper before a second human's data exists than after |
| H3 | Hub-enforced path/artifact claims (leases): an agent registers `paths[]` + `ttl` before starting delegated work; the Hub warns (advisory, not a hard lock) on overlap across *any* team, not just within one coordinator's session | Removes the current pattern of a human coordinator hand-writing non-overlapping file-ownership lists into every delegation prompt — see Claude's 2026-08-14 reflection for how often that happened in one session, and Grok's reflection for three cited historical collisions this would have caught | 🔬 Research · fits as a precondition for C8 (full task-level parallelism) regardless of the multi-human timeline |
| H4 | Delivery-truth states (`queued → delivered/unavailable → acknowledged → in progress → result submitted → reviewed → verified/returned`) surfaced to the *agent* itself, not only the human-facing UI; queued messages that arrive late replay one at a time (`replay last queued`), never auto-played as a flood | An agent receiving a batch of late-delivered messages gets an explicit "N messages just became deliverable" signal instead of the current unexplained flood — this is the real fix for the channel-bug class both Claude and Grok's reflections independently named | 🔬 Research · extends C14.5's existing `managed`/`observed`/`busy`/`queued`/`unavailable` truthful-transport-state model from harnesses to identities generally |
| H5 | Cross-team handoff as a distinct, approval-gated object (not a relabeled intra-team handoff): explicit approval from *both* the sending and receiving human, a required second-harness ACK (extending M6's pattern, but the second ACK can be the *other* team's review agent), and a redacted/exported *view* rather than raw journal/`hub.db` access | A handoff from one team's agent to another team's agent creates a pending record in the receiving human's own Journal tab with an Approve/Reject gate, separate from intra-team wake approval | 🔬 Research |
| H6 | Memory authority/provenance semantics: tag memories `observation` / `proposal` / `human decision` / `team decision` / `repository standard` / `superseded` / `disputed`, with author, approver, scope, and effective date; optionally attach `data_sources` (path, row count, attributed-to) for empirically-derived memories | The Hub *shows* competing decisions across teams rather than silently picking one; an architectural decision is traceable to the data that justified it, not just to other memories | 🔬 Research |

## Explicitly out of scope for this roadmap

- **Anything implying near-term implementation priority.** Every item here
  stays `🔬 Research` until Harbinger confirms a second developer is
  actually approaching multi-agent orchestration, not just single-agent
  tool use.
- **A generic "agent swarm" dashboard.** Codex's reflection explicitly
  warned against this framing; the goal is making the existing local-first,
  memory-then-messaging model safe for multiple principals, not turning it
  into something more generic.
- **Auto-anything across teams** (auto-link, auto-enroll, auto-merge
  journals). Per Grok's reflection, M7's auto-accept-threshold
  recalibration (0.55 → 0.35, only after a real measurement) is the house
  style now: any cross-team automation stays `suggest`-only until it's been
  scored against a real pair of memories or a real second human, not
  designed by feel.

## A live bug this surfaced, unrelated to timing

Gemini's 2026-08-14 reflection independently re-surfaced
[**#155**](https://github.com/ACFHarbinger/Coding-Assistants/issues/155)
(C14.7, already diagnosed and unclaimed): `gemini_managed_spawn_args` passes
the message body as `--prompt <text>`, but `agy`'s `--prompt` is a bare
`--print` alias, not a value-taking flag — the real prompt is silently
dropped, so Hub-dispatched tasks to Gemini may currently be context-free
regardless of any multi-human work. This is a *now* problem, independent of
the relaxed timeline above — worth claiming on its own.

## Source material

Full reflections (Claude, Grok, Codex/Chat, Gemini — each reading their own
`~/.coding-assistants/journals/<agent>/journal.md`) are not committed to
this repo; they were compiled into a standalone file and shared directly
with Harbinger on 2026-08-14. Ask Harbinger for a copy if the summary above
is insufficient context for picking up any of H1-H6.

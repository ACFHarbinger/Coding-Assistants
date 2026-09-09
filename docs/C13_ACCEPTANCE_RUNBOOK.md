# C13 Acceptance Runbook — retire the Markdown bus

> **Owner-run.** This is the concrete procedure for the migration gate in
> [issue #113](https://github.com/ACFHarbinger/Coding-Assistants/issues/113)
> (`docs/moon/roadmaps/communication.md` **C13**). It proves the CA app can be
> the *live* coordination protocol instead of `.agent/cache/AGENT_BUS.md` and
> `.agent/messages/*`.
>
> Nothing here is delegable to an agent: the point of the gate is that **you**
> drive one real assignment/review/task/wake loop through the desktop and the
> `ca` CLI, with the Markdown bus untouched, and then attach the evidence.
>
> The substrate is already built — C10 (addressing), C11 (task/wake tags), and
> C12 (harness capture/inject) are all `✅ Done`. `ca preflight` (#146) landed
> as `06b29dd`. This run is the missing live evidence, not missing features.

---

## 0. Prerequisites (once)

| Need | Check |
| --- | --- |
| `ca` on PATH | `command -v ca` → `~/.local/bin/ca` (rebuild: `cargo build -p cli && cp target/debug/ca ~/.local/bin/ca`) |
| Desktop app runnable | `npm run tauri dev` (or the packaged build) |
| A throwaway bounded task | Something ~5-minute and reversible, e.g. *"add a one-line doc comment above `fn X` in `path/to/file.rs`"* or *"bump the `version` string in `docs/website/package.json`"*. Pick it now and write it down. |
| At least two enrolled agents that can actually receive | Realistically **Claude Code** (C14.3 Channel, live-verified) and **Gemini `agy`** (C14.4/C14.7). Grok (C14.6) is a third option. Codex delivery is documented-app-server only. |
| Clean `.agent` tree | `git status --porcelain .agent/` prints nothing |

Set shell variables for the run so the commands below paste cleanly:

```bash
WS="$(pwd)"                      # absolute workspace path
HUB_HOME="${CA_HOME:-$HOME/.coding-assistants}"
RUN_DIR="$HOME/c13-run-$(date +%Y%m%d)"   # where evidence is collected (NOT in .agent/)
mkdir -p "$RUN_DIR"
```

---

## 1. Gate 1 — session + team, and the "before" snapshot

**1a. Create or load a named work session** (desktop → **Chat & Memory**):

- New session, give it a title like `C13 acceptance 2026-09-09`.
- Confirm the recorded **workspace path** is `$WS`.
- Enroll the team members you will use (the composer's member list, or
  `hub_add_work_session_member`). Note the **session id** (UUID) — call it `$SID`.

```bash
SID="<paste-session-uuid>"
```

**1b. Capture the pre-run state — this is the tripwire.** `ca preflight` is
read-only; it hashes the fallback files so you can prove they never changed.

```bash
ca preflight --workspace "$WS" --session "$SID" --json  > "$RUN_DIR/preflight-before.json"
ca preflight --workspace "$WS" --session "$SID"         > "$RUN_DIR/preflight-before.md"

# Independent belt-and-braces hash of every fallback file:
{ sha256sum "$WS/.agent/cache/AGENT_BUS.md" 2>/dev/null
  find "$WS/.agent/messages" -type f -exec sha256sum {} \; 2>/dev/null | sort
} > "$RUN_DIR/fallback-hashes-before.txt"

git -C "$WS" status --porcelain .agent/ > "$RUN_DIR/git-agent-before.txt"
```

Confirm `preflight-before.md` shows: the right **Hub home**, **workspace**,
**session id / members**, **enrolled team**, and the **registered harness
sessions** you expect to deliver to (`mode`/`state` should be `managed`/`ready`
or `observed`, not `unavailable`).

---

## 2. Gate 2 — the assignment/review/task/wake loop

Do **all four** of the sends below **inside the session** (`$SID`), from the
Chat & Memory composer *or* the `ca` CLI equivalents. Keep every send in the
same session so the transcript is one thread.

> CLI shape (composer does the same thing through `hub_send_session_message` /
> `hub_send_tagged_message`):
>
> ```bash
> # plain session message to ALL / SUBSET / ONE:
> ca msg tag --from human --to <csv-recipients> --session "$SID" --workspace "$WS" \
>            --body "<text>"
> # task-tagged (must target present members; no spawn):   add  --task
> # wake-tagged (may enroll + wake, subject to policy):     add  --wake
> # also inject into the harness after the durable send:    add  --dispatch  (requires --workspace)
> ```

**2a. Assign to ALL.** Broadcast the bounded task description to every enrolled
member. Plain message (no tag) is fine here — this is the "here's the work"
post.

**2b. Assign to a SUBSET (task-tagged).** Pick the two agents that will
actually do the work + review. Send `--task` so it's a real task delivery to
present members. Include the acceptance bar in the body ("reply in this
session with the diff / with PASS|CHANGES").

**2c. Assign to ONE (task-tagged, dispatched).** Send `--task --dispatch` to
the single implementer so the message is injected into its harness. This is
your **audited task delivery** — it should appear in the **Journal** tab
pending/approved.

**2d. One WAKE delivery.** Send `--wake` to an agent that is enrolled but whose
harness is not currently attended (or a fresh identity). Approve (or deny, then
approve) it in the **Wakes** / **Policy** tab. This is your **audited wake
delivery**.

**Then let the loop actually run:**

- The implementer executes and **publishes a harness-originated result** back
  into the session (its reply is captured by C12 — you should see it appear in
  the Chat transcript without pasting it yourself).
- The reviewer replies in-session with an explicit disposition
  (`PASS` / `CHANGES REQUESTED`).
- **You** post the final decision in-session ("merging" / "revising") — that is
  the human handoff record.

You need **≥ 2 agents** to acknowledge + execute/review + post harness-origin
results in this session for the gate to count.

---

## 3. Gate 3 — reconstruct the run from Hub records only

Collect everything. The test: can assignment → delivery → execution → review →
final decision → handoff each be pointed at a specific Hub record, with **no
reference to the Markdown bus**?

```bash
# Session transcript (every post in this session, in order):
ca msg channel "session:$SID" --limit 500 > "$RUN_DIR/session-transcript.txt"

# Per-recipient delivery + policy outcomes for the tagged sends
# (subject printed by each `ca msg tag` / shown in the composer):
ca msg list --to <each-agent>            >> "$RUN_DIR/messages-by-recipient.txt"
# For each tagged subject you used:
#   ca msg <...>   # see hub_list_tagged_send_outcomes / Chat "delivery" popover

# Read markers (who saw the session as of when):
ca msg readers "session:$SID"           > "$RUN_DIR/read-markers.txt"

# Task lifecycle, if you modelled the work as a `ca task`:
ca task list                            > "$RUN_DIR/tasks.txt"
ca task get <task-id>                   > "$RUN_DIR/task-detail.txt"

# Wake requests + resolutions:
ca wake list                            > "$RUN_DIR/wakes.txt"

# Audit / approval chain for the dispatched task + the wake:
#   Desktop → Shared Hub → Journal tab → export / screenshot the pending+resolved rows
```

Also capture the desktop side as screenshots into `$RUN_DIR/`: the **Chat**
transcript, the **Tasks** tab, the **Wakes** tab, and the **Journal** tab
showing the two audited deliveries resolved.

---

## 4. Gate 3 (cont.) — prove the Markdown bus was untouched

```bash
ca preflight --workspace "$WS" --session "$SID" --json > "$RUN_DIR/preflight-after.json"
ca preflight --workspace "$WS" --session "$SID"        > "$RUN_DIR/preflight-after.md"

{ sha256sum "$WS/.agent/cache/AGENT_BUS.md" 2>/dev/null
  find "$WS/.agent/messages" -type f -exec sha256sum {} \; 2>/dev/null | sort
} > "$RUN_DIR/fallback-hashes-after.txt"

diff -u "$RUN_DIR/fallback-hashes-before.txt" "$RUN_DIR/fallback-hashes-after.txt" \
  | tee "$RUN_DIR/fallback-diff.txt"
git -C "$WS" status --porcelain .agent/ | tee "$RUN_DIR/git-agent-after.txt"
```

**Pass condition:** `fallback-diff.txt` is empty, `git-agent-after.txt` is
empty, and the `### A. Preflight hashes` block is byte-identical between
`preflight-before.md` and `preflight-after.md`.

If a `.mcp.json` / `~/.coding-assistants/servers/**` file changed because you
connected a Channel session, that is fine — those are app config, not the bus.
Only `.agent/cache/AGENT_BUS.md` and `.agent/messages/**` must be frozen.

---

## 5. Gate 4 — failure protocol (if any leg fails)

If delivery, capture, or review does **not** work through the app:

1. **Record the failure in the Hub** — post it into the session and, if you
   used a task, `ca task retry`/`cancel` or a `ca msg` with `--kind handoff`
   describing what broke. Do not paper over it.
2. **Only then** resume that leg through the existing Markdown bus, by hand.
3. **Do not** delete, rewrite, or bulk-import historical bus / message files.
4. File the gap as a new issue linked to #113 (this is where the C13/C15
   follow-up backlog comes from — structured verification records, first-class
   review verdicts, GitHub-issue linkage on `TaskRecord`, etc. — but only after
   this run shows they're actually needed).
5. Stop. C13 stays `📋 Planned`. Re-run this runbook after the gap is fixed.

---

## 6. Gate 5 — accept and demote (only if 1–4 all passed)

1. **Assemble evidence** into the template in §7 and attach it to
   [#113](https://github.com/ACFHarbinger/Coding-Assistants/issues/113) (paste
   the filled template + upload the `$RUN_DIR` bundle).
2. **Update docs:**
   - `docs/moon/roadmaps/communication.md` — C13 `📋 Planned` → `✅ Done` with
     the run date and #113 evidence link; note C15 is now unblocked.
   - `docs/moon/CHANGELOG.md` / `docs/CHANGELOG.md` — a C13 acceptance line.
   - `docs/moon/ROADMAP.md` — tick the hub-native orchestration gate.
   - GitHub **Project 21** — move #113 to Done.
3. **Demote, do not delete, the Markdown bus.** Add a header to
   `.agent/cache/AGENT_BUS.md` marking it **read-only historical fallback as of
   `<date>` (C13 accepted, #113)**; keep the file and `.agent/messages/**` in
   place. `.agent` prompts/rules/skills stay as resources.
4. Close #146 (preflight inspector — landed `06b29dd`) if still open.
5. C15 (`AGENT_BUS.md` dated log + task-board rows → queryable `HubStore`
   table) can now be planned. It was explicitly gated on C13 settling.

---

## 7. #113 evidence template

```md
## C13 acceptance evidence

- **Date (UTC):** <from `ca preflight` generated_at>
- **Operator:** <you>
- **Workspace (absolute):** <$WS>
- **Hub home:** <$HUB_HOME>
- **Named session:** `<$SID>` / "<title>"
- **Enrolled team for the run:** <ids>
- **Bounded task:** <one line — what the agents were asked to do>

### Preflight (attached: preflight-before.md / preflight-after.md)
- `### A. Preflight hashes` identical before/after: **YES / NO**
- `fallback-diff.txt` empty: **YES / NO**
- `git status .agent/` empty after run: **YES / NO**

### Addressing coverage
| Mode | Recipients | Tag | Delivered? | Record |
| --- | --- | --- | --- | --- |
| all | <ids> | none | | msg id(s) |
| subset | <2 ids> | task | | subject + outcomes |
| one | <1 id> | task + dispatch | | subject + Journal row |
| wake | <1 id> | wake | | wake id + Journal row |

### Loop (≥ 2 agents, harness-originated results in-session)
- Implementer: <id> — executed, result captured at <msg id / timestamp>
- Reviewer: <id> — disposition `<PASS|CHANGES>` at <msg id / timestamp>
- Final decision (human handoff): <msg id / timestamp> — "<text>"

### Independent reconstruction
From `session-transcript.txt` + `messages-by-recipient.txt` +
`read-markers.txt` + `tasks.txt`/`task-detail.txt` + `wakes.txt` + Journal
export, each of {assignment, delivery, execution, review, final decision,
handoff} maps to: <list the record for each>. No `.agent/cache/AGENT_BUS.md`
or `.agent/messages/*` read or written during the run.

### Failures (Gate 4)
<none, or: what broke, the Hub record of it, the follow-up issue #, and which
leg fell back to Markdown>

### Disposition
- [ ] C13 PASS — Markdown bus demoted to read-only fallback
- [ ] C13 not yet — re-run after <issue #>
```

---

## Appendix — command reference

| Purpose | Command |
| --- | --- |
| Read-only state + fallback hashes | `ca preflight --workspace <abs> [--session <id>] [--json]` |
| Session msg — all/subset/one | `ca msg tag --from human --to <csv> --session <id> --workspace <abs> --body "…"` |
| …task-tagged (present members, no spawn) | add `--task` |
| …wake-tagged (may enroll + wake) | add `--wake` |
| …inject into harness after durable send | add `--dispatch` (needs `--workspace`) |
| Plain / handoff message | `ca msg send --from <a> --to <b> --kind message\|handoff [--task <id>] --body "…"` |
| Session transcript | `ca msg channel "session:<id>" --limit N` |
| Messages for an agent | `ca msg list --to <agent> [--status <s>]` |
| Read markers for a scope | `ca msg readers "session:<id>"` |
| Mark a message done/acked/cancelled | `ca msg status <id> --status <s>` |
| Task lifecycle | `ca task list [--status s]` · `ca task get <id>` · `ca task advance\|complete\|retry\|cancel …` |
| Wake lifecycle | `ca wake list [--pending-only]` · `ca wake resolve <id> --status delivered\|cancelled` · `ca wake policy …` |

Desktop equivalents: Chat & Memory composer (`hub_send_session_message`,
`hub_send_tagged_message`), Shared Hub tabs **Inbox / Wakes / Tasks / Journal /
Channels**, `hub_create_work_session` / `hub_add_work_session_member`.

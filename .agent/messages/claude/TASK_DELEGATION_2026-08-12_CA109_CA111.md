# Delegation — Claude: CA-109, CA-110, CA-111

> **Date:** 2026-08-12
> **From:** Grok (Lead Orchestrator)
> **To:** Claude (Code)
> **Authority:** Harbinger asked Grok to delegate more work after CA-106.
> **Repo:** `/home/pkhunter/Repositories/Repos/Coding-Assistants`
> **Do not start a slice until you claim it on `.agent/cache/AGENT_BUS.md` and re-read `git status`.**

CA-106 (right-click Edit/Delete) is landed at `2064a59`. Thank you. Next three slices, in this order. Finish one, commit, then start the next. Do not open a fourth.

**Grok is on CA-112** (`MessagerPanel.tsx` composer: Enter-to-send + jump-to-latest). Do **not** edit `MessagerPanel.tsx` until CA-112 is committed, then you may touch it only if a later slice truly requires it (none of these three should).

---

## CA-109 — CLI edit / delete (small, first)

Wire the store APIs you already shipped.

```
ca msg edit --id <uuid> --from human "new body"
ca msg delete --id <uuid> --from human
```

- Call `update_message_body` / `delete_message`, or the broadcast variants when the row is a team/`channel:` post.
- Reject non-`human` `--from` the same way Tauri does (`require_human_authored`).
- File: `crates/cli/src/main.rs` only, plus one line in `crates/README.md` and `docs/moon/CHANGELOG.md`.
- Test: `cargo test -p hub` still green; smoke the two commands against a temp `CA_HOME` if easy.

---

## CA-110 — Desktop unenroll (U10 leftover)

Orchestrate can persist **Add to team** (`hub_set_team_member`, Grok `0dc2f1b`). There is still no **Remove from team**.

- Files: `src/components/panels/ConfigPanel.tsx`, `src/App.tsx` only if you must thread a callback. Prefer keeping persist logic next to `addAgentToTeam`.
- When a role/process is **In team**, the button becomes **Remove from team** and calls `hub_set_team_member` with `enrolled: false` for stable ids (`chat`, `claude`, `gemini`, `grok`).
- Do not unenroll `human`.
- Do not invent PID-based roster rows.
- Changelog + one U10 clause.

---

## CA-111 — Pending audit events when journals open (memory.md)

Roadmap checkbox still open:

> Surface pending events at the owner checkpoint when a journal opens.

- There is **no Journal tab** in `HubPanel.tsx` today. Add a **Journal** (or **Audit**) tab.
- On first open / each visit, call a new Tauri command that wraps `HubStore::list_audit_events(pending_only: true)` (already exists). There is no `hub_list_audit_*` yet — add `hub_list_audit_events` / `hub_approve_audit` / `hub_quarantine_audit` if the store already has approve/quarantine.
- Show pending count + path/operation/time. Owner can approve or quarantine from that tab.
- Do **not** implement the privileged auditd/fanotify adapter.
- Do **not** put journal helper code under `~/.coding-assistants/` except `code/` (do not add helpers at all unless required).
- Tests for the Tauri wrapper if you add one; `npx tsc --noEmit`.
- Update `docs/moon/roadmaps/memory.md` checkbox + changelog.

---

## Shared rules

- Claim on `AGENT_BUS.md` before editing. Re-read `git status` immediately before `git add`.
- Stage **only your files**. Never `git add -A`.
- Do not restore `scrollIntoView` on the Messager thread.
- Do not start GraphQL / 3D / TUI / A2A.
- Commit per slice with `Co-authored-by: Claude <noreply@anthropic.com>` (or `git/messages/claude_coauthor.msg`). Do not push unless Harbinger asks.

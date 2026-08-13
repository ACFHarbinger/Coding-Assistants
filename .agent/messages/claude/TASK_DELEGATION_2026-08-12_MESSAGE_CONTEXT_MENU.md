# Delegation — Claude: Messager message context menu (edit / delete)

> **Date:** 2026-08-12
> **From:** Grok (Lead Orchestrator)
> **To:** Claude (Code)
> **Authority:** Harbinger asked Grok to delegate this slice.
> **Repo:** `/home/pkhunter/Repositories/Repos/Coding-Assistants`
> **Do not start until you have posted a claim on `.agent/cache/AGENT_BUS.md` and re-read `git status`.**

---

## Task: CA-106 — Right-click Edit / Delete on Messager messages

Harbinger wants Messager-like **right-click options on a message bubble**: at least **Edit** and **Delete**.

### Why this is yours

Grok is fixing the auto-scroll-while-reading bug in `MessagerPanel.tsx` / `App.tsx`. Chat still owns CA-102 channel-query wiring in `cli` / `hub_cmds.rs` / `src-tauri/src/lib.rs` (those files may be dirty — **do not stage them**). You own a new, bounded slice: message mutation API + context menu UI.

### Product behavior

1. Right-click a message bubble (not the empty thread chrome) → custom menu, **not** the browser default.
2. Menu items: **Edit**, **Delete**. Optional later: Copy, Quote. Do not build a kitchen sink.
3. **Edit:** inline textarea on that bubble (or a small popover). Save writes the new body. Esc / Cancel discards.
4. **Delete:** confirm, then remove (or mark cancelled and hide) so the bubble leaves the channel view.
5. Team broadcasts are **N SQLite rows** (one per enrolled recipient) sharing a subject. Edit/delete **must** update or remove **every copy** of that post, not just the row that happened to render. New posts use `channel:<name>:<uuid>` as the shared subject; legacy posts use exact `channel:<name>` and can be grouped by `from_agent + body + created_at` second.
6. Only Harbinger (`from_agent === "human"`) can edit/delete in v1 unless you already have a clean permission helper. Do not let an agent silently rewrite another agent's line.
7. After save/delete, refresh the hub list (`onRefresh`). Do **not** call `scrollIntoView` on the thread.

### Suggested implementation shape

**Store (`crates/hub/src/store.rs`) — only if Chat is not mid-edit there. Re-read the file first.**

- `update_message_body(id, body) -> MessageRecord`
- `delete_message(id)` **or** `set_message_status(id, Cancelled)` plus list filters that hide cancelled
- `update_broadcast(subject, body)` / `delete_broadcast(subject)` when `subject` starts with `channel:` or `team:` / `private:`

Add a focused unit test: send-to-team, edit via broadcast subject, all copies match; delete, none remain visible.

**Tauri (`src-tauri/src/hub_cmds.rs` + `lib.rs`)**

- `hub_update_message` / `hub_delete_message` (or broadcast variants)
- Chat may have uncommitted CA-102 commands in these files. **Re-read, append, do not revert Chat's `hub_list_channel_messages` / `hub_list_message_memories`.**

**CLI (optional, nice):** `ca msg edit --id` / `ca msg delete --id`. Same write-confinement rule for `crates/cli/src/main.rs`.

**UI (`src/components/panels/MessagerPanel.tsx`)**

- `onContextMenu` on the bubble; `preventDefault`
- Small absolutely-positioned menu; click-away and Escape close it
- Match existing glass styles; no new CSS framework
- Keep Grok's scroll pinning (`scrollBoxRef` / `stickToBottomRef`). Do not restore the old `messagesEndRef.scrollIntoView` effect.

### Verification

- `cargo test -p hub`
- `npx tsc --noEmit`
- Right-click one `#general` “hi”, edit it, confirm **one** bubble changes (not four copies).
- Delete it, confirm it disappears.
- Scroll up in a long thread; poll must **not** drag the viewport down (Grok’s fix — do not regress).

### Docs

- `docs/moon/CHANGELOG.md` Unreleased
- One line on `docs/moon/roadmaps/ui.md` U10 if you add the menu

### Git

- Stage **only your files**. Never `git add -A`.
- Leave Chat’s remaining CA-102 dirt (`cli`, maybe `hub_cmds.rs`/`lib.rs` if you are not the one finishing those commands).
- Commit with `Co-authored-by: Claude <noreply@anthropic.com>` (or the repo’s `git/messages/claude_coauthor.msg`).
- Do not push unless Harbinger asks.

### Out of scope

- GraphQL, 3D, TUI, A2A
- Encrypted journals
- Changing wake policy
- Restyling the whole Messager chrome

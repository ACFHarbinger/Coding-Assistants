# #163 — UI-freeze audit (2026-08-29)

Fourth pass at #163 ("multiple features freeze the app UI for several seconds
with no feedback"). Prior rounds: `e36b0b2` (message send), `726f28c` (tab
switch / quotas / channel-connected), `fca8258` (harness inject/start/stop/
presence/detect/Grok-connect/tagged-send). Issue stays open as a *class*.

## Root cause (unchanged, verified in `726f28c`)

A non-`async` `#[tauri::command]` runs its body inline on the thread that
dispatches IPC — on Linux, the webview's own event-loop thread. Any slow sync
command freezes the **whole window** (dragging, tab switches, everything),
not just the calling panel. Fix pattern already established in this repo:
make the command `async fn` and move the real work onto
`tauri::async_runtime::spawn_blocking`, keeping a `_blocking` inner fn for
tests. Frontend: show a pending state on the caller.

## Command inventory

156 `#[tauri::command]` total — **32 async, 124 sync**. Most sync commands are
single local-SQLite calls (`open_store()?.…`) that return sub-millisecond and
are not worth touching. The freeze risk is the subset that shell out, touch
the filesystem beyond a trivial read, or do bulk work:

| Command | File | Blocking work | Status |
|---|---|---|---|
| `hub_export_markdown_git` | `messaging.rs` | 3× `git` subprocess (`rev-parse`/`add`/`commit`) | **fixed — this PR** |
| `hub_export_markdown` | `messaging.rs` | serialize whole store → markdown tree on disk | **fixed — this PR** |
| `hub_read_avatar_preview` | `hub/avatar.rs` | `std::fs::read` whole image + base64 encode | open — batch 2 |
| `hub_set_agent_avatar` | `hub/avatar.rs` | `std::fs::read` image (path branch) + store write | open — batch 2 |
| `hub_get_attachment` / `hub_save_attachment` | `messager/attachments.rs` | base64 of attachment blob, sync | open — batch 2 |
| `hub_purge_stale_memories` / `hub_age_out_short_term` / `hub_compact_short_term` | `messaging.rs` | bulk DB delete/update | open — batch 3 (measure first) |
| `hub_list_channel_messages` / `hub_poll_messages` / `hub_list_messages` | `messaging.rs` | DB read, grows with store; `poll` is on a timer | open — batch 3 (measure first) |

Async commands that still run blocking `std::fs` **directly in the async body**
(not on `spawn_blocking`) — lower severity (stalls an async worker, not the
webview thread) but worth tightening:
`hub_capture_{claude,codex,gemini,grok}_session` → the `harness/*.rs` capture
helpers do recursive `fs::read_dir` + `read_to_string`. `capture_commands.rs`
already wraps one path in `spawn_blocking`; confirm all four do.

## Frontend pending-state gaps

`HubPanel.run()` set only `status`/`error`, no pending state — **fixed this
PR** (`setStatus("Working…")` for the call duration; now paints because the
backing commands are off-thread). Still to audit for a pending state:
per-row buttons in `HubPanelView` (Compact ST, budget actions), the Settings
danger-zone actions (#132), avatar crop/apply in `AgentAvatar.tsx`.

## This PR (batch 1)

`hub_export_markdown` + `hub_export_markdown_git` → `async` + `spawn_blocking`,
`_blocking` inners kept for the policy test. `HubPanel.run()` shows
`Working…`. `cargo test`/`clippy`/`npm run build` green. **Not** reproduced
live — the original multi-second stall needs a large store / slow disk;
the change follows the pattern verified against `tauri-macros` codegen in
`726f28c`.

# Inbox/chat UX handoff — Chat/Codex

Date: 2026-08-12

## Scope implemented

Layered a Slack-like conversation surface onto `src/components/HubPanel.tsx`:

- agent/team conversation sidebar with pending-message counts;
- selected-conversation filtering and existing text search;
- three-second inbox refresh while the Inbox tab is active;
- subject field and Team recipient support in the composer;
- explicit “Mark unread as read” action backed by `hub_poll_messages`.

The existing Tauri IPC contract is unchanged. `npm run build` and
`cargo test -p hub` pass (11 tests).

## Ownership boundary

`HubPanel.tsx` already contained another agent’s staged inbox/search changes
when this slice was started. Those staged changes were preserved; this slice
is currently unstaged in the same file and must be reviewed/merged before
commit. Do not reset, checkout, or stage the whole file without coordinating
with the owner of the staged changes.

The repository also contains concurrent staged work in `src/App.tsx`,
`src-tauri/src/hub_cmds.rs`, `crates/hub/src/store.rs`, documentation, and
`src/components/panels/SlackChatPanel.tsx`; none of those files were modified
by this slice.

//! C14.3: opt-in Claude Code "Channel" bridge glue.
//!
//! This module is pure Hub-side logic: reading pending authenticated
//! events addressed to `claude`, recording Claude's reply back into the
//! Hub, and durably tracking permission-relay requests so nothing is ever
//! auto-approved. Nothing here speaks MCP or touches Claude Code's
//! internal `cc-socks` control socket — the actual MCP stdio server that
//! Claude Code spawns via the documented `claude/channel` capability lives
//! in the separate `claude-channel` binary crate, which links against
//! this module. This file also never mutates `bridge::claude`'s C12
//! capture-only delivery-safety path; that bridge continues to serve
//! sessions that have not opted into a Channel.
//!
//! **Authenticated sender gate:** rather than a bespoke crypto layer, the
//! gate reuses the Hub's existing trust boundary — only messages from an
//! *enrolled team member* are ever pushed into a live Claude session.
//! Both the bridge process and the Hub already trust the same local
//! SQLite store; the risk this gate defends against is an unenrolled or
//! stray identity string reaching a live session, not a network attacker.
//!
//! **Permission relay:** reuses the existing hash-chained `audit_events`
//! table (the same one Settings' audit stream is a typed filter over)
//! instead of a new table. A request starts `pending`; the bridge relays
//! `notifications/claude/channel/permission` only after a human explicitly
//! calls [`resolve_permission_request`] — there is no code path that
//! marks a request `approved` on its own.
//!
//! Split by concern, one file per module, each ≤500 LoC:
//! [`workspaces`] (setup/list/rename/delete the app-owned `.mcp.json`
//! registry), [`events`] (the disturb/quiet poll split), [`reply`] (routing
//! Claude's output back into the Hub), [`permissions`] (the never-auto-
//! approved relay lifecycle), and [`terminal`] (detecting and launching a
//! live Channel-connected session).

#[cfg(test)]
mod acceptance;
mod events;
mod permissions;
mod reply;
mod terminal;
mod workspaces;

pub const CLAUDE_AGENT_ID: &str = "claude";

pub use events::{poll_channel_events, poll_quiet_channel_events, ChannelEvent};
pub use permissions::{
    get_permission_request, record_permission_request, resolve_permission_request,
    PermissionVerdict,
};
pub use reply::record_channel_reply;
pub use terminal::{channel_bridge_pids, is_channel_session_live, launch_claude_channel_session};
pub use workspaces::{
    delete_channel_workspace, list_channel_workspaces, rename_channel_workspace, servers_dir,
    setup_claude_channel, workspace_server_name, ChannelWorkspace,
};

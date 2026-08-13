//! C14.3: opt-in Claude Code "Channel" MCP bridge.
//!
//! Claude Code spawns this binary as a stdio MCP server once the owner has
//! (1) run `--setup` for a workspace, which writes a `.mcp.json` entry via
//! [`hub::setup_claude_channel`], and (2) starts `claude --channels` in
//! that workspace so the documented, research-preview `claude/channel`
//! capability is active. Every event this process sends or receives
//! crosses the documented MCP `claude/channel`/`claude/channel/permission`
//! surface — it never touches Claude Code's undocumented internal
//! `cc-socks` control socket, and it never mutates
//! `hub::bridge::claude`'s C12 capture-only delivery path. A Claude
//! session that has not opted into a Channel keeps using that
//! capture-only path exactly as before.
//!
//! Protocol notes (from Anthropic's documented Channels research preview):
//! - `initialize` capability negotiation declares
//!   `capabilities.experimental["claude/channel"]` (push events in) and
//!   `["claude/channel/permission"]` (opt-in permission relay).
//! - Pushing an event into the session is an MCP *notification*
//!   (`notifications/claude/channel`, `{content, meta}`), not a response
//!   to any request — Claude Code does not poll this server, so a
//!   background thread here polls the Hub and pushes proactively.
//! - Claude replies through a normal MCP *tool* (`tools/call`); the tool
//!   itself is not special, only what the server's handler does with the
//!   call (here: route it back into the Hub) is bridge-specific.
//! - A permission dialog opening in the session arrives here as
//!   `notifications/claude/channel/permission_request`; the relayed
//!   verdict (`notifications/claude/channel/permission`) is sent only
//!   after a human resolves it via [`hub::resolve_permission_request`] —
//!   this process never decides on its own.
//!
//! Split by concern, one file per module under `src/main/`, each ≤500 LoC:
//! `cli` (the `--setup`/`--list`/`--rename`/`--delete` subcommands),
//! `protocol` (pure MCP payload shaping for `reply`/`check_inbox`), and
//! `server` (the actual stdio request dispatch and background poller).

#[path = "main/cli.rs"]
mod cli;
#[path = "main/protocol.rs"]
mod protocol;
#[path = "main/server.rs"]
mod server;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--setup") => cli::run_setup(&args[1..]),
        Some("--list") => cli::run_list(),
        Some("--rename") => cli::run_rename(&args[1..]),
        Some("--delete") => cli::run_delete(&args[1..]),
        _ => server::run_server(&args),
    }
}

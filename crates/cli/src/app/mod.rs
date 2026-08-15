//! `ca` CLI argument surface: the top-level `Cli`/[`Command`] shells and
//! the per-domain subcommand enums. The payload enums live in
//! [`commands`] (split out for the 500-LoC cap, #158); agent/harness/
//! journal payloads are their own sibling modules.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod agent;
mod commands;
mod harness;
mod journal;

pub(crate) use agent::AgentCommand;
pub(crate) use commands::{
    AuditCommand, BudgetCommand, InboxCommand, MemoryCommand, MsgCommand, TaskCommand, WakeCommand,
};
pub(crate) use harness::HarnessCommand;
pub(crate) use journal::JournalCommand;

#[derive(Parser)]
#[command(name = "ca", about = "Coding-Assistants shared hub CLI")]
pub(crate) struct Cli {
    /// Hub data directory (contains hub.db, journals/, markdown/, wake/).
    /// Defaults to $CA_HOME or ~/.coding-assistants.
    #[arg(long, env = "CA_HOME")]
    pub(crate) home: Option<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Create the hub data directory and database if they don't exist yet.
    Init,
    /// Shared memory operations.
    Memory {
        #[command(subcommand)]
        action: MemoryCommand,
    },
    /// Durable inbox/outbox messages between agents.
    Msg {
        #[command(subcommand)]
        action: MsgCommand,
    },
    /// Ephemeral wake requests (separate from durable memory/messages).
    Wake {
        #[command(subcommand)]
        action: WakeCommand,
    },
    /// Append to a private, per-agent journal (never shared).
    Journal {
        #[command(subcommand)]
        action: JournalCommand,
    },
    /// Manage known agent identities and A2A Agent Cards.
    Agent {
        #[command(subcommand)]
        action: AgentCommand,
    },
    /// Backward-compatible alias for `agent list`.
    Agents,
    /// Export episodic/semantic memory as git-friendly Markdown.
    ExportMarkdown {
        #[arg(long)]
        out: Option<PathBuf>,
        /// `git add` + `git commit` the export if `out` (or the default
        /// `markdown/` dir) is inside a Git work tree (M3). No-op, not an
        /// error, outside a repo or when nothing changed.
        #[arg(long)]
        commit: bool,
        #[arg(long, requires = "commit")]
        message: Option<String>,
    },
    /// Sequential multi-agent workflow tasks (C5).
    Task {
        #[command(subcommand)]
        action: TaskCommand,
    },
    /// Per-agent spend budgets and exhaustion handoffs (C6).
    Budget {
        #[command(subcommand)]
        action: BudgetCommand,
    },
    /// Observe filesystem changes and retain them for owner review.
    Audit {
        #[command(subcommand)]
        action: AuditCommand,
    },
    /// Run a long-lived adapter-facing inbox consumer.
    Inbox {
        #[command(subcommand)]
        action: InboxCommand,
    },
    /// Poll a harness's on-disk session transcript into the shared hub (C12).
    Harness {
        #[command(subcommand)]
        action: HarnessCommand,
    },
    /// Persist a cancellation/shutdown handoff so interrupted work is not lost (C6).
    Shutdown {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        objective: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        delegate_to: Option<String>,
    },
    /// Launch the keyboard-driven Ratatui terminal client (U7).
    Tui {
        /// Override the active workspace path for this invocation.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Override the active session ID for this invocation.
        #[arg(long)]
        session: Option<String>,
        /// Persist the specified invocation workspace as the default workspace setting.
        #[arg(long, requires = "workspace")]
        set_as_default_workspace_settings: bool,
        /// Persist the specified invocation session as the default session setting.
        #[arg(long, requires = "session")]
        set_as_default_session_settings: bool,
    },
    /// Read-only C13 owner-run inspector. Never writes Hub, settings, or `.agent/**`.
    Preflight {
        /// Repository to hash for Markdown-bus fallback files. Must be absolute.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Existing work-session id to describe (not created).
        #[arg(long)]
        session: Option<String>,
        /// Emit JSON instead of the #113 markdown block.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

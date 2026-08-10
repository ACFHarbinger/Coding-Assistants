//! `ca` -- the shared CLI helper from `docs/moon/roadmaps/communication.md` (C2).
//!
//! Lets any of the external agent tool-calling loops (Claude Code, Codex,
//! Gemini/Antigravity, Grok Build, ...) read/write the shared hub without
//! depending on the Tauri desktop process being open.

use ca_hub::{parse_scope, parse_tier, Hub};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ca", about = "Coding-Assistants shared hub CLI")]
struct Cli {
    /// Path to the hub SQLite database. Defaults to ~/.coding-assistants/hub.sqlite3.
    #[arg(long, env = "CA_HUB_DB")]
    db: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write a durable message to another agent (or broadcast if --to is omitted).
    WriteMessage {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        thread: Option<String>,
        body: String,
    },
    /// Read messages, optionally filtered by recipient/workspace.
    ReadMessages {
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long, default_value_t = false)]
        unread_only: bool,
    },
    /// Mark a message as read.
    MarkRead { id: String },
    /// Write a shared memory entry. Scope is "global" or a workspace identifier.
    WriteMemory {
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long, default_value = "short_term")]
        tier: String,
        #[arg(long)]
        agent: Option<String>,
        content: String,
    },
    /// Substring-search shared memory in a scope (global memories always included).
    SearchMemory {
        #[arg(long, default_value = "global")]
        scope: String,
        query: String,
    },
    /// Write a private, per-agent journal entry (never shared).
    JournalWrite {
        #[arg(long)]
        agent: String,
        content: String,
    },
    /// Read back only the calling agent's own journal.
    JournalRead {
        #[arg(long)]
        agent: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(Hub::default_path);
    let hub = Hub::open(db_path)?;

    match cli.command {
        Command::WriteMessage { from, to, workspace, thread, body } => {
            let msg = hub.write_message(&from, to.as_deref(), workspace.as_deref(), thread.as_deref(), &body)?;
            println!("{}", serde_json::to_string_pretty(&msg)?);
        }
        Command::ReadMessages { to, workspace, unread_only } => {
            let msgs = hub.read_messages(to.as_deref(), workspace.as_deref(), unread_only)?;
            println!("{}", serde_json::to_string_pretty(&msgs)?);
        }
        Command::MarkRead { id } => {
            hub.mark_read(&id)?;
            println!("ok");
        }
        Command::WriteMemory { scope, tier, agent, content } => {
            let entry = hub.write_memory(&parse_scope(&scope), parse_tier(&tier), agent.as_deref(), &content)?;
            println!("{}", serde_json::to_string_pretty(&entry)?);
        }
        Command::SearchMemory { scope, query } => {
            let results = hub.search_memory(&parse_scope(&scope), &query)?;
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        Command::JournalWrite { agent, content } => {
            let entry = hub.journal_write(&agent, &content)?;
            println!("{}", serde_json::to_string_pretty(&entry)?);
        }
        Command::JournalRead { agent } => {
            let entries = hub.journal_read(&agent)?;
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
    }
    Ok(())
}

//! `ca` -- the shared CLI helper from `docs/moon/roadmaps/communication.md` (C2).
//!
//! Lets any of the external agent tool-calling loops (Claude Code, Codex,
//! Gemini/Antigravity, Grok Build, ...) read/write the shared hub without
//! depending on the Tauri desktop process being open. Backed by
//! `ca_hub::HubStore`; command surface matches `crates/README.md`.

use ca_hub::{HubStore, MemoryScope, MemoryTier, MessageKind, MessageStatus, WakeStatus};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ca", about = "Coding-Assistants shared hub CLI")]
struct Cli {
    /// Hub data directory (contains hub.db, journals/, markdown/, wake/).
    /// Defaults to $CA_HOME or ~/.coding-assistants.
    #[arg(long, env = "CA_HOME")]
    home: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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
    /// List known agent identities.
    Agents,
    /// Export episodic/semantic memory as git-friendly Markdown.
    ExportMarkdown {
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum MemoryCommand {
    Write {
        #[arg(long, default_value = "short_term")]
        tier: String,
        #[arg(long, default_value = "global")]
        scope: String,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        body: String,
    },
    List {
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long, default_value_t = false)]
        include_stale: bool,
    },
    Search {
        query: String,
    },
    Stale {
        id: String,
        #[arg(long, default_value_t = false)]
        unstale: bool,
    },
    /// Promote a memory to a higher tier (short_term→episodic→semantic).
    Promote {
        id: String,
        #[arg(long, default_value = "episodic")]
        to: String,
    },
    /// Delete a memory row permanently.
    Delete {
        id: String,
    },
    /// Compact short-term: keep newest N, promote the rest to episodic.
    Compact {
        #[arg(long, default_value_t = 50)]
        keep: usize,
    },
    /// Permanently delete memories marked stale.
    PurgeStale,
    /// Soft-stale short-term rows older than N hours.
    AgeOut {
        #[arg(long, default_value_t = 72)]
        hours: i64,
    },
}

#[derive(Subcommand)]
enum MsgCommand {
    Send {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long, default_value = "message")]
        kind: String,
        #[arg(long)]
        subject: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        task: Option<String>,
        body: String,
    },
    Poll {
        #[arg(long)]
        to: String,
        #[arg(long, default_value_t = false)]
        no_ack: bool,
    },
    List {
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Mark a message done/acked/cancelled.
    Status {
        id: String,
        #[arg(long)]
        status: String,
    },
}

#[derive(Subcommand)]
enum WakeCommand {
    Request {
        #[arg(long)]
        target: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        message_id: Option<String>,
        #[arg(long, default_value_t = false)]
        human_gate: bool,
    },
    List {
        #[arg(long)]
        target: Option<String>,
        #[arg(long, default_value_t = false)]
        pending_only: bool,
    },
    /// Mark a wake delivered or cancelled.
    Resolve {
        id: String,
        #[arg(long, default_value = "delivered")]
        status: String,
    },
    /// Get or set standing wake/human-gate policy (C4).
    Policy {
        #[arg(long)]
        set_default_gate: Option<bool>,
        #[arg(long)]
        set_allow_auto: Option<bool>,
    },
}

#[derive(Subcommand)]
enum JournalCommand {
    Append {
        #[arg(long)]
        agent: String,
        entry: String,
    },
}

fn default_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".coding-assistants")
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let home = cli.home.unwrap_or_else(default_home);
    let store = HubStore::open(&home)?;

    match cli.command {
        Command::Init => {
            println!("initialized hub at {}", store.data_dir().display());
        }
        Command::Agents => {
            println!("{}", serde_json::to_string_pretty(&store.list_agents()?)?);
        }
        Command::ExportMarkdown { out } => {
            let path = store.export_markdown(out.as_deref())?;
            println!("exported to {}", path.display());
        }
        Command::Memory { action } => match action {
            MemoryCommand::Write {
                tier,
                scope,
                agent,
                workspace,
                title,
                tags,
                body,
            } => {
                let tier = MemoryTier::parse(&tier)?;
                let scope = MemoryScope::parse(&scope)?;
                let record = store.write_memory(
                    tier,
                    scope,
                    agent.as_deref(),
                    workspace.as_deref(),
                    title.as_deref(),
                    &body,
                    &tags,
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            MemoryCommand::List {
                scope,
                tier,
                workspace,
                include_stale,
            } => {
                let scope = scope.map(|s| MemoryScope::parse(&s)).transpose()?;
                let tier = tier.map(|t| MemoryTier::parse(&t)).transpose()?;
                let records =
                    store.list_memories(scope, tier, workspace.as_deref(), include_stale)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            MemoryCommand::Search { query } => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&store.search_memories(&query)?)?
                );
            }
            MemoryCommand::Stale { id, unstale } => {
                store.mark_memory_stale(&id, !unstale)?;
                println!("ok");
            }
            MemoryCommand::Promote { id, to } => {
                let to = MemoryTier::parse(&to)?;
                let record = store.promote_memory(&id, to)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            MemoryCommand::Delete { id } => {
                store.delete_memory(&id)?;
                println!("ok");
            }
            MemoryCommand::Compact { keep } => {
                let report = store.compact_short_term(keep)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            MemoryCommand::PurgeStale => {
                let n = store.purge_stale_memories()?;
                println!("{{\"purged\":{n}}}");
            }
            MemoryCommand::AgeOut { hours } => {
                let n = store.mark_short_term_stale_older_than(hours)?;
                println!("{{\"aged_out\":{n}}}");
            }
        },
        Command::Msg { action } => match action {
            MsgCommand::Send {
                from,
                to,
                kind,
                subject,
                workspace,
                task,
                body,
            } => {
                let kind = MessageKind::parse(&kind)?;
                let record = store.send_message(
                    &from,
                    &to,
                    kind,
                    &body,
                    subject.as_deref(),
                    workspace.as_deref(),
                    task.as_deref(),
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            MsgCommand::Poll { to, no_ack } => {
                let records = store.poll_messages(&to, !no_ack)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            MsgCommand::List { to, status } => {
                let status = status.map(|s| MessageStatus::parse(&s)).transpose()?;
                let records = store.list_messages(to.as_deref(), status)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            MsgCommand::Status { id, status } => {
                let status = MessageStatus::parse(&status)?;
                let record = store.set_message_status(&id, status)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
        },
        Command::Wake { action } => match action {
            WakeCommand::Request {
                target,
                reason,
                message_id,
                human_gate,
            } => {
                let record = store.request_wake(
                    &target,
                    reason.as_deref(),
                    message_id.as_deref(),
                    human_gate,
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            WakeCommand::List {
                target,
                pending_only,
            } => {
                let records = store.list_wakes(target.as_deref(), pending_only)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            WakeCommand::Resolve { id, status } => {
                let status = match status.as_str() {
                    "delivered" => WakeStatus::Delivered,
                    "cancelled" => WakeStatus::Cancelled,
                    "pending" => WakeStatus::Pending,
                    other => anyhow::bail!("unknown wake status: {other}"),
                };
                store.set_wake_status(&id, status)?;
                println!("ok");
            }
            WakeCommand::Policy {
                set_default_gate,
                set_allow_auto,
            } => {
                let mut policy = store.get_wake_policy()?;
                if let Some(v) = set_default_gate {
                    policy.default_requires_human_gate = v;
                }
                if let Some(v) = set_allow_auto {
                    policy.allow_auto_wake = v;
                }
                if set_default_gate.is_some() || set_allow_auto.is_some() {
                    store.set_wake_policy(&policy)?;
                }
                println!("{}", serde_json::to_string_pretty(&policy)?);
            }
        },
        Command::Journal { action } => match action {
            JournalCommand::Append { agent, entry } => {
                let path = store.append_private_journal(&agent, &entry)?;
                println!("appended to {}", path.display());
            }
        },
    }
    Ok(())
}

//! `ca` -- the shared CLI helper from `docs/moon/roadmaps/communication.md` (C2).
//!
//! Lets any of the external agent tool-calling loops (Claude Code, Codex,
//! Gemini/Antigravity, Grok Build, ...) read/write the shared hub without
//! depending on the Tauri desktop process being open. Backed by
//! `hub::HubStore`; command surface matches `crates/README.md`.

use clap::{ArgAction, Parser, Subcommand};
use hub::{
    inject_harness_with_store, HarnessInjectRequest, HubStore, MemoryScope, MemoryTier,
    MessageKind, MessageStatus, TaskStatus, WakeStatus, WorkflowStep,
};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sha2::{Digest, Sha256};
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
}

#[derive(Subcommand)]
enum AuditCommand {
    /// Watch a directory recursively until interrupted.
    Watch { root: PathBuf },
    /// List changes that have not yet been owner-approved or quarantined.
    Pending,
    /// List all recorded changes.
    List,
    /// Mark one event approved, or approve all currently pending events.
    Approve {
        id: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Mark an event as quarantined pending a later remediation workflow.
    Quarantine { id: String },
    /// Verify every hash-chain link.
    Verify,
}

#[derive(Subcommand)]
enum InboxCommand {
    /// Stream an agent's pending messages as JSONL until interrupted.
    Watch {
        #[arg(long)]
        agent: String,
        /// Poll interval in milliseconds.
        #[arg(long, default_value_t = 500)]
        interval_ms: u64,
        /// Consume wakes marked as requiring human approval. Use only when
        /// the adapter itself is the approved recipient boundary.
        #[arg(long, default_value_t = false)]
        accept_gated: bool,
        /// Optional long-lived adapter program. JSONL records are forwarded
        /// to its stdin in addition to this command's stdout.
        #[arg(long, value_name = "PROGRAM")]
        forward: Option<PathBuf>,
        /// Argument passed to --forward; may be repeated.
        #[arg(long = "forward-arg", action = ArgAction::Append)]
        forward_args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum BudgetCommand {
    /// Set (or reset) an agent's budget.
    Set {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        limit: f64,
    },
    /// Show an agent's current spend/limit/pause status.
    Status { agent: String },
    /// Record spend against an agent's budget.
    Spend {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        amount: f64,
    },
    /// Reserve units before an external provider call; rejects over-limit calls.
    Consume {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        amount: f64,
    },
    /// Pause an agent, write a Markdown handoff summary, and hand the task
    /// off to another agent (or "human" by default).
    Pause {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        objective: String,
        #[arg(long)]
        completed: String,
        #[arg(long)]
        missing: String,
        #[arg(long)]
        delegate_to: Option<String>,
    },
    /// Clear a budget pause so the agent can receive wakes again.
    Resume { agent: String },
}

#[derive(Subcommand)]
enum AgentCommand {
    /// List known agent identities.
    List,
    /// List agents with persisted Slack/Orchestrate team enrollment.
    Team,
    /// Enroll an existing agent on the team roster.
    Enroll {
        #[arg(long)]
        id: String,
    },
    /// Remove an agent from the team roster (still privately addressable).
    Unenroll {
        #[arg(long)]
        id: String,
    },
    /// Register an A2A Agent Card for discovery.
    RegisterCard {
        #[arg(long)]
        agent: String,
        /// Path to the agent.json card file
        #[arg(long)]
        path: PathBuf,
    },
}

#[derive(Subcommand)]
enum TaskCommand {
    /// Create a workflow. --steps is JSON array of
    /// {agent, instruction, role?, max_retries?, parallel_group?} objects.
    /// Consecutive steps sharing parallel_group form a bounded parallel stage.
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        steps: String,
        /// Max concurrent wakes inside a parallel stage (default 4).
        #[arg(long, default_value_t = 4)]
        max_parallel: u32,
        #[arg(long, default_value = "true")]
        require_approval: bool,
    },
    List {
        #[arg(long)]
        status: Option<String>,
    },
    Get {
        id: String,
    },
    /// Advance one **stage** (or complete after the last stage).
    /// Fails while a parallel stage still has open agents.
    Advance {
        id: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// Mark one agent finished in the current parallel stage (wakes queued agents).
    Complete {
        id: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// Re-dispatch the current stage (honours max_retries; may mark task failed).
    Retry {
        id: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    Cancel {
        id: String,
    },
}

#[derive(Subcommand)]
enum MemoryCommand {
    Write {
        /// short_term | episodic | semantic
        #[arg(long, default_value = "short_term")]
        tier: String,
        /// global | workspace
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
        /// global | workspace
        #[arg(long)]
        scope: Option<String>,
        /// short_term | episodic | semantic
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
        /// episodic | semantic
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
    /// C11: send a task- and/or wake-tagged message, enforcing the same
    /// rules as the Chat & Memory composer — task targets must already be
    /// present team/session members (rejected otherwise, no spawn); wake
    /// may enroll a new identity before delivery, subject to wake policy.
    Tag {
        #[arg(long)]
        from: String,
        /// Comma-separated recipient agent ids.
        #[arg(long, value_delimiter = ',')]
        to: Vec<String>,
        #[arg(long, default_value_t = false)]
        task: bool,
        #[arg(long, default_value_t = false)]
        wake: bool,
        #[arg(long)]
        subject: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        session: Option<String>,
        /// Inject each accepted target through its harness after the durable
        /// tagged-send policy accepts delivery. Requires an absolute
        /// --workspace; rejected targets are never dispatched.
        #[arg(long, requires = "workspace", default_value_t = false)]
        dispatch: bool,
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
    /// List one Slack-style channel (`channel:<name>` messages only).
    Channel {
        channel: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Resolve durable memory references embedded in a message body.
    Memories { message_id: String },
    /// Mark a message done/acked/cancelled.
    Status {
        id: String,
        #[arg(long)]
        status: String,
    },
    /// Edit a message (and every sibling copy of the same team/channel
    /// broadcast). Only Harbinger's own posts may be edited (CA-106/CA-109).
    Edit {
        #[arg(long)]
        id: String,
        #[arg(long)]
        from: String,
        body: String,
    },
    /// Delete (cancel) a message and every sibling copy of the same
    /// team/channel broadcast. Only Harbinger's own posts may be deleted.
    Delete {
        #[arg(long)]
        id: String,
        #[arg(long)]
        from: String,
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

#[derive(Subcommand)]
enum HarnessCommand {
    /// Read a harness's on-disk session transcript and record any new
    /// assistant-authored text into the shared hub (C12), the same way the
    /// desktop's periodic refresh does — but usable headless, so a C13 live
    /// acceptance run does not require the Tauri app to be open.
    Capture {
        /// grok | claude | chat (Codex) | gemini
        #[arg(long)]
        harness: String,
        /// Absolute path to the workspace the harness session ran in.
        #[arg(long)]
        workspace: PathBuf,
        /// The harness's own on-disk session/conversation id. Locates one
        /// specific transcript; omit to use the most recently modified one.
        #[arg(long)]
        disk_session: Option<String>,
        /// The Chat & Memory work-session uuid to scope this capture into
        /// (`channel:session:<id>:capture`). Omit to post to the team feed.
        #[arg(long)]
        hub_session: Option<String>,
    },
}

fn default_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".coding-assistants")
}

/// CA-106/CA-109: only Harbinger may edit/delete a chat message, mirroring
/// the desktop `require_human_authored` check in
/// `src-tauri/src/hub/commands.rs`.
/// Checked against both the caller-supplied `--from` and the message's
/// actual `from_agent`, since only the latter is authoritative.
fn require_human_authored(store: &HubStore, from: &str, message_id: &str) -> anyhow::Result<()> {
    if from != "human" {
        anyhow::bail!("only Harbinger (--from human) may edit or delete a chat message");
    }
    let message = store
        .get_message(message_id)?
        .ok_or_else(|| anyhow::anyhow!("message not found: {message_id}"))?;
    if message.from_agent != "human" {
        anyhow::bail!("only Harbinger may edit or delete a chat message");
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let home = cli.home.unwrap_or_else(default_home);
    let store = HubStore::open(&home)?;

    match cli.command {
        Command::Init => {
            println!("initialized hub at {}", store.data_dir().display());
        }
        Command::Agent { action } => match action {
            AgentCommand::List => {
                println!("{}", serde_json::to_string_pretty(&store.list_agents()?)?);
            }
            AgentCommand::Team => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&store.list_team_members()?)?
                );
            }
            AgentCommand::Enroll { id } => {
                let record = store.set_team_member(&id, true)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            AgentCommand::Unenroll { id } => {
                let record = store.set_team_member(&id, false)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            AgentCommand::RegisterCard { agent, path } => {
                let json = std::fs::read_to_string(&path)?;
                let card: hub::AgentCard = serde_json::from_str(&json)?;
                store.upsert_agent_card(&agent, &card)?;
                println!("registered card for {}", agent);
            }
        },
        Command::Agents => {
            println!("{}", serde_json::to_string_pretty(&store.list_agents()?)?);
        }
        Command::ExportMarkdown {
            out,
            commit,
            message,
        } => {
            if commit {
                let outcome = store.export_markdown_git(out.as_deref(), message.as_deref())?;
                println!(
                    "exported to {} ({})",
                    outcome.path.display(),
                    if outcome.committed {
                        "committed"
                    } else {
                        &outcome.detail
                    }
                );
            } else {
                let path = store.export_markdown(out.as_deref())?;
                println!("exported to {}", path.display());
            }
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
                if to == "team" {
                    let records = store.send_message_to_team(
                        &from,
                        kind,
                        &body,
                        subject.as_deref(),
                        workspace.as_deref(),
                        task.as_deref(),
                    )?;
                    println!("{}", serde_json::to_string_pretty(&records)?);
                    return Ok(());
                }
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
            MsgCommand::Tag {
                from,
                to,
                task,
                wake,
                subject,
                workspace,
                task_id,
                session,
                dispatch,
                body,
            } => {
                let dispatch_workspace = tagged_dispatch_workspace(dispatch, workspace.as_deref())?;
                let outcomes = store.send_tagged_message(
                    &from,
                    &to,
                    task,
                    wake,
                    &body,
                    subject.as_deref(),
                    workspace.as_deref(),
                    task_id.as_deref(),
                    session.as_deref(),
                )?;
                if let Some(workspace) = dispatch_workspace {
                    for outcome in outcomes.iter().filter(|outcome| outcome.accepted) {
                        let dispatch_result = inject_harness_with_store(
                            &store,
                            &HarnessInjectRequest {
                                harness: outcome.to_agent.clone(),
                                workspace: workspace.clone(),
                                session_id: session.clone(),
                                message_id: outcome.message_id.clone(),
                                body: body.clone(),
                                is_task: task,
                                is_wake: wake,
                            },
                        );
                        let dispatch_event = match dispatch_result {
                            Ok(result) => serde_json::json!({
                                "type": "harness_dispatch",
                                "target": outcome.to_agent,
                                "message_id": outcome.message_id,
                                "result": result,
                                "error": null,
                            }),
                            Err(error) => serde_json::json!({
                                "type": "harness_dispatch",
                                "target": outcome.to_agent,
                                "message_id": outcome.message_id,
                                "result": null,
                                "error": error.to_string(),
                            }),
                        };
                        eprintln!("{dispatch_event}");
                    }
                }
                println!("{}", serde_json::to_string_pretty(&outcomes)?);
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
            MsgCommand::Channel { channel, limit } => {
                let records = store.list_channel_messages(&channel, limit)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            MsgCommand::Memories { message_id } => {
                let memories = store.list_message_memories(&message_id)?;
                println!("{}", serde_json::to_string_pretty(&memories)?);
            }
            MsgCommand::Status { id, status } => {
                let status = MessageStatus::parse(&status)?;
                let record = store.set_message_status(&id, status)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            MsgCommand::Edit { id, from, body } => {
                require_human_authored(&store, &from, &id)?;
                let records = store.update_broadcast(&id, &body)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            MsgCommand::Delete { id, from } => {
                require_human_authored(&store, &from, &id)?;
                let count = store.delete_broadcast(&id)?;
                println!("{{\"deleted\": {count}}}");
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
        Command::Task { action } => match action {
            TaskCommand::Create {
                title,
                workspace,
                steps,
                max_parallel,
                require_approval,
            } => {
                let steps: Vec<WorkflowStep> = serde_json::from_str(&steps)
                    .map_err(|e| anyhow::anyhow!("--steps JSON: {e}"))?;
                let record = store.create_task_with_parallel(
                    &title,
                    workspace.as_deref(),
                    &steps,
                    max_parallel,
                    require_approval,
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::List { status } => {
                let status = status.map(|s| TaskStatus::parse(&s)).transpose()?;
                let records = store.list_tasks(status)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            TaskCommand::Get { id } => {
                let record = store
                    .get_task(&id)?
                    .ok_or_else(|| anyhow::anyhow!("task not found: {id}"))?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::Advance { id, from, note } => {
                let record = store.advance_task(&id, from.as_deref(), note.as_deref())?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::Complete { id, agent, note } => {
                let record = store.complete_parallel_member(&id, &agent, note.as_deref())?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::Retry { id, from, note } => {
                let record = store.retry_task(&id, from.as_deref(), note.as_deref())?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::Cancel { id } => {
                let record = store.cancel_task(&id)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
        },
        Command::Budget { action } => match action {
            BudgetCommand::Set { agent, limit } => {
                let status = store.set_agent_budget(&agent, limit)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            BudgetCommand::Status { agent } => {
                let status = store
                    .get_budget(&agent)?
                    .ok_or_else(|| anyhow::anyhow!("no budget set for {agent}"))?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            BudgetCommand::Spend { agent, amount } => {
                let status = store.record_budget_usage(&agent, amount)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            BudgetCommand::Consume { agent, amount } => {
                let status = store.try_consume_budget(&agent, amount)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            BudgetCommand::Pause {
                agent,
                task,
                objective,
                completed,
                missing,
                delegate_to,
            } => {
                let outcome = store.pause_for_budget(
                    &agent,
                    task.as_deref(),
                    &objective,
                    &completed,
                    &missing,
                    delegate_to.as_deref(),
                )?;
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            }
            BudgetCommand::Resume { agent } => {
                let status = store.resume_agent(&agent)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
        },
        Command::Audit { action } => match action {
            AuditCommand::Watch { root } => {
                let root = std::fs::canonicalize(&root)?;
                if !root.is_dir() {
                    anyhow::bail!("audit root is not a directory: {}", root.display());
                }
                let (sender, receiver) = std::sync::mpsc::channel();
                let mut watcher = RecommendedWatcher::new(sender, Config::default())?;
                watcher.watch(&root, RecursiveMode::Recursive)?;
                println!("watching {} (Ctrl-C to stop)", root.display());
                while let Ok(result) = receiver.recv() {
                    match result {
                        Ok(event) => {
                            let operation = audit_operation(&event.kind);
                            for path in event.paths {
                                let relative = path.strip_prefix(&root).unwrap_or(&path);
                                let process = audit_process_context();
                                let hash = audit_file_hash(&path);
                                let record = store.record_audit_event(
                                    &root,
                                    relative,
                                    operation,
                                    &process,
                                    hash.as_deref(),
                                )?;
                                println!("{} {} {}", record.id, record.operation, record.path);
                            }
                        }
                        Err(error) => eprintln!("audit watcher error: {error}"),
                    }
                }
            }
            AuditCommand::Pending => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&store.list_audit_events(true)?)?
                );
            }
            AuditCommand::List => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&store.list_audit_events(false)?)?
                );
            }
            AuditCommand::Approve { id, all } => {
                if all {
                    for event in store.list_audit_events(true)? {
                        store.set_audit_status(&event.id, "approved")?;
                    }
                } else {
                    let id = id.ok_or_else(|| anyhow::anyhow!("provide an event id or --all"))?;
                    store.set_audit_status(&id, "approved")?;
                }
                println!("approved");
            }
            AuditCommand::Quarantine { id } => {
                store.set_audit_status(&id, "quarantined")?;
                println!("quarantined");
            }
            AuditCommand::Verify => {
                let count = store.verify_audit_chain()?;
                println!("verified {count} audit events");
            }
        },
        Command::Inbox { action } => match action {
            InboxCommand::Watch {
                agent,
                interval_ms,
                accept_gated,
                forward,
                forward_args,
            } => {
                if interval_ms == 0 {
                    anyhow::bail!("--interval-ms must be greater than zero");
                }
                use std::io::Write;
                let mut forwarder = if let Some(program) = forward {
                    let mut child = std::process::Command::new(program)
                        .args(forward_args)
                        .stdin(std::process::Stdio::piped())
                        .spawn()?;
                    child.stdin.take()
                } else {
                    if !forward_args.is_empty() {
                        anyhow::bail!("--forward-arg requires --forward");
                    }
                    None
                };
                let ready = serde_json::json!({
                    "type": "ready",
                    "agent": agent,
                    "interval_ms": interval_ms,
                    "accept_gated": accept_gated
                })
                .to_string();
                println!("{ready}");
                if let Some(stdin) = forwarder.as_mut() {
                    writeln!(stdin, "{ready}")?;
                    stdin.flush()?;
                }
                std::io::stdout().flush()?;
                loop {
                    let pending_wakes = store.list_wakes(Some(&agent), true)?;
                    let messages =
                        store.list_messages(Some(&agent), Some(MessageStatus::Pending))?;
                    let mut delivered_ids = Vec::new();
                    for message in messages {
                        let gated = pending_wakes.iter().any(|wake| {
                            wake.message_id.as_deref() == Some(message.id.as_str())
                                && wake.requires_human_gate
                        });
                        if gated && !accept_gated {
                            // The durable message remains available, but the
                            // adapter must not cross the human gate silently.
                            continue;
                        }
                        let message =
                            store.set_message_status(&message.id, MessageStatus::Acked)?;
                        delivered_ids.push(message.id.clone());
                        let line = serde_json::json!({
                            "type": "message",
                            "agent": agent,
                            "message": message
                        })
                        .to_string();
                        println!("{line}");
                        if let Some(stdin) = forwarder.as_mut() {
                            writeln!(stdin, "{line}")?;
                            stdin.flush()?;
                        }
                        std::io::stdout().flush()?;
                    }
                    for wake in pending_wakes {
                        if wake.requires_human_gate && !accept_gated {
                            continue;
                        }
                        if let Some(message_id) = &wake.message_id {
                            if !delivered_ids.iter().any(|id| id == message_id) {
                                continue;
                            }
                        }
                        store.set_wake_status(&wake.id, WakeStatus::Delivered)?;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(interval_ms));
                }
            }
        },
        Command::Harness { action } => match action {
            HarnessCommand::Capture {
                harness,
                workspace,
                disk_session,
                hub_session,
            } => {
                let outcome = capture_harness_session(
                    &store,
                    &harness,
                    &workspace,
                    disk_session.as_deref(),
                    hub_session.as_deref(),
                )?;
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            }
        },
        Command::Shutdown {
            agent,
            task,
            objective,
            reason,
            delegate_to,
        } => {
            let outcome = store.record_shutdown(
                &agent,
                task.as_deref(),
                &objective,
                &reason,
                delegate_to.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&outcome)?);
        }
    }
    Ok(())
}

fn tagged_dispatch_workspace(
    dispatch: bool,
    workspace: Option<&str>,
) -> anyhow::Result<Option<PathBuf>> {
    if !dispatch {
        return Ok(None);
    }
    let workspace =
        PathBuf::from(workspace.ok_or_else(|| anyhow::anyhow!("--dispatch requires --workspace"))?);
    if !workspace.is_absolute() {
        anyhow::bail!("--dispatch requires an absolute --workspace path");
    }
    Ok(Some(workspace))
}

fn audit_operation(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Create(_) => "created",
        EventKind::Remove(_) => "removed",
        EventKind::Modify(_) => "modified",
        EventKind::Access(_) => "accessed",
        EventKind::Other => "other",
        EventKind::Any => "other",
    }
}

fn audit_file_hash(path: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let digest = Sha256::digest(bytes);
    Some(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn audit_process_context() -> String {
    let exe = std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string());
    serde_json::json!({
        "pid": std::process::id(),
        "exe": exe,
        "cmdline": std::env::args().collect::<Vec<_>>(),
        "attribution": "observer_process; originating_writer_not_available_in_user_space_notify"
    })
    .to_string()
}

// --- C12: `ca harness capture` ---------------------------------------------
//
// The desktop app's periodic refresh polls each harness's real capture
// adapter, which lives in `src-tauri/src/harness_*.rs` (a different crate
// this CLI does not and should not depend on). To make C13's "hub-native run
// without the desktop app" requirement possible, this re-implements the same
// four on-disk transcript formats independently here, against the shared
// `hub::HubStore::record_harness_capture` dedup path — so a headless `ca
// harness capture` run and the desktop's poll converge on the same durable
// state even though they don't share code across the crate boundary.

#[derive(serde::Serialize)]
struct HarnessCaptureOutcome {
    harness: String,
    transcript_found: bool,
    scanned: usize,
    captured: Vec<hub::MessageRecord>,
}

fn home_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
}

fn recent_json_lines(path: &std::path::Path, tail_lines: usize) -> Vec<serde_json::Value> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = raw.lines().collect();
    let start = lines.len().saturating_sub(tail_lines);
    lines[start..]
        .iter()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .collect()
}

/// Grok TUI: `~/.grok/sessions/<percent-encoded-abs-workspace>/<session>/chat_history.jsonl`.
fn grok_encode_workspace(workspace: &std::path::Path) -> String {
    workspace
        .to_string_lossy()
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn grok_transcript_path(
    sessions_root: &std::path::Path,
    workspace: &std::path::Path,
    disk_session: Option<&str>,
) -> Option<PathBuf> {
    let root = sessions_root.join(grok_encode_workspace(workspace));
    if let Some(session) = disk_session {
        let candidate = root.join(session).join("chat_history.jsonl");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|dir| {
            let history = dir.join("chat_history.jsonl");
            let modified = std::fs::metadata(&history).ok()?.modified().ok()?;
            Some((modified, history))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn grok_assistant_texts(path: &std::path::Path) -> Vec<String> {
    recent_json_lines(path, 200)
        .into_iter()
        .filter(|value| value.get("type").and_then(|t| t.as_str()) == Some("assistant"))
        .filter_map(|value| {
            value
                .get("content")
                .and_then(|c| c.as_str())
                .map(str::to_string)
        })
        .filter(|text| !text.trim().is_empty())
        .collect()
}

/// Claude Code: `~/.claude/projects/<workspace-with-slashes-as-dashes>/<session>.jsonl`.
fn claude_encode_workspace(workspace: &std::path::Path) -> String {
    workspace.to_string_lossy().replace('/', "-")
}

fn claude_transcript_path(
    projects_dir: &std::path::Path,
    workspace: &std::path::Path,
    disk_session: Option<&str>,
) -> Option<PathBuf> {
    let dir = projects_dir.join(claude_encode_workspace(workspace));
    if let Some(session) = disk_session {
        let candidate = dir.join(format!("{session}.jsonl"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|path| {
            let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn claude_assistant_texts(path: &std::path::Path) -> Vec<String> {
    recent_json_lines(path, 500)
        .into_iter()
        .filter(|value| value.get("type").and_then(|t| t.as_str()) == Some("assistant"))
        .filter_map(|value| {
            let content = value.get("message")?.get("content")?.as_array()?;
            let text: String = content
                .iter()
                .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n\n");
            (!text.trim().is_empty()).then_some(text)
        })
        .collect()
}

/// Codex: `~/.codex/sessions/YYYY/MM/DD/*.jsonl`, matched by `session_meta.cwd`.
fn codex_transcript_paths(sessions_root: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Ok(years) = std::fs::read_dir(sessions_root) else {
        return paths;
    };
    for year in years.filter_map(Result::ok).map(|entry| entry.path()) {
        let Ok(months) = std::fs::read_dir(&year) else {
            continue;
        };
        for month in months.filter_map(Result::ok).map(|entry| entry.path()) {
            let Ok(days) = std::fs::read_dir(&month) else {
                continue;
            };
            for day in days.filter_map(Result::ok).map(|entry| entry.path()) {
                let Ok(files) = std::fs::read_dir(&day) else {
                    continue;
                };
                paths.extend(
                    files
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl")),
                );
            }
        }
    }
    paths
}

fn codex_transcript_metadata(path: &std::path::Path) -> Option<(String, String)> {
    let raw = std::fs::read_to_string(path).ok()?;
    raw.lines().take(16).find_map(|line| {
        let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
        if value.get("type").and_then(|t| t.as_str()) != Some("session_meta") {
            return None;
        }
        let payload = value.get("payload")?;
        Some((
            payload.get("cwd")?.as_str()?.to_string(),
            payload.get("session_id")?.as_str()?.to_string(),
        ))
    })
}

fn codex_transcript_path(
    sessions_root: &std::path::Path,
    workspace: &std::path::Path,
    disk_session: Option<&str>,
) -> Option<PathBuf> {
    let workspace = workspace.to_string_lossy();
    codex_transcript_paths(sessions_root)
        .into_iter()
        .filter_map(|path| {
            let (cwd, session_id) = codex_transcript_metadata(&path)?;
            if cwd != workspace || disk_session.is_some_and(|id| id != session_id) {
                return None;
            }
            Some((std::fs::metadata(&path).ok()?.modified().ok()?, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn codex_assistant_texts(path: &std::path::Path) -> Vec<String> {
    recent_json_lines(path, 500)
        .into_iter()
        .filter(|value| value.get("type").and_then(|t| t.as_str()) == Some("response_item"))
        .filter_map(|value| {
            let payload = value.get("payload")?;
            if payload.get("type").and_then(|t| t.as_str()) != Some("message")
                || payload.get("role").and_then(|r| r.as_str()) != Some("assistant")
            {
                return None;
            }
            let content = payload.get("content")?.as_array()?;
            let text: String = content
                .iter()
                .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("output_text"))
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n\n");
            (!text.trim().is_empty()).then_some(text)
        })
        .collect()
}

/// Antigravity CLI: `~/.gemini/antigravity-cli/brain/<conv-id>/.system_generated/logs/transcript.jsonl`.
fn gemini_transcript_path(
    brain_dir: &std::path::Path,
    disk_session: Option<&str>,
) -> Option<PathBuf> {
    if let Some(conv_id) = disk_session {
        let candidate = brain_dir
            .join(conv_id)
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    std::fs::read_dir(brain_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter_map(|dir| {
            let log_file = dir
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");
            let modified = std::fs::metadata(&log_file).ok()?.modified().ok()?;
            Some((modified, log_file))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn gemini_assistant_texts(path: &std::path::Path) -> Vec<String> {
    recent_json_lines(path, 500)
        .into_iter()
        .filter(|value| {
            value.get("source").and_then(|s| s.as_str()) == Some("MODEL")
                || value.get("type").and_then(|t| t.as_str()) == Some("PLANNER_RESPONSE")
        })
        .filter_map(|value| {
            value
                .get("content")
                .and_then(|c| c.as_str())
                .map(str::trim)
                .map(str::to_string)
        })
        .filter(|text| !text.is_empty() && !text.starts_with("```json"))
        .collect()
}

fn capture_harness_session(
    store: &HubStore,
    harness: &str,
    workspace: &std::path::Path,
    disk_session: Option<&str>,
    hub_session: Option<&str>,
) -> anyhow::Result<HarnessCaptureOutcome> {
    let (agent_id, texts, transcript_found) = match harness {
        "grok" => {
            let workspace = workspace
                .canonicalize()
                .unwrap_or_else(|_| workspace.to_path_buf());
            match grok_transcript_path(
                &home_dir().join(".grok").join("sessions"),
                &workspace,
                disk_session,
            ) {
                Some(path) => ("grok", grok_assistant_texts(&path), true),
                None => ("grok", Vec::new(), false),
            }
        }
        "claude" => {
            match claude_transcript_path(
                &home_dir().join(".claude").join("projects"),
                workspace,
                disk_session,
            ) {
                Some(path) => ("claude", claude_assistant_texts(&path), true),
                None => ("claude", Vec::new(), false),
            }
        }
        "chat" | "codex" => {
            let workspace = workspace
                .canonicalize()
                .unwrap_or_else(|_| workspace.to_path_buf());
            match codex_transcript_path(
                &home_dir().join(".codex").join("sessions"),
                &workspace,
                disk_session,
            ) {
                Some(path) => ("chat", codex_assistant_texts(&path), true),
                None => ("chat", Vec::new(), false),
            }
        }
        "gemini" | "agy" => {
            let brain_dir = home_dir()
                .join(".gemini")
                .join("antigravity-cli")
                .join("brain");
            match gemini_transcript_path(&brain_dir, disk_session) {
                Some(path) => ("gemini", gemini_assistant_texts(&path), true),
                None => ("gemini", Vec::new(), false),
            }
        }
        other => anyhow::bail!("unknown harness: {other} (expected grok, claude, chat, or gemini)"),
    };

    let mut captured = Vec::new();
    if transcript_found {
        for text in &texts {
            if let Some(record) = store.record_harness_capture(
                harness,
                agent_id,
                hub_session,
                text,
                Some(&workspace.to_string_lossy()),
            )? {
                captured.push(record);
            }
        }
    }
    Ok(HarnessCaptureOutcome {
        harness: harness.to_string(),
        transcript_found,
        scanned: texts.len(),
        captured,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn tagged_dispatch_is_opt_in_and_requires_an_absolute_workspace() {
        assert_eq!(tagged_dispatch_workspace(false, None).unwrap(), None);
        assert!(tagged_dispatch_workspace(true, None).is_err());
        assert!(tagged_dispatch_workspace(true, Some("relative/workspace")).is_err());
        assert_eq!(
            tagged_dispatch_workspace(true, Some("/tmp/c12-cli-dispatch"))
                .unwrap()
                .as_deref(),
            Some(Path::new("/tmp/c12-cli-dispatch"))
        );
    }

    #[test]
    fn unknown_harness_id_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let result =
            capture_harness_session(&store, "not-a-harness", Path::new("/tmp/x"), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn grok_capture_extracts_assistant_text_and_dedups() {
        let sessions_root = tempfile::tempdir().unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(store_dir.path()).unwrap();
        let workspace = Path::new("/tmp/cli-c12-grok");
        let dir = sessions_root
            .path()
            .join(grok_encode_workspace(workspace))
            .join("sess-1");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("chat_history.jsonl"),
            "{\"type\":\"reasoning\",\"content\":\"thinking\"}\n{\"type\":\"assistant\",\"content\":\"cli capture works\"}\n",
        )
        .unwrap();
        let path = grok_transcript_path(sessions_root.path(), workspace, None).unwrap();
        let texts = grok_assistant_texts(&path);
        assert_eq!(texts, vec!["cli capture works".to_string()]);

        // Dedup happens in the store, not in the path/parsing helpers above —
        // exercise that boundary directly here.
        let first = store
            .record_harness_capture("grok", "grok", Some("hub-1"), &texts[0], None)
            .unwrap();
        assert!(first.is_some());
        let second = store
            .record_harness_capture("grok", "grok", Some("hub-1"), &texts[0], None)
            .unwrap();
        assert!(second.is_none(), "repeat capture must dedup");
    }

    #[test]
    fn claude_capture_skips_thinking_and_tool_use_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = Path::new("/fake/workspace");
        let session_dir = dir.path().join(claude_encode_workspace(workspace));
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(
            session_dir.join("s1.jsonl"),
            "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"thinking\",\"thinking\":\"x\"}]}}\n\
             {\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"cli claude capture\"}]}}\n",
        )
        .unwrap();
        let path = claude_transcript_path(dir.path(), workspace, None).unwrap();
        assert_eq!(
            claude_assistant_texts(&path),
            vec!["cli claude capture".to_string()]
        );
    }

    #[test]
    fn codex_capture_matches_by_workspace_and_disk_session() {
        let root = tempfile::tempdir().unwrap();
        let day_dir = root.path().join("2026").join("08").join("13");
        std::fs::create_dir_all(&day_dir).unwrap();
        let workspace = Path::new("/tmp/cli-c12-codex");
        std::fs::write(
            day_dir.join("rollout.jsonl"),
            format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}\",\"session_id\":\"disk-a\"}}}}\n\
                 {{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{{\"type\":\"output_text\",\"text\":\"cli codex capture\"}}]}}}}\n",
                workspace.display()
            ),
        )
        .unwrap();
        let path = codex_transcript_path(root.path(), workspace, Some("disk-a")).unwrap();
        assert_eq!(
            codex_assistant_texts(&path),
            vec!["cli codex capture".to_string()]
        );
        assert!(codex_transcript_path(root.path(), workspace, Some("disk-b")).is_none());
    }

    #[test]
    fn gemini_capture_extracts_model_responses_only() {
        let brain_dir = tempfile::tempdir().unwrap();
        let logs_dir = brain_dir
            .path()
            .join("conv-1")
            .join(".system_generated")
            .join("logs");
        std::fs::create_dir_all(&logs_dir).unwrap();
        std::fs::write(
            logs_dir.join("transcript.jsonl"),
            "{\"source\":\"USER_EXPLICIT\",\"content\":\"hi\"}\n\
             {\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"content\":\"cli gemini capture\"}\n",
        )
        .unwrap();
        let path = gemini_transcript_path(brain_dir.path(), None).unwrap();
        assert_eq!(
            gemini_assistant_texts(&path),
            vec!["cli gemini capture".to_string()]
        );
    }
}

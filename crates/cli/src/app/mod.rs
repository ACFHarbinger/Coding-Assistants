use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

mod harness;
mod journal;

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

#[derive(Subcommand)]
pub(crate) enum AuditCommand {
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
pub(crate) enum InboxCommand {
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
pub(crate) enum BudgetCommand {
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
pub(crate) enum AgentCommand {
    /// List known agent identities.
    List,
    /// List agents with persisted Messager/Orchestrate team enrollment.
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
pub(crate) enum TaskCommand {
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
pub(crate) enum MemoryCommand {
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
pub(crate) enum MsgCommand {
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
    /// List one Messager-style channel (`channel:<name>` messages only).
    Channel {
        channel: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Resolve durable memory references embedded in a message body.
    Memories { message_id: String },
    /// Mark that `agent` has read `scope` (a channel id, work session id, or
    /// `dm-<agent>` pairing) as of now, or an explicit `--at` RFC 3339
    /// timestamp. Never regresses an existing, more recent marker.
    Read {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        scope: String,
        #[arg(long)]
        at: Option<String>,
    },
    /// List every team member's read marker for `scope`.
    Readers { scope: String },
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
pub(crate) enum WakeCommand {
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

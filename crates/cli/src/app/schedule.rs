//! `ca schedule` payload (Scheduled Goals S4 / #354).

use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub(crate) enum ScheduleCommand {
    /// List scheduled tasks, newest first. --status filters by state.
    List {
        /// scheduled | active | paused | completed | cancelled | failed
        #[arg(long)]
        status: Option<String>,
    },
    /// Create a scheduled task (one-shot by default; --repeat-interval-secs
    /// keeps it re-prompting until gates pass or a stop condition trips).
    // Boxed so the small variants stay small (clippy large_enum_variant).
    Create(Box<ScheduleCreateArgs>),
    /// Show one scheduled task as JSON.
    Get { id: String },
    /// Cancel a scheduled task (terminal; already-terminal tasks are a no-op).
    Cancel { id: String },
    /// Run the dispatch/response/gate state machine for every due task and
    /// print a per-task outcome summary. This is the headless entry point
    /// OS triggers (cron/systemd) invoke. Enforces the in-hub credit
    /// allowance only — live provider quota is app-side and out of reach
    /// here (see roadmap S5c).
    Tick {
        /// Override "now" (RFC3339) for diagnostics/dry-runs. Defaults to
        /// the real current time.
        #[arg(long)]
        now: Option<String>,
    },
}

#[derive(Args)]
pub(crate) struct ScheduleCreateArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub instruction: String,
    /// Agent id to target, or "team" for the standing roster.
    #[arg(long)]
    pub target: String,
    /// Absolute workspace path the schedule runs against.
    #[arg(long)]
    pub workspace: String,
    /// First-fire time, RFC3339.
    #[arg(long)]
    pub fire_at: String,
    /// Seconds between re-prompts while gates are unmet. Omit for a
    /// one-shot task (fires once, no gate loop).
    #[arg(long)]
    pub repeat_interval_secs: Option<i64>,
    /// Wait for a response this long before counting the attempt as
    /// failed (against max-attempts) and re-prompting.
    #[arg(long, default_value_t = 3600)]
    pub response_timeout_secs: i64,
    /// JSON array of GateCriterion objects, same shape the hub stores,
    /// e.g. [{"kind":"max_lines_per_file","max_lines":100,"globs":["**/*.rs"]}].
    #[arg(long, default_value = "[]")]
    pub gate_criteria: String,
    /// How criteria combine: all (default) | any.
    #[arg(long, default_value = "all")]
    pub gate_combinator: String,
    /// In-hub credit ceiling for this task. Enforced by headless tick.
    #[arg(long)]
    pub credit_allowance: Option<f64>,
    /// Also pause when a live provider-quota window is exhausted. Only
    /// the in-app poller (S6) can honor this; headless `ca schedule
    /// tick` never sees live quota (S5c bridges that gap).
    #[arg(long, default_value_t = false)]
    pub stop_on_quota_exhausted: bool,
    /// Mandatory stop condition: attempts cap, >= 1.
    #[arg(long, default_value_t = 1)]
    pub max_attempts: u32,
    /// Optional wall-clock cutoff, RFC3339.
    #[arg(long)]
    pub deadline: Option<String>,
    #[arg(long, default_value = "human")]
    pub created_by: String,
}

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum JournalCommand {
    Append {
        #[arg(long)]
        agent: String,
        entry: String,
        /// Write to the shared coordination log (`bus_entries`) instead of
        /// the private per-agent journal file. Private files stay legacy.
        #[arg(long, default_value_t = false)]
        shared: bool,
        /// Topic for a shared entry (default `log`).
        #[arg(long)]
        topic: Option<String>,
        /// Issue reference for a shared entry (e.g. `#330`).
        #[arg(long)]
        issue: Option<String>,
        /// Task id for a shared entry.
        #[arg(long)]
        task: Option<String>,
    },
    List {
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        topic: Option<String>,
        /// RFC3339 lower bound on `created_at`.
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        issue: Option<String>,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        limit: Option<i64>,
    },
}

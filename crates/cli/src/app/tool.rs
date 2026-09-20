//! `ca tool` payload (P5 / #333).

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum ToolCommand {
    /// Run a PATH program with explicit argv (never a shell).
    #[command(allow_hyphen_values = true)]
    Run {
        /// Working directory. Defaults to the current directory.
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Workspace root used by the sandbox. Defaults to --cwd.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Program name on PATH (Standard/Strict) or absolute path (Permissive).
        program: String,
        /// Arguments passed as data, one argv slot each.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Execute a pending `os_tool.proposed` audit after human approval.
    Approve { id: String },
    /// List pending OS-tool proposals.
    Pending,
}

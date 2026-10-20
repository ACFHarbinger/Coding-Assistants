//! `ca guardrail` payload (Guardrails primitive P16 / #360).

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum GuardrailCommand {
    /// List rules (hub-wide plus --workspace's when given), oldest first.
    List {
        /// Absolute workspace path; omit for every rule.
        #[arg(long)]
        workspace: Option<String>,
    },
    /// Add a rule. --spec is one JSON object:
    /// {"kind":"deny_phrases","phrases":[...],"case_insensitive":true} |
    /// {"kind":"require_phrases","phrases":[...]} |
    /// {"kind":"max_chars","max":N} | {"kind":"no_secrets"}.
    Add {
        #[arg(long)]
        name: String,
        /// input (screens a prompt before dispatch) | output (screens a
        /// reply before a stage advances / a gate is evaluated).
        #[arg(long)]
        stage: String,
        /// blocking (default) | parallel — see `hub::guardrails`.
        #[arg(long, default_value = "blocking")]
        mode: String,
        /// Absolute workspace this rule is limited to; omit for hub-wide.
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        spec: String,
    },
    /// Enable or disable a rule without deleting it.
    Enable {
        id: String,
        #[arg(long, default_value = "true")]
        on: bool,
    },
    /// Delete a rule.
    Remove { id: String },
    /// Dry-run: screen text (or stdin with -) against the effective rules
    /// for a stage/workspace. Writes no audit row.
    Check {
        #[arg(long)]
        stage: String,
        #[arg(long)]
        workspace: Option<String>,
        /// Text to screen; "-" reads stdin.
        text: String,
    },
}

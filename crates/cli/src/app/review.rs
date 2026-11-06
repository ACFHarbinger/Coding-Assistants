//! `ca review` subcommand payloads (Team Scorecard T7 / #347).

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum ReviewCommand {
    /// Record one classified review finding on the hash-chained audit stream.
    /// Findings are the evidence layer behind the scorecard's reviewer-side
    /// signature (fix-find rate, owner-override reversals).
    Finding {
        /// Reviewer agent id (explicit identity — never inferred).
        #[arg(long)]
        reviewer: String,
        /// Hub task id the finding belongs to (optional but feeds fix-find rate).
        #[arg(long)]
        task: Option<String>,
        /// blocking | major | minor | note
        #[arg(long)]
        severity: String,
        /// implementation_defect | changed_requirement | clarification | reviewer_preference
        #[arg(long)]
        origin: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// Owner correction of a finding's classification. Correction, not
    /// deletion: the original row stays on the chain and this row points at
    /// it, which is exactly the reversal signal.
    Correct {
        /// Audit id of the `review.finding` row being corrected.
        #[arg(long)]
        finding_id: String,
        #[arg(long)]
        reason: String,
    },
}

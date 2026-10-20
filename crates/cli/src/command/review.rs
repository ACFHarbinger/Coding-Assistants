//! `ca review` subcommand dispatch (Team Scorecard T7 / #347).

use crate::app::ReviewCommand;
use hub::{HubStore, ReviewFinding};

pub(super) fn run(store: &HubStore, action: ReviewCommand) -> anyhow::Result<()> {
    match action {
        ReviewCommand::Finding {
            reviewer,
            task,
            severity,
            origin,
            note,
        } => {
            let finding = ReviewFinding {
                reviewer,
                task_id: task,
                severity,
                origin,
                note,
            };
            let event = store.record_review_finding(&finding)?;
            println!("{}", serde_json::to_string_pretty(&event)?);
        }
        ReviewCommand::Correct { finding_id, reason } => {
            // The CLI runs as the local owner; corrections are owner-only
            // and the store enforces that authority.
            let event =
                store.record_review_finding_corrected("human", &finding_id, "human", &reason)?;
            println!("{}", serde_json::to_string_pretty(&event)?);
        }
    }
    Ok(())
}

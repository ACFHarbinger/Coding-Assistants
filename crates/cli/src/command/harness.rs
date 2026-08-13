use crate::app::HarnessCommand;
use crate::harness::capture_harness_session;
use hub::HubStore;

pub(super) fn run(store: &HubStore, action: HarnessCommand) -> anyhow::Result<()> {
    match action {
        HarnessCommand::Capture {
            harness,
            workspace,
            disk_session,
            hub_session,
        } => {
            let outcome = capture_harness_session(
                store,
                &harness,
                &workspace,
                disk_session.as_deref(),
                hub_session.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&outcome)?);
        }
    }
    Ok(())
}

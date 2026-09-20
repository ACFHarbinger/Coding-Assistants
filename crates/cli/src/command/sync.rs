//! `ca sync` runner (S4 / #94).

use anyhow::Context;
use hub::sync::{self, SyncSession};
use hub::HubStore;

use crate::app::SyncCommand;

pub(crate) fn run(store: &HubStore, action: SyncCommand) -> anyhow::Result<()> {
    let home = store.data_dir();
    let schema = store.hub_schema_version().context("hub schema_version")?;
    match action {
        SyncCommand::Preview => {
            let plan = sync::build_plan(home, schema, "preview")?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        SyncCommand::Up => print_run(sync::run_locked(home, schema, "up")?)?,
        SyncCommand::Down => print_run(sync::run_locked(home, schema, "down")?)?,
        SyncCommand::Sync => print_run(sync::run_locked(home, schema, "sync")?)?,
        SyncCommand::Cancel => {
            sync::release(home)?;
            println!("{{\"cancelled\":true}}");
        }
    }
    Ok(())
}

fn print_run(session: SyncSession) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(&session)?);
    Ok(())
}

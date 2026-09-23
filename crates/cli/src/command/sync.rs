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
        SyncCommand::Conflicts => {
            let items = sync::list_conflicts(home)?;
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
        SyncCommand::Resolve { slug, choice } => {
            let choice = sync::ConflictChoice::parse(&choice)?;
            let decision = sync::apply_choice(store, &slug, choice)?;
            println!("{}", serde_json::to_string_pretty(&decision)?);
        }
        SyncCommand::Tombstones => {
            let items = sync::list_tombstones(home)?;
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
        SyncCommand::Expired => {
            let items = sync::expired_cleanup_candidates(home, chrono::Utc::now())?;
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
        SyncCommand::PurgeExpired { confirm } => {
            if !confirm {
                anyhow::bail!("purge-expired requires --confirm");
            }
            let report = sync::purge_expired(store, &[], true)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        SyncCommand::Devices => {
            let list = sync::list_trust(home)?;
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        SyncCommand::Register { id } => {
            let parsed = hub::sync::DeviceId::parse(&id)?;
            let entry = sync::register(home, parsed)?;
            println!("{}", serde_json::to_string_pretty(&entry)?);
        }
        SyncCommand::Revoke { id } => {
            let parsed = hub::sync::DeviceId::parse(&id)?;
            let entry = sync::revoke(home, parsed)?;
            println!("{}", serde_json::to_string_pretty(&entry)?);
        }
        SyncCommand::Trust { id } => {
            let parsed = hub::sync::DeviceId::parse(&id)?;
            let entry = sync::retrust(home, parsed)?;
            println!("{}", serde_json::to_string_pretty(&entry)?);
        }
        SyncCommand::Retry => print_run(sync::run_locked(home, schema, "retry")?)?,
        SyncCommand::History => {
            let rows = sync::list_history(home)?;
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
        SyncCommand::Diagnostics => {
            let diag = sync::export_diagnostics(home)?;
            println!("{}", serde_json::to_string_pretty(&diag)?);
        }
        SyncCommand::Limits => {
            let limits = sync::ensure_limits(home)?;
            println!("{}", serde_json::to_string_pretty(&limits)?);
        }
    }
    Ok(())
}

fn print_run(session: SyncSession) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(&session)?);
    Ok(())
}

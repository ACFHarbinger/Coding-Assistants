//! Thin Tauri wrap around `hub::stop_managed_harness`.

use crate::commands::commands::store::open_store;
use crate::harness::blocking::run_blocking;
use hub::StopManagedOutcome;
use std::path::Path;

/// Kill the process that actually backs liveness for this harness in
/// `workspace`. Does not relaunch. Claude's stored terminal-emulator pid
/// is never treated as the session. SIGTERM/SIGKILL settle sleeps belong
/// off the IPC thread (#163).
#[tauri::command]
pub async fn hub_stop_managed_harness(
    harness: String,
    workspace: String,
) -> Result<StopManagedOutcome, String> {
    run_blocking("hub_stop_managed_harness", move || {
        hub::stop_managed_harness(&open_store()?, &harness, Path::new(&workspace))
    })
    .await
}

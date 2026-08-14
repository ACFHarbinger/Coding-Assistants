//! Thin Tauri wrap around `hub::stop_managed_harness`.

use crate::commands::commands::store::open_store;
use hub::StopManagedOutcome;
use std::path::Path;

/// Kill the process that actually backs liveness for this harness in
/// `workspace`. Does not relaunch. Claude's stored terminal-emulator pid
/// is never treated as the session.
#[tauri::command]
pub fn hub_stop_managed_harness(
    harness: String,
    workspace: String,
) -> Result<StopManagedOutcome, String> {
    hub::stop_managed_harness(&open_store()?, &harness, Path::new(&workspace))
}

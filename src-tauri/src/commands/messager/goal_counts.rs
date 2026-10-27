//! Accepted Goal and Wake send counts for the Shared Hub Tasks dashboard.
use super::store::open_store;
use hub::TaggedSendCounts;

#[tauri::command]
pub fn hub_tagged_send_counts() -> Result<TaggedSendCounts, String> {
    open_store()?
        .tagged_send_counts()
        .map_err(|error| error.to_string())
}

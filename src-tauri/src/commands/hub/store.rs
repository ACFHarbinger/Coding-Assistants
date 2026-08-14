//! Hub store initialization and agent-card commands.
use hub::HubStore;
use std::path::PathBuf;
fn default_home() -> PathBuf {
    hub::default_hub_home()
}

pub fn open_store() -> Result<HubStore, String> {
    HubStore::open(default_home()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_init() -> Result<String, String> {
    let store = open_store()?;
    Ok(store.data_dir().display().to_string())
}

/// The Shared Hub dashboard's "Data dir:" line calls this on mount; it was
/// never actually registered as a command (only `HubPanel.tsx` referenced
/// the name), so that line always showed "Command hub_get_data_dir not
/// found" instead of the real path. Same value `hub_init` returns.
#[tauri::command]
pub fn hub_get_data_dir() -> Result<String, String> {
    let store = open_store()?;
    Ok(store.data_dir().display().to_string())
}

#[tauri::command]
pub fn hub_list_agents() -> Result<Vec<hub::AgentRecord>, String> {
    open_store()?.list_agents().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_upsert_agent_card(agent: String, card: hub::AgentCard) -> Result<(), String> {
    open_store()?
        .upsert_agent_card(&agent, &card)
        .map_err(|e| e.to_string())
}

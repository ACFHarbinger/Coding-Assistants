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

#[tauri::command]
pub async fn hub_set_agent_display_name(
    app: tauri::AppHandle,
    agent_id: String,
    display_name: String,
) -> Result<hub::AgentRecord, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let updated = hub_set_agent_display_name_blocking(&agent_id, &display_name)?;
        use tauri::Emitter;
        let _ = app.emit("hub:agents-changed", ());
        Ok(updated)
    })
    .await
    .map_err(|e| format!("hub_set_agent_display_name worker panic: {e}"))?
}

pub fn hub_set_agent_display_name_blocking(
    agent_id: &str,
    display_name: &str,
) -> Result<hub::AgentRecord, String> {
    open_store()?
        .set_agent_display_name(agent_id, display_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hub_set_agent_role(
    app: tauri::AppHandle,
    agent_id: String,
    role: Option<String>,
) -> Result<hub::AgentRecord, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let updated = hub_set_agent_role_blocking(&agent_id, role.as_deref())?;
        use tauri::Emitter;
        let _ = app.emit("hub:agents-changed", ());
        Ok(updated)
    })
    .await
    .map_err(|e| format!("hub_set_agent_role worker panic: {e}"))?
}

pub fn hub_set_agent_role_blocking(
    agent_id: &str,
    role: Option<&str>,
) -> Result<hub::AgentRecord, String> {
    open_store()?
        .set_agent_role(agent_id, role)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_agent_display_name_blocking_fails_on_nonexistent() {
        let res = hub_set_agent_display_name_blocking("nonexistent_xyz", "New Name");
        assert!(res.is_err());
    }

    #[test]
    fn set_agent_role_blocking_fails_on_nonexistent() {
        let res = hub_set_agent_role_blocking("nonexistent_xyz", Some("lead"));
        assert!(res.is_err());
    }
}

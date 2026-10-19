//! Scorecard commands for the Shared Hub Team Scorecard (Roadmap T3 / #343).

use crate::commands::commands::store::open_store;
use hub::{ScorecardView, SettingsStore};

#[tauri::command]
pub fn hub_get_scorecard(caller_agent_id: Option<String>) -> Result<ScorecardView, String> {
    let store = open_store()?;
    let settings_store = SettingsStore::open(hub::default_hub_home());
    let visibility = settings_store.snapshot().orchestration.scorecard_visibility;
    store
        .get_scorecard(caller_agent_id.as_deref(), visibility)
        .map_err(|e| e.to_string())
}

//! Scorecard commands for the Shared Hub Team Scorecard (Roadmap T3 / #343).

use crate::commands::commands::store::open_store;
use hub::{ScorecardView, SettingsStore};

/// Retrieves the team scorecard projection subject to the active visibility policy.
///
/// # Trust Boundary & Security Note
///
/// `caller_agent_id` identifies the agent requesting the scorecard (e.g. `Some("gemini")`
/// or `None` / `Some("human")` for the human workspace owner).
///
/// In the desktop application (Tauri IPC), invocations originate directly from the local
/// webview running trusted frontend code with the human operator in the loop. Passing
/// `None` or `Some("human")` grants access to the `OwnerOnly` view.
///
/// When exposing scorecard queries over remote or multi-tenant transports (e.g., TCP server,
/// peer-to-peer sync, or unauthenticated agent-to-agent RPCs), `caller_agent_id` MUST NOT
/// be taken from unauthenticated client input. Remote transports must authenticate the caller
/// (e.g. via pairing tokens or mutual TLS) before asserting `caller_agent_id`, to prevent
/// unauthorized agents or external callers from impersonating the workspace owner or other agents.
#[tauri::command]
pub fn hub_get_scorecard(caller_agent_id: Option<String>) -> Result<ScorecardView, String> {
    let store = open_store()?;
    let settings_store = SettingsStore::open(hub::default_hub_home());
    let visibility = settings_store.snapshot().orchestration.scorecard_visibility;
    store
        .get_scorecard(caller_agent_id.as_deref(), visibility)
        .map_err(|e| e.to_string())
}

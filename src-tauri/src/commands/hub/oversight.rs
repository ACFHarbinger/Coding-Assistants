//! Owner-path oversight commands (#342 / T2).
//!
//! The authority setter takes no actor argument, so a lead cannot point
//! this command at itself. Grants still name `granted_by`; the store
//! accepts a standing lead only after the owner has enabled delegation.

use super::store::open_store;
use hub::OversightExceptionRequest;

#[tauri::command]
pub fn hub_set_oversight_role(
    agent_id: String,
    role: Option<String>,
) -> Result<hub::AgentRecord, String> {
    open_store()?
        .set_oversight_role("human", &agent_id, role.as_deref())
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn hub_set_oversight_exception_authority(delegated_lead: bool) -> Result<(), String> {
    open_store()?
        .set_oversight_exception_authority("human", delegated_lead)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn hub_grant_oversight_exception(
    granted_by: String,
    request: OversightExceptionRequest,
) -> Result<hub::AuditEvent, String> {
    open_store()?
        .grant_oversight_exception(&granted_by, &request)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn hub_revoke_oversight_exception(
    actor: String,
    grant_id: String,
    reason: String,
) -> Result<hub::AuditEvent, String> {
    open_store()?
        .revoke_oversight_exception(&actor, &grant_id, &reason)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn hub_graduate_trial(
    actor: String,
    agent_id: String,
    to_role: String,
    reason: String,
) -> Result<hub::AgentRecord, String> {
    open_store()?
        .graduate_trial(&actor, &agent_id, &to_role, &reason)
        .map_err(|err| err.to_string())
}

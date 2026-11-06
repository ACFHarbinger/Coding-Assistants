//! Role/permission commands: the typed boundary the desktop Orchestrate UI
//! (and Shared Hub/Settings panels) use to define roles, assign them to
//! team members, configure per-provider defaults, and resolve pending
//! human-approval gates. Mirrors `hub::store::roles` 1:1 — no logic lives
//! here beyond argument shaping and error-string mapping.

use super::store::open_store;
use hub::{EffectiveAgentPermissions, PendingGateApproval, Role, RoleProviderDefault};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertRoleArgs {
    pub id: String,
    pub display_name: String,
    pub daily_ungated_quota: Option<i64>,
    pub max_broadcast_recipients: Option<i64>,
    pub can_archive_messages: bool,
    pub can_update_agent_roles: bool,
    pub can_allocate_tasks: bool,
    pub responsibilities: Vec<String>,
}

#[tauri::command]
pub fn hub_upsert_role(args: UpsertRoleArgs) -> Result<Role, String> {
    open_store()?
        .upsert_role(
            &args.id,
            &args.display_name,
            args.daily_ungated_quota,
            args.max_broadcast_recipients,
            args.can_archive_messages,
            args.can_update_agent_roles,
            args.can_allocate_tasks,
            &args.responsibilities,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_get_role(id: String) -> Result<Option<Role>, String> {
    open_store()?.get_role(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_roles() -> Result<Vec<Role>, String> {
    open_store()?.list_roles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_delete_role(id: String) -> Result<(), String> {
    open_store()?.delete_role(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_assign_agent_role(agent_id: String, role_id: String) -> Result<(), String> {
    open_store()?
        .assign_agent_role(&agent_id, &role_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_unassign_agent_role(agent_id: String, role_id: String) -> Result<(), String> {
    open_store()?
        .unassign_agent_role(&agent_id, &role_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_agent_roles(agent_id: String) -> Result<Vec<Role>, String> {
    open_store()?
        .list_agent_roles(&agent_id)
        .map_err(|e| e.to_string())
}

/// What `agent_id` may actually do right now — the union across every
/// assigned role (or its resolved provider default), for the Orchestrate
/// role cards and any pre-send UI check to render directly.
#[tauri::command]
pub fn hub_effective_agent_permissions(
    agent_id: String,
    workspace: Option<String>,
) -> Result<EffectiveAgentPermissions, String> {
    open_store()?
        .effective_agent_permissions(&agent_id, workspace.as_deref())
        .map_err(|e| e.to_string())
}

/// How many ungated task/wake sends `agent_id` has already used up today —
/// pairs with the role's `daily_ungated_quota` for a "12 / 20 today"-style
/// display.
#[tauri::command]
pub fn hub_gate_quota_used_today(agent_id: String) -> Result<i64, String> {
    open_store()?
        .gate_quota_used_today(&agent_id)
        .map_err(|e| e.to_string())
}

/// Sets the default role a `provider` resolves to when it has no explicit
/// role assignment. `workspace` omitted sets the *global* default; a
/// workspace-specific default always wins over it for that same provider.
#[tauri::command]
pub fn hub_set_role_provider_default(
    provider: String,
    workspace: Option<String>,
    role_id: String,
) -> Result<(), String> {
    open_store()?
        .set_role_provider_default(&provider, workspace.as_deref(), &role_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_role_provider_defaults() -> Result<Vec<RoleProviderDefault>, String> {
    open_store()?
        .list_role_provider_defaults()
        .map_err(|e| e.to_string())
}

/// Task/wake sends currently waiting on a human decision because they
/// exceeded their sender's role quota or broadcast-recipient limit.
/// `status` filters to `"pending"`, `"approved"`, or `"rejected"`; omitted
/// returns every gate approval ever recorded, most recent first.
#[tauri::command]
pub fn hub_list_pending_gate_approvals(
    status: Option<String>,
) -> Result<Vec<PendingGateApproval>, String> {
    open_store()?
        .list_pending_gate_approvals(status.as_deref())
        .map_err(|e| e.to_string())
}

/// Approving actually delivers the original send (via the normal,
/// unchanged `send_tagged_message`); rejecting never sends anything and
/// instead notifies the original sender why, automatically.
#[tauri::command]
pub fn hub_resolve_gate_approval(id: String, approve: bool) -> Result<PendingGateApproval, String> {
    open_store()?
        .resolve_gate_approval(&id, approve)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::commands::tests::CA_HOME_ENV_LOCK;

    fn with_ca_home<T>(prefix: &str, run: impl FnOnce() -> T) -> T {
        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "tauri-roles-{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::env::set_var("CA_HOME", &dir);
        let result = run();
        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
        result
    }

    #[test]
    fn role_crud_and_assignment_round_trip_through_the_tauri_boundary() {
        with_ca_home("crud", || {
            let role = hub_upsert_role(UpsertRoleArgs {
                id: "reviewer".into(),
                display_name: "Reviewer".into(),
                daily_ungated_quota: Some(10),
                max_broadcast_recipients: Some(5),
                can_archive_messages: true,
                can_update_agent_roles: false,
                can_allocate_tasks: false,
                responsibilities: vec!["reviewer".into()],
            })
            .expect("upsert");
            assert_eq!(role.display_name, "Reviewer");

            hub_assign_agent_role("gemini".into(), "reviewer".into()).expect("assign");
            let assigned = hub_list_agent_roles("gemini".into()).expect("list");
            assert_eq!(assigned.len(), 1);
            assert_eq!(assigned[0].id, "reviewer");

            let effective =
                hub_effective_agent_permissions("gemini".into(), None).expect("effective");
            assert_eq!(effective.daily_ungated_quota, Some(10));
            assert!(effective.can_archive_messages);

            hub_unassign_agent_role("gemini".into(), "reviewer".into()).expect("unassign");
            assert!(hub_delete_role("reviewer".into()).is_ok());
            assert!(hub_get_role("reviewer".into()).unwrap().is_none());
        });
    }

    #[test]
    fn provider_defaults_round_trip_and_starter_roles_are_present() {
        with_ca_home("defaults", || {
            let defaults = hub_list_role_provider_defaults().expect("list defaults");
            assert!(defaults
                .iter()
                .any(|d| d.provider == "grok" && d.workspace_path.is_empty()));

            let lead = hub_get_role("lead-orchestrator".into())
                .expect("get")
                .expect("starter role seeded");
            assert_eq!(lead.display_name, "Lead Orchestrator");

            hub_set_role_provider_default(
                "grok".into(),
                Some("/abs/repo".into()),
                "lead-orchestrator".into(),
            )
            .expect("set workspace default");
            let defaults = hub_list_role_provider_defaults().expect("list again");
            assert!(defaults
                .iter()
                .any(|d| d.provider == "grok" && d.workspace_path == "/abs/repo"));
        });
    }

    #[test]
    fn gate_approval_list_and_resolve_round_trip() {
        with_ca_home("gate", || {
            hub_upsert_role(UpsertRoleArgs {
                id: "capped".into(),
                display_name: "Capped".into(),
                daily_ungated_quota: Some(0),
                max_broadcast_recipients: Some(10),
                can_archive_messages: false,
                can_update_agent_roles: false,
                can_allocate_tasks: false,
                responsibilities: vec![],
            })
            .unwrap();
            hub_assign_agent_role("grok".into(), "capped".into()).unwrap();

            let store = open_store().unwrap();
            store.set_team_member("claude", true).unwrap();
            store
                .send_tagged_message_gated(
                    "grok",
                    &["claude".to_string()],
                    false,
                    true,
                    "needs approval",
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap();

            let pending = hub_list_pending_gate_approvals(Some("pending".into())).unwrap();
            assert_eq!(pending.len(), 1);
            assert_eq!(hub_gate_quota_used_today("grok".into()).unwrap(), 0);

            let resolved = hub_resolve_gate_approval(pending[0].id.clone(), true).unwrap();
            assert_eq!(resolved.status, "approved");
        });
    }
}

//! Typed, redacted settings IPC and scope resolution (Settings S2 / #128).
//!
//! Builds on the S1 `hub::SettingsStore` (#127). Commands here never send a
//! filesystem path to the frontend — only effective values, field-status
//! pills, and load-status diagnostics without their underlying path. Every
//! mutation is recorded on the dedicated settings audit stream, which is a
//! typed filter over the same Hub audit chain other commands already read.
use hub::{
    EffectiveHarnessSettings, EffectiveOrchestrationPolicy, EffectiveSettings, HarnessSettings,
    LoadStatus, ProfileSnapshot, ProviderProfile, SandboxStrictness, SettingsField, SettingsStore,
};

fn open_settings_store() -> SettingsStore {
    SettingsStore::open(hub::default_hub_home())
}

fn settings_field_name(field: SettingsField) -> &'static str {
    match field {
        SettingsField::BackupRetention => "storage.backup_retention",
        SettingsField::DefaultWorkspace => "general.default_workspace",
        SettingsField::DefaultSession => "general.default_session",
        SettingsField::ConfirmNewEnrollment => "orchestration.confirm_new_enrollment",
        SettingsField::ConfirmBroadcast => "orchestration.confirm_broadcast",
        SettingsField::AutoEnrollmentAllowed => "orchestration.auto_enrollment_allowed",
        SettingsField::SandboxStrictness => "orchestration.sandbox_strictness",
        SettingsField::RetentionDays => "orchestration.retention_days",
        SettingsField::ExportEnabled => "orchestration.export_enabled",
        SettingsField::LinkSuggestionMode => "orchestration.link_suggestion_mode",
    }
}

fn record_settings_audit(field: &str, scope: &str, action: &str) -> Result<(), String> {
    super::store::open_store()?
        .record_settings_audit_event(field, scope, action)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Partial update: `None` fields are left untouched. One field today;
/// later slices add more `Option<T>` fields without changing this shape.
#[derive(Debug, serde::Deserialize)]
pub struct SettingsPatch {
    pub backup_retention: Option<u32>,
}

/// Mirrors `hub::LoadStatus` without its file path, so the frontend can
/// show a malformed/interrupted-write diagnostic without ever learning a
/// configuration path.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SettingsLoadStatusDto {
    Missing,
    Loaded,
    Invalid { reason: String },
    Unreadable { reason: String },
}

impl From<&LoadStatus> for SettingsLoadStatusDto {
    fn from(status: &LoadStatus) -> Self {
        match status {
            LoadStatus::Missing => Self::Missing,
            LoadStatus::Loaded => Self::Loaded,
            LoadStatus::Invalid { reason } => Self::Invalid {
                reason: reason.clone(),
            },
            LoadStatus::Unreadable { reason } => Self::Unreadable {
                reason: reason.clone(),
            },
        }
    }
}

#[tauri::command]
pub fn settings_get_effective(workspace: Option<String>) -> Result<EffectiveSettings, String> {
    Ok(open_settings_store().effective(workspace.as_deref()))
}

#[tauri::command]
pub fn settings_get_load_status() -> Result<SettingsLoadStatusDto, String> {
    Ok(SettingsLoadStatusDto::from(
        &open_settings_store().load().status,
    ))
}

/// `workspace: None` updates the global default; `Some(path)` sets a
/// workspace-local override. `path` is kept exactly as given (never
/// symlink-resolved), so distinct paths to the same repository stay
/// distinct override identities.
#[tauri::command]
pub fn settings_update(
    workspace: Option<String>,
    patch: SettingsPatch,
) -> Result<EffectiveSettings, String> {
    let mut store = open_settings_store();
    if let Some(retention) = patch.backup_retention {
        match workspace.as_deref() {
            None => store
                .set_backup_retention(retention)
                .map_err(|e| e.to_string())?,
            Some(ws) => store
                .set_workspace_backup_retention(ws, retention)
                .map_err(|e| e.to_string())?,
        }
        store.save().map_err(|e| e.to_string())?;
        let scope = workspace.as_deref().unwrap_or("global");
        record_settings_audit("storage.backup_retention", scope, "update")?;
    }
    Ok(store.effective(workspace.as_deref()))
}

/// Reset one field of a workspace override back to inherited. There is no
/// global reset: the global value *is* the default.
#[tauri::command]
pub fn settings_reset_field(
    workspace: String,
    field: SettingsField,
) -> Result<EffectiveSettings, String> {
    let mut store = open_settings_store();
    store
        .reset_workspace_field(&workspace, field)
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(settings_field_name(field), &workspace, "reset")?;
    Ok(store.effective(Some(&workspace)))
}

/// `default_workspace` is global-only (there is no per-workspace override of
/// "which workspace opens by default"). `workspace: None` clears it.
#[tauri::command]
pub fn settings_set_default_workspace(
    workspace: Option<String>,
) -> Result<EffectiveSettings, String> {
    let mut store = open_settings_store();
    store
        .set_default_workspace(workspace.as_deref())
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit("general.default_workspace", "global", "update")?;
    Ok(store.effective(None))
}

/// `workspace: None` sets the global default session; `Some(path)` sets that
/// workspace's override. `session: None` clears the value at that scope.
#[tauri::command]
pub fn settings_set_default_session(
    workspace: Option<String>,
    session: Option<String>,
) -> Result<EffectiveSettings, String> {
    let mut store = open_settings_store();
    match workspace.as_deref() {
        None => store
            .set_default_session(session.as_deref())
            .map_err(|e| e.to_string())?,
        Some(ws) => store
            .set_workspace_default_session(ws, session.as_deref())
            .map_err(|e| e.to_string())?,
    }
    store.save().map_err(|e| e.to_string())?;
    let scope = workspace.as_deref().unwrap_or("global");
    record_settings_audit("general.default_session", scope, "update")?;
    Ok(store.effective(workspace.as_deref()))
}

#[tauri::command]
pub fn settings_list_audit_events() -> Result<Vec<hub::AuditEvent>, String> {
    super::store::open_store()?
        .list_settings_audit_events()
        .map_err(|e| e.to_string())
}

/// Redacted profile list. Never includes a credential or filesystem path.
#[tauri::command]
pub fn settings_list_profiles() -> Result<Vec<ProfileSnapshot>, String> {
    Ok(open_settings_store().list_profiles())
}

/// Create or replace a global named profile. `secret` is a reference only
/// (keychain id, env-var name, or existing-login). A raw credential is
/// rejected by the store validator.
#[tauri::command]
pub fn settings_upsert_profile(profile: ProviderProfile) -> Result<Vec<ProfileSnapshot>, String> {
    let mut store = open_settings_store();
    store
        .upsert_profile(profile.clone())
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit("profile", "global", &format!("upsert:{}", profile.name))?;
    Ok(store.list_profiles())
}

#[tauri::command]
pub fn settings_rename_profile(from: String, to: String) -> Result<Vec<ProfileSnapshot>, String> {
    let mut store = open_settings_store();
    store
        .rename_profile(&from, &to)
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit("profile", "global", &format!("rename:{from}->{to}"))?;
    Ok(store.list_profiles())
}

/// Removes profile configuration only. Does not delete a keychain secret.
#[tauri::command]
pub fn settings_remove_profile(name: String) -> Result<Vec<ProfileSnapshot>, String> {
    let mut store = open_settings_store();
    store.remove_profile(&name).map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit("profile", "global", &format!("remove:{name}"))?;
    Ok(store.list_profiles())
}

/// Workspace selects a default profile *name* for one harness. Profile
/// fields are not copied.
#[tauri::command]
pub fn settings_set_workspace_default_profile(
    workspace: String,
    harness: String,
    profile: String,
) -> Result<EffectiveSettings, String> {
    let mut store = open_settings_store();
    store
        .set_workspace_default_profile(&workspace, &harness, &profile)
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(
        &format!("workspace.default_profile.{harness}"),
        &workspace,
        "update",
    )?;
    Ok(store.effective(Some(&workspace)))
}

#[tauri::command]
pub fn settings_reset_workspace_default_profile(
    workspace: String,
    harness: String,
) -> Result<EffectiveSettings, String> {
    let mut store = open_settings_store();
    store
        .reset_workspace_default_profile(&workspace, &harness)
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(
        &format!("workspace.default_profile.{harness}"),
        &workspace,
        "reset",
    )?;
    Ok(store.effective(Some(&workspace)))
}

/// Global harness process settings, plus the workspace's selected default
/// profile when `workspace` is given.
#[tauri::command]
pub fn settings_list_harnesses(
    workspace: Option<String>,
) -> Result<Vec<EffectiveHarnessSettings>, String> {
    Ok(open_settings_store()
        .effective(workspace.as_deref())
        .harnesses)
}

#[tauri::command]
pub fn settings_update_harness(settings: HarnessSettings) -> Result<HarnessSettings, String> {
    let mut store = open_settings_store();
    // This older full-record endpoint backs the capture/injection toggles.
    // Model and effort now have dedicated update endpoints; callers that do
    // not know about those newer optional fields must not erase them.
    let current = store
        .harness_settings(&settings.harness)
        .map_err(|e| e.to_string())?;
    let settings = HarnessSettings {
        default_model: settings.default_model.or(current.default_model),
        default_effort: settings.default_effort.or(current.default_effort),
        ..settings
    };
    store
        .set_harness_settings(settings.clone())
        .map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    record_settings_audit(&format!("harness.{}", settings.harness), "global", "update")?;
    store
        .harness_settings(&settings.harness)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// Orchestration and storage policy (Settings S5 / #131)
// ---------------------------------------------------------------------

/// Orchestration policy partial update. `retention_days` is intentionally
/// excluded — see `settings_set_retention_days`, which needs a three-state
/// (untouched / set / cleared) contract this patch can't express.
#[derive(Debug, serde::Deserialize)]
pub struct OrchestrationPatch {
    pub confirm_new_enrollment: Option<bool>,
    pub confirm_broadcast: Option<bool>,
    pub auto_enrollment_allowed: Option<bool>,
    pub sandbox_strictness: Option<SandboxStrictness>,
    pub export_enabled: Option<bool>,
}

/// `workspace: None` updates the global default; `Some(path)` sets a
/// workspace-local override. Each changed field gets its own audit row.
#[tauri::command]
pub fn settings_update_orchestration(
    workspace: Option<String>,
    patch: OrchestrationPatch,
) -> Result<EffectiveSettings, String> {
    let mut store = open_settings_store();
    let mut changed_fields: Vec<&'static str> = Vec::new();

    if let Some(v) = patch.confirm_new_enrollment {
        match workspace.as_deref() {
            None => store
                .set_confirm_new_enrollment(v)
                .map_err(|e| e.to_string())?,
            Some(ws) => store
                .set_workspace_confirm_new_enrollment(ws, v)
                .map_err(|e| e.to_string())?,
        }
        changed_fields.push("orchestration.confirm_new_enrollment");
    }
    if let Some(v) = patch.confirm_broadcast {
        match workspace.as_deref() {
            None => store.set_confirm_broadcast(v).map_err(|e| e.to_string())?,
            Some(ws) => store
                .set_workspace_confirm_broadcast(ws, v)
                .map_err(|e| e.to_string())?,
        }
        changed_fields.push("orchestration.confirm_broadcast");
    }
    if let Some(v) = patch.auto_enrollment_allowed {
        match workspace.as_deref() {
            None => store
                .set_auto_enrollment_allowed(v)
                .map_err(|e| e.to_string())?,
            Some(ws) => store
                .set_workspace_auto_enrollment_allowed(ws, v)
                .map_err(|e| e.to_string())?,
        }
        changed_fields.push("orchestration.auto_enrollment_allowed");
    }
    if let Some(v) = patch.sandbox_strictness {
        match workspace.as_deref() {
            None => store.set_sandbox_strictness(v).map_err(|e| e.to_string())?,
            Some(ws) => store
                .set_workspace_sandbox_strictness(ws, v)
                .map_err(|e| e.to_string())?,
        }
        changed_fields.push("orchestration.sandbox_strictness");
    }
    if let Some(v) = patch.export_enabled {
        match workspace.as_deref() {
            None => store.set_export_enabled(v).map_err(|e| e.to_string())?,
            Some(ws) => store
                .set_workspace_export_enabled(ws, v)
                .map_err(|e| e.to_string())?,
        }
        changed_fields.push("orchestration.export_enabled");
    }

    if !changed_fields.is_empty() {
        store.save().map_err(|e| e.to_string())?;
        let scope = workspace.as_deref().unwrap_or("global");
        for field in changed_fields {
            record_settings_audit(field, scope, "update")?;
        }
    }
    Ok(store.effective(workspace.as_deref()))
}

/// `workspace: None` sets the global retention window (`days: None` keeps
/// records indefinitely). `Some(path)` sets that workspace's override,
/// which always names a concrete day count — clear an override with
/// `settings_reset_field` instead of passing `days: None` here.
#[tauri::command]
pub fn settings_set_retention_days(
    workspace: Option<String>,
    days: Option<u32>,
) -> Result<EffectiveSettings, String> {
    let mut store = open_settings_store();
    match workspace.as_deref() {
        None => store.set_retention_days(days).map_err(|e| e.to_string())?,
        Some(ws) => {
            let days = days.ok_or_else(|| {
                "a workspace override needs a concrete retention day count; use settings_reset_field to clear it".to_string()
            })?;
            store
                .set_workspace_retention_days(ws, days)
                .map_err(|e| e.to_string())?;
        }
    }
    store.save().map_err(|e| e.to_string())?;
    let scope = workspace.as_deref().unwrap_or("global");
    record_settings_audit("orchestration.retention_days", scope, "update")?;
    Ok(store.effective(workspace.as_deref()))
}

/// Composes Settings' orchestration policy with the Hub's existing
/// `WakePolicy` into one typed view, so Settings is the sole *editor* of
/// standing policy even though the wake-gate bit's storage deliberately
/// stays in `HubStore` — every C10-C13 wake path already reads it there,
/// so moving it would mean touching every one of those call sites instead
/// of composing at the IPC layer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StandingPolicySnapshot {
    /// Mirrors `hub::WakePolicy::default_requires_human_gate`.
    pub confirm_wakes: bool,
    /// Mirrors `hub::WakePolicy::allow_auto_wake`. When false, any wake
    /// attempting to bypass the human gate is rejected outright rather
    /// than silently falling back to requiring approval.
    pub allow_auto_wake: bool,
    pub orchestration: EffectiveOrchestrationPolicy,
}

#[tauri::command]
pub fn settings_get_standing_policy(
    workspace: Option<String>,
) -> Result<StandingPolicySnapshot, String> {
    let orchestration = open_settings_store()
        .effective(workspace.as_deref())
        .orchestration;
    let wake_policy = super::store::open_store()?
        .get_wake_policy()
        .map_err(|e| e.to_string())?;
    Ok(StandingPolicySnapshot {
        confirm_wakes: wake_policy.default_requires_human_gate,
        allow_auto_wake: wake_policy.allow_auto_wake,
        orchestration,
    })
}

/// Global only: the wake human-gate is not a per-workspace concept in
/// today's `WakePolicy`.
#[tauri::command]
pub fn settings_set_confirm_wakes(value: bool) -> Result<StandingPolicySnapshot, String> {
    let hub_store = super::store::open_store()?;
    let mut policy = hub_store.get_wake_policy().map_err(|e| e.to_string())?;
    policy.default_requires_human_gate = value;
    hub_store
        .set_wake_policy(&policy)
        .map_err(|e| e.to_string())?;
    record_settings_audit("orchestration.confirm_wakes", "global", "update")?;
    let orchestration = open_settings_store().effective(None).orchestration;
    Ok(StandingPolicySnapshot {
        confirm_wakes: value,
        allow_auto_wake: policy.allow_auto_wake,
        orchestration,
    })
}

/// Global only, same as `settings_set_confirm_wakes`.
#[tauri::command]
pub fn settings_set_allow_auto_wake(value: bool) -> Result<StandingPolicySnapshot, String> {
    let hub_store = super::store::open_store()?;
    let mut policy = hub_store.get_wake_policy().map_err(|e| e.to_string())?;
    policy.allow_auto_wake = value;
    hub_store
        .set_wake_policy(&policy)
        .map_err(|e| e.to_string())?;
    record_settings_audit("orchestration.allow_auto_wake", "global", "update")?;
    let orchestration = open_settings_store().effective(None).orchestration;
    Ok(StandingPolicySnapshot {
        confirm_wakes: policy.default_requires_human_gate,
        allow_auto_wake: value,
        orchestration,
    })
}

/// Per-agent budgets, exposed through Settings' typed command surface.
/// Storage stays in `HubStore`'s existing `agent_budgets` table — every C6
/// budget flow already reads/writes it — rather than duplicating it in
/// `settings.toml`.
#[tauri::command]
pub fn settings_list_agent_budgets() -> Result<Vec<hub::BudgetStatus>, String> {
    super::store::open_store()?
        .list_agent_budgets()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn settings_set_agent_budget(
    agent_id: String,
    limit_units: f64,
) -> Result<hub::BudgetStatus, String> {
    let status = super::store::open_store()?
        .set_agent_budget(&agent_id, limit_units)
        .map_err(|e| e.to_string())?;
    record_settings_audit("orchestration.agent_budget", &agent_id, "update")?;
    Ok(status)
}

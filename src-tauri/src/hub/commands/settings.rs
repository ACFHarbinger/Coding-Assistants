//! Typed, redacted settings IPC and scope resolution (Settings S2 / #128).
//!
//! Builds on the S1 `hub::SettingsStore` (#127). Commands here never send a
//! filesystem path to the frontend — only effective values, field-status
//! pills, and load-status diagnostics without their underlying path. Every
//! mutation is recorded on the dedicated settings audit stream, which is a
//! typed filter over the same Hub audit chain other commands already read.
use hub::{EffectiveSettings, LoadStatus, SettingsField, SettingsStore};

fn open_settings_store() -> SettingsStore {
    SettingsStore::open(hub::default_hub_home())
}

fn settings_field_name(field: SettingsField) -> &'static str {
    match field {
        SettingsField::BackupRetention => "storage.backup_retention",
        SettingsField::DefaultWorkspace => "general.default_workspace",
        SettingsField::DefaultSession => "general.default_session",
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

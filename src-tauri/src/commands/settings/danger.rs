//! Backing behavior for the Settings danger tab (S6 remainder / #132).
//!
//! Split out of `settings.rs` (already over the 500-LoC cap): irreversible
//! workspace purges with per-action audit events. Every command here deletes
//! hard and returns the affected row count so the UI can report exactly what
//! changed. Cancellation safety is structural — the frontend only invokes
//! these after typed target confirmation, and there is no partial path: the
//! combined workspace purge runs both halves before reporting.
use std::path::PathBuf;

/// Outcome of [`settings_purge_workspace_data`]: per-kind row counts so the
/// confirmation success copy can name exactly what was deleted.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspacePurgeReport {
    pub messages: usize,
    pub memories: usize,
}

fn record_settings_audit(field: &str, scope: &str, action: &str) -> Result<(), String> {
    super::store::open_store()?
        .record_settings_audit_event(field, scope, action)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn workspace_path(workspace: &str) -> Result<PathBuf, String> {
    let trimmed = workspace.trim();
    if trimmed.is_empty() {
        return Err("workspace must not be empty".to_string());
    }
    Ok(PathBuf::from(trimmed))
}

/// Permanently delete every message recorded against `workspace` (hard
/// `DELETE`, all kinds and statuses). Returns the deleted row count.
#[tauri::command]
pub fn settings_purge_workspace_transcript(workspace: String) -> Result<usize, String> {
    let path = workspace_path(&workspace)?;
    let deleted = super::store::open_store()?
        .purge_messages_in_workspace(&path)
        .map_err(|e| e.to_string())?;
    record_settings_audit("transcript", &workspace, &format!("purge:{deleted}"))?;
    Ok(deleted)
}

/// Permanently delete every memory recorded against `workspace`, all tiers
/// including non-stale ones. Returns the deleted row count.
#[tauri::command]
pub fn settings_purge_workspace_memories(workspace: String) -> Result<usize, String> {
    let path = workspace_path(&workspace)?;
    let deleted = super::store::open_store()?
        .purge_memories_in_workspace(&path)
        .map_err(|e| e.to_string())?;
    record_settings_audit("memory", &workspace, &format!("purge:{deleted}"))?;
    Ok(deleted)
}

/// Permanently delete the workspace's transcript *and* memories in one
/// confirmed action, with a single audit event naming both counts. Wakes are
/// agent-scoped (no workspace column) and harness registrations are runtime
/// state — neither is touched, and the danger-tab copy says so.
#[tauri::command]
pub fn settings_purge_workspace_data(workspace: String) -> Result<WorkspacePurgeReport, String> {
    let path = workspace_path(&workspace)?;
    let store = super::store::open_store()?;
    let messages = store
        .purge_messages_in_workspace(&path)
        .map_err(|e| e.to_string())?;
    let memories = store
        .purge_memories_in_workspace(&path)
        .map_err(|e| e.to_string())?;
    record_settings_audit(
        "workspace-data",
        &workspace,
        &format!("purge:messages={messages},memories={memories}"),
    )?;
    Ok(WorkspacePurgeReport { messages, memories })
}

#[cfg(test)]
mod tests {
    use super::super::store::open_store;
    use super::*;
    use crate::commands::commands::tests::CA_HOME_ENV_LOCK;

    #[test]
    fn empty_workspace_is_rejected_before_touching_the_store() {
        let err = settings_purge_workspace_transcript("   ".to_string()).unwrap_err();
        assert!(err.contains("must not be empty"), "{err}");
        let err = settings_purge_workspace_memories(String::new()).unwrap_err();
        assert!(err.contains("must not be empty"), "{err}");
        let err = settings_purge_workspace_data("  ".to_string()).unwrap_err();
        assert!(err.contains("must not be empty"), "{err}");
    }

    #[test]
    fn purge_commands_delete_exact_counts_and_leave_one_audit_event() {
        use hub::{MemoryScope, MemoryTier};

        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("CA_HOME");
        std::env::set_var("CA_HOME", home.path());

        let workspace = "/danger-tab-workspace";
        let store = open_store().unwrap();
        store
            .send_message(
                "grok",
                "claude",
                hub::MessageKind::Message,
                "doomed transcript",
                None,
                Some(workspace),
                None,
            )
            .unwrap();
        store
            .write_memory(
                MemoryTier::ShortTerm,
                MemoryScope::Global,
                Some("grok"),
                Some(workspace),
                None,
                "doomed memory",
                &[],
            )
            .unwrap();
        // A neighboring workspace must survive the purge.
        store
            .send_message(
                "grok",
                "claude",
                hub::MessageKind::Message,
                "safe transcript",
                None,
                Some("/neighbor-workspace"),
                None,
            )
            .unwrap();
        drop(store);

        let report = settings_purge_workspace_data(workspace.to_string()).unwrap();
        assert_eq!(report.messages, 1);
        assert_eq!(report.memories, 1);

        let store = open_store().unwrap();
        assert_eq!(store.list_messages(None, None).unwrap().len(), 1);
        assert_eq!(
            store.list_memories(None, None, None, true).unwrap().len(),
            0,
            "only the neighbor message (not a memory) may remain"
        );
        let events = store.list_settings_audit_events().unwrap();
        let expected = serde_json::json!({
            "field": "workspace-data",
            "scope": workspace,
        })
        .to_string();
        assert!(
            events.iter().any(|event| event.process_json == expected),
            "combined purge must leave one audit event"
        );

        match previous_home {
            Some(value) => std::env::set_var("CA_HOME", value),
            None => std::env::remove_var("CA_HOME"),
        }
    }
}

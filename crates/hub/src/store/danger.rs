//! S6 danger-zone purges with their audit rows in a single transaction
//! (#132, Codex review: deletion must never commit without its audit row,
//! and the combined transcript+memory purge is one atomic unit).
//!
//! Each method opens one `unchecked_transaction`, performs its hard
//! `DELETE`s, inserts the audit row, marks it approved, and commits last.
//! Counts are returned only after the commit succeeds — any failure before
//! that (a failing delete, a failing audit insert) drops the transaction
//! and rolls back everything, so a caller can never observe deleted rows
//! with no audit record, nor a half-completed combined purge.
use super::policies::{insert_audit_event, SETTINGS_AUDIT_ROOT};
use super::*;

fn approve_in(tx: &rusqlite::Transaction, event_id: &str) -> Result<(), HubError> {
    let changed = tx.execute(
        "UPDATE audit_events SET status = 'approved' WHERE id = ?1",
        params![event_id],
    )?;
    if changed == 0 {
        return Err(HubError::NotFound(format!("audit event {event_id}")));
    }
    Ok(())
}

fn settings_process_json(field: &str, scope: &str) -> String {
    serde_json::json!({ "field": field, "scope": scope }).to_string()
}

/// Workspace paths a purge must match: the path as given plus its
/// canonicalized form when they differ, so symlinked checkouts purge exactly
/// one workspace. Shared with the non-audited single-table purges.
pub(super) fn workspace_purge_paths(workspace: &Path) -> Vec<String> {
    let raw = workspace.to_string_lossy().into_owned();
    let canonical = workspace
        .canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .ok();
    match canonical {
        Some(resolved) if resolved != raw => vec![raw, resolved],
        _ => vec![raw],
    }
}

fn delete_where_workspace(
    tx: &rusqlite::Transaction,
    table: &str,
    workspace: &Path,
) -> Result<usize, HubError> {
    // Table names are fixed per call site, never caller input.
    let sql = format!("DELETE FROM {table} WHERE workspace_path = ?1");
    let mut total = 0;
    for path in workspace_purge_paths(workspace) {
        total += tx.execute(&sql, params![path])?;
    }
    Ok(total)
}

impl HubStore {
    /// Hard-delete the workspace transcript and its audit row atomically.
    /// The count returns only after the commit.
    pub fn purge_messages_in_workspace_with_audit(
        &self,
        workspace: &Path,
    ) -> Result<usize, HubError> {
        let scope = workspace.to_string_lossy().into_owned();
        let tx = self.conn.unchecked_transaction()?;
        let deleted = delete_where_workspace(&tx, "messages", workspace)?;
        let event = insert_audit_event(
            &tx,
            SETTINGS_AUDIT_ROOT,
            "transcript",
            &format!("purge:{deleted}"),
            &settings_process_json("transcript", &scope),
            None,
        )?;
        approve_in(&tx, &event.id)?;
        tx.commit()?;
        Ok(deleted)
    }

    /// Hard-delete the workspace memories (all tiers) and its audit row
    /// atomically. The count returns only after the commit.
    pub fn purge_memories_in_workspace_with_audit(
        &self,
        workspace: &Path,
    ) -> Result<usize, HubError> {
        let scope = workspace.to_string_lossy().into_owned();
        let tx = self.conn.unchecked_transaction()?;
        let deleted = delete_where_workspace(&tx, "memories", workspace)?;
        let event = insert_audit_event(
            &tx,
            SETTINGS_AUDIT_ROOT,
            "memory",
            &format!("purge:{deleted}"),
            &settings_process_json("memory", &scope),
            None,
        )?;
        approve_in(&tx, &event.id)?;
        tx.commit()?;
        Ok(deleted)
    }

    /// Hard-delete the workspace transcript *and* memories plus one audit
    /// row as a single atomic unit. Either half failing — or the audit
    /// insert failing — rolls back the other half too. Counts return only
    /// after the commit.
    pub fn purge_workspace_data_with_audit(
        &self,
        workspace: &Path,
    ) -> Result<(usize, usize), HubError> {
        let scope = workspace.to_string_lossy().into_owned();
        let tx = self.conn.unchecked_transaction()?;
        let messages = delete_where_workspace(&tx, "messages", workspace)?;
        let memories = delete_where_workspace(&tx, "memories", workspace)?;
        let event = insert_audit_event(
            &tx,
            SETTINGS_AUDIT_ROOT,
            "workspace-data",
            &format!("purge:messages={messages},memories={memories}"),
            &settings_process_json("workspace-data", &scope),
            None,
        )?;
        approve_in(&tx, &event.id)?;
        tx.commit()?;
        Ok((messages, memories))
    }
}

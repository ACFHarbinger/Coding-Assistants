//! Saved named workspaces (U24 / #317). Durable Hub SQLite rows that the
//! Orchestrate Workspace Root picker selects; `localStorage['ca.workspaceRoot']`
//! remains a last-used-path fallback.

use super::*;
use rusqlite::OptionalExtension;
use std::path::Path;

impl HubStore {
    pub fn ensure_workspaces_table(&self) -> Result<(), HubError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, created_at FROM workspaces ORDER BY name COLLATE NOCASE, created_at",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(id, name, path, created_at)| self.workspace_record(id, name, path, created_at))
            .collect()
    }

    /// Create or update a saved workspace. `id = None` inserts (or renames the
    /// existing row for the same path). `link_session_id` optionally points a
    /// work session at this workspace.
    pub fn save_workspace(
        &self,
        id: Option<&str>,
        name: &str,
        path: &str,
        link_session_id: Option<&str>,
    ) -> Result<WorkspaceRecord, HubError> {
        let name = validate_workspace_name(name)?;
        let path = normalize_workspace_path(path)?;
        let record = if let Some(id) = id.map(str::trim).filter(|id| !id.is_empty()) {
            self.update_workspace(id, &name, &path)?
        } else if let Some(existing) = self.find_workspace_by_path(&path)? {
            self.update_workspace(&existing.id, &name, &path)?
        } else {
            self.insert_workspace(&name, &path)?
        };
        if let Some(session_id) = link_session_id.map(str::trim).filter(|id| !id.is_empty()) {
            self.set_work_session_workspace(session_id, Some(&record.id))?;
        }
        self.get_workspace(&record.id)?
            .ok_or_else(|| HubError::NotFound(record.id))
    }

    pub fn delete_workspace(&self, id: &str) -> Result<(), HubError> {
        let id = id.trim();
        if id.is_empty() {
            return Err(HubError::Invalid("workspace id is required".into()));
        }
        self.conn.execute(
            "UPDATE work_sessions SET workspace_id = NULL WHERE workspace_id = ?1",
            params![id],
        )?;
        let deleted = self
            .conn
            .execute("DELETE FROM workspaces WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(HubError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub fn set_work_session_workspace(
        &self,
        session_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<WorkSessionRecord, HubError> {
        if self.get_work_session(session_id)?.is_none() {
            return Err(HubError::NotFound(session_id.to_string()));
        }
        let workspace_id = workspace_id.map(str::trim).filter(|id| !id.is_empty());
        if let Some(workspace_id) = workspace_id {
            if self.get_workspace(workspace_id)?.is_none() {
                return Err(HubError::NotFound(workspace_id.to_string()));
            }
            self.conn.execute(
                "UPDATE work_sessions SET workspace_id = ?1 WHERE id = ?2",
                params![workspace_id, session_id],
            )?;
        } else {
            self.conn.execute(
                "UPDATE work_sessions SET workspace_id = NULL WHERE id = ?1",
                params![session_id],
            )?;
        }
        self.get_work_session(session_id)?
            .ok_or_else(|| HubError::NotFound(session_id.to_string()))
    }

    fn insert_workspace(&self, name: &str, path: &str) -> Result<WorkspaceRecord, HubError> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO workspaces(id, name, path, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, path, created_at],
        )?;
        self.workspace_record(id, name.to_string(), path.to_string(), created_at)
    }

    fn update_workspace(
        &self,
        id: &str,
        name: &str,
        path: &str,
    ) -> Result<WorkspaceRecord, HubError> {
        let updated = self.conn.execute(
            "UPDATE workspaces SET name = ?1, path = ?2 WHERE id = ?3",
            params![name, path, id],
        )?;
        if updated == 0 {
            return Err(HubError::NotFound(id.to_string()));
        }
        self.get_workspace(id)?
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }

    fn find_workspace_by_path(&self, path: &str) -> Result<Option<WorkspaceRecord>, HubError> {
        self.conn
            .query_row(
                "SELECT id, name, path, created_at FROM workspaces WHERE path = ?1",
                params![path],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?
            .map(|(id, name, path, created_at)| self.workspace_record(id, name, path, created_at))
            .transpose()
    }

    fn get_workspace(&self, id: &str) -> Result<Option<WorkspaceRecord>, HubError> {
        self.conn
            .query_row(
                "SELECT id, name, path, created_at FROM workspaces WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?
            .map(|(id, name, path, created_at)| self.workspace_record(id, name, path, created_at))
            .transpose()
    }

    fn workspace_record(
        &self,
        id: String,
        name: String,
        path: String,
        created_at: String,
    ) -> Result<WorkspaceRecord, HubError> {
        let linked = self
            .conn
            .query_row(
                "SELECT id, name FROM work_sessions WHERE workspace_id = ?1
                 ORDER BY created_at DESC LIMIT 1",
                params![id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        Ok(WorkspaceRecord {
            id,
            name,
            path,
            created_at,
            linked_session_id: linked.as_ref().map(|row| row.0.clone()),
            linked_session_name: linked.map(|row| row.1),
        })
    }
}

fn validate_workspace_name(name: &str) -> Result<String, HubError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(HubError::Invalid(
            "workspace name must be between 1 and 120 characters".into(),
        ));
    }
    Ok(name.to_string())
}

fn normalize_workspace_path(path: &str) -> Result<String, HubError> {
    let trimmed = path.trim();
    if trimmed.is_empty() || !Path::new(trimmed).is_absolute() {
        return Err(HubError::Invalid(
            "workspace path must be an absolute path".into(),
        ));
    }
    if trimmed == "/" {
        return Ok("/".into());
    }
    Ok(trimmed.trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_list_rename_and_delete_a_workspace() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let created = store
            .save_workspace(None, "Coding Assistants", "/tmp/ca-u24-a", None)
            .unwrap();
        assert_eq!(created.name, "Coding Assistants");
        assert_eq!(created.path, "/tmp/ca-u24-a");
        assert!(created.linked_session_id.is_none());

        let listed = store.list_workspaces().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        let renamed = store
            .save_workspace(Some(&created.id), "CA", "/tmp/ca-u24-b", None)
            .unwrap();
        assert_eq!(renamed.id, created.id);
        assert_eq!(renamed.name, "CA");
        assert_eq!(renamed.path, "/tmp/ca-u24-b");

        store.delete_workspace(&created.id).unwrap();
        assert!(store.list_workspaces().unwrap().is_empty());
    }

    #[test]
    fn saving_the_same_path_renames_the_existing_row() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let first = store
            .save_workspace(None, "One", "/tmp/ca-u24-same", None)
            .unwrap();
        let second = store
            .save_workspace(None, "Two", "/tmp/ca-u24-same/", None)
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.name, "Two");
        assert_eq!(store.list_workspaces().unwrap().len(), 1);
    }

    #[test]
    fn linking_a_session_and_delete_clears_the_fk() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = store
            .save_workspace(None, "Linked", "/tmp/ca-u24-link", None)
            .unwrap();
        let session = store.create_work_session("Team chat").unwrap();
        let linked = store
            .save_workspace(
                Some(&workspace.id),
                "Linked",
                "/tmp/ca-u24-link",
                Some(&session.id),
            )
            .unwrap();
        assert_eq!(
            linked.linked_session_id.as_deref(),
            Some(session.id.as_str())
        );
        assert_eq!(linked.linked_session_name.as_deref(), Some("Team chat"));

        let fetched = store.get_work_session(&session.id).unwrap().unwrap();
        assert_eq!(fetched.workspace_id.as_deref(), Some(workspace.id.as_str()));

        store.delete_workspace(&workspace.id).unwrap();
        let cleared = store.get_work_session(&session.id).unwrap().unwrap();
        assert!(cleared.workspace_id.is_none());
    }

    #[test]
    fn rejects_relative_paths_and_empty_names() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store.save_workspace(None, "x", "relative", None).is_err());
        assert!(store
            .save_workspace(None, "   ", "/tmp/ca-u24-x", None)
            .is_err());
        assert!(store.delete_workspace("missing").is_err());
    }

    #[test]
    fn create_work_session_can_attach_a_workspace() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = store
            .save_workspace(None, "Proj", "/tmp/ca-u24-create", None)
            .unwrap();
        let session = store
            .create_work_session_in_workspace("Scoped", Some(&workspace.id))
            .unwrap();
        assert_eq!(session.workspace_id.as_deref(), Some(workspace.id.as_str()));
        assert!(store
            .create_work_session_in_workspace("Bad", Some("nope"))
            .is_err());
    }
}

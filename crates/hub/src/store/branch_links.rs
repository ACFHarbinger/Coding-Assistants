//! Persisted manual branch → issue links (U19 / #313).

use super::*;
use chrono::Utc;
use rusqlite::params;
use std::collections::BTreeMap;

impl HubStore {
    pub fn ensure_branch_link_table(&self) -> Result<(), HubError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS branch_issue_links (
                workspace TEXT NOT NULL,
                branch TEXT NOT NULL,
                issue_number INTEGER NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (workspace, branch)
            );
            "#,
        )?;
        Ok(())
    }

    pub fn list_branch_issue_links(
        &self,
        workspace: &str,
    ) -> Result<BTreeMap<String, i64>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT branch, issue_number FROM branch_issue_links WHERE workspace = ?1 ORDER BY branch",
        )?;
        let rows = stmt.query_map(params![workspace], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        Ok(rows.filter_map(|row| row.ok()).collect())
    }

    pub fn set_branch_issue_link(
        &self,
        workspace: &str,
        branch: &str,
        issue_number: Option<i64>,
    ) -> Result<(), HubError> {
        let workspace = workspace.trim();
        let branch = branch.trim();
        if workspace.is_empty() || !std::path::Path::new(workspace).is_absolute() {
            return Err(HubError::Invalid(
                "workspace must be an absolute path".into(),
            ));
        }
        if branch.is_empty() || branch.contains('\0') {
            return Err(HubError::Invalid("branch name is required".into()));
        }
        match issue_number {
            None => {
                self.conn.execute(
                    "DELETE FROM branch_issue_links WHERE workspace = ?1 AND branch = ?2",
                    params![workspace, branch],
                )?;
            }
            Some(number) if number > 0 => {
                self.conn.execute(
                    "INSERT INTO branch_issue_links(workspace, branch, issue_number, updated_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(workspace, branch) DO UPDATE SET
                        issue_number = excluded.issue_number,
                        updated_at = excluded.updated_at",
                    params![workspace, branch, number, Utc::now().to_rfc3339()],
                )?;
            }
            Some(_) => {
                return Err(HubError::Invalid("issue number must be positive".into()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn set_list_and_clear_a_branch_issue_link() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let ws = "/tmp/ca-u19-ws";
        store
            .set_branch_issue_link(ws, "feat/spike", Some(313))
            .unwrap();
        let links = store.list_branch_issue_links(ws).unwrap();
        assert_eq!(links.get("feat/spike"), Some(&313));
        store.set_branch_issue_link(ws, "feat/spike", None).unwrap();
        assert!(store.list_branch_issue_links(ws).unwrap().is_empty());
    }

    #[test]
    fn rejects_relative_workspace_and_non_positive_issue() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store.set_branch_issue_link("rel", "main", Some(1)).is_err());
        assert!(store
            .set_branch_issue_link("/abs", "main", Some(0))
            .is_err());
    }
}

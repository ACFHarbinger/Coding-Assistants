//! Role definitions and agent↔role assignment.

use super::super::*;
use super::CTO_ROLE_ID;

fn row_to_role(row: &rusqlite::Row) -> rusqlite::Result<Role> {
    let responsibilities_json: String = row.get(8)?;
    Ok(Role {
        id: row.get(0)?,
        display_name: row.get(1)?,
        is_builtin: row.get::<_, i64>(2)? != 0,
        daily_ungated_quota: row.get(3)?,
        max_broadcast_recipients: row.get(4)?,
        can_archive_messages: row.get::<_, i64>(5)? != 0,
        can_update_agent_roles: row.get::<_, i64>(6)? != 0,
        can_allocate_tasks: row.get::<_, i64>(7)? != 0,
        responsibilities: serde_json::from_str(&responsibilities_json).unwrap_or_default(),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub(super) const ROLE_COLUMNS: &str =
    "id, display_name, is_builtin, daily_ungated_quota, max_broadcast_recipients, \
     can_archive_messages, can_update_agent_roles, can_allocate_tasks, responsibilities_json, \
     created_at, updated_at";

impl HubStore {
    /// Creates or updates a non-builtin role. Never touches `is_builtin`
    /// (always `false` for caller-created roles) — the protected `cto`
    /// role can only ever be created by [`Self::ensure_builtin_roles`].
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_role(
        &self,
        id: &str,
        display_name: &str,
        daily_ungated_quota: Option<i64>,
        max_broadcast_recipients: Option<i64>,
        can_archive_messages: bool,
        can_update_agent_roles: bool,
        can_allocate_tasks: bool,
        responsibilities: &[String],
    ) -> Result<Role, HubError> {
        let id = id.trim();
        let display_name = display_name.trim();
        if id.is_empty() || display_name.is_empty() {
            return Err(HubError::Invalid(
                "role id and display_name must not be empty".into(),
            ));
        }
        if id == CTO_ROLE_ID {
            return Err(HubError::Invalid(
                "the cto role is protected and cannot be created or edited directly".into(),
            ));
        }
        if let Some(existing) = self.get_role(id)? {
            if existing.is_builtin {
                return Err(HubError::Invalid(format!(
                    "role {id} is builtin and cannot be edited"
                )));
            }
        }
        let now = Utc::now().to_rfc3339();
        let responsibilities_json = serde_json::to_string(responsibilities)
            .map_err(|e| HubError::Invalid(e.to_string()))?;
        self.conn.execute(
            r#"
            INSERT INTO roles(
                id, display_name, is_builtin, daily_ungated_quota, max_broadcast_recipients,
                can_archive_messages, can_update_agent_roles, can_allocate_tasks,
                responsibilities_json, created_at, updated_at
            ) VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
            ON CONFLICT(id) DO UPDATE SET
                display_name = excluded.display_name,
                daily_ungated_quota = excluded.daily_ungated_quota,
                max_broadcast_recipients = excluded.max_broadcast_recipients,
                can_archive_messages = excluded.can_archive_messages,
                can_update_agent_roles = excluded.can_update_agent_roles,
                can_allocate_tasks = excluded.can_allocate_tasks,
                responsibilities_json = excluded.responsibilities_json,
                updated_at = excluded.updated_at
            "#,
            params![
                id,
                display_name,
                daily_ungated_quota,
                max_broadcast_recipients,
                can_archive_messages as i64,
                can_update_agent_roles as i64,
                can_allocate_tasks as i64,
                responsibilities_json,
                now,
            ],
        )?;
        self.get_role(id)?
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }

    pub fn get_role(&self, id: &str) -> Result<Option<Role>, HubError> {
        self.conn
            .query_row(
                &format!("SELECT {ROLE_COLUMNS} FROM roles WHERE id = ?1"),
                params![id],
                row_to_role,
            )
            .optional()
            .map_err(HubError::from)
    }

    pub fn list_roles(&self) -> Result<Vec<Role>, HubError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {ROLE_COLUMNS} FROM roles ORDER BY display_name"
        ))?;
        let rows = stmt.query_map([], row_to_role)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Rejects deleting a builtin role or one still assigned to any agent —
    /// unassign first so an agent never silently loses every permission.
    pub fn delete_role(&self, id: &str) -> Result<(), HubError> {
        let role = self
            .get_role(id)?
            .ok_or_else(|| HubError::NotFound(id.to_string()))?;
        if role.is_builtin {
            return Err(HubError::Invalid(format!(
                "role {id} is builtin and cannot be deleted"
            )));
        }
        let assignees: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM agent_role_assignments WHERE role_id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        if assignees > 0 {
            return Err(HubError::Invalid(format!(
                "role {id} is still assigned to {assignees} agent(s); unassign first"
            )));
        }
        self.conn
            .execute("DELETE FROM roles WHERE id = ?1", params![id])?;
        self.conn.execute(
            "DELETE FROM role_provider_defaults WHERE role_id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn assign_agent_role(&self, agent_id: &str, role_id: &str) -> Result<(), HubError> {
        self.get_role(role_id)?
            .ok_or_else(|| HubError::NotFound(role_id.to_string()))?;
        self.conn.execute(
            "INSERT OR IGNORE INTO agent_role_assignments(agent_id, role_id, assigned_at) VALUES (?1, ?2, ?3)",
            params![agent_id, role_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// The `cto` role can never be unassigned from `human` — it's the
    /// human's permanent, always-full-access identity in this system.
    pub fn unassign_agent_role(&self, agent_id: &str, role_id: &str) -> Result<(), HubError> {
        if agent_id == "human" && role_id == CTO_ROLE_ID {
            return Err(HubError::Invalid(
                "the cto role cannot be unassigned from human".into(),
            ));
        }
        self.conn.execute(
            "DELETE FROM agent_role_assignments WHERE agent_id = ?1 AND role_id = ?2",
            params![agent_id, role_id],
        )?;
        Ok(())
    }

    pub fn list_agent_roles(&self, agent_id: &str) -> Result<Vec<Role>, HubError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {cols} FROM roles r \
             JOIN agent_role_assignments a ON a.role_id = r.id \
             WHERE a.agent_id = ?1 ORDER BY r.display_name",
            cols = ROLE_COLUMNS
                .split(", ")
                .map(|c| format!("r.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))?;
        let rows = stmt.query_map(params![agent_id], row_to_role)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn cto_role_cannot_be_created_edited_deleted_or_unassigned_from_human() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store
            .upsert_role(CTO_ROLE_ID, "CTO", None, None, true, true, true, &[])
            .is_err());
        assert!(store.delete_role(CTO_ROLE_ID).is_err());
        assert!(store.unassign_agent_role("human", CTO_ROLE_ID).is_err());
    }

    #[test]
    fn upsert_role_creates_and_updates() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let role = store
            .upsert_role(
                "reviewer",
                "Reviewer",
                Some(10),
                Some(5),
                false,
                false,
                false,
                &["reviewer".to_string()],
            )
            .unwrap();
        assert_eq!(role.display_name, "Reviewer");
        assert_eq!(role.daily_ungated_quota, Some(10));
        assert!(!role.is_builtin);

        let updated = store
            .upsert_role(
                "reviewer",
                "Senior Reviewer",
                Some(20),
                Some(5),
                true,
                false,
                false,
                &["reviewer".to_string(), "role_manager".to_string()],
            )
            .unwrap();
        assert_eq!(updated.display_name, "Senior Reviewer");
        assert_eq!(updated.daily_ungated_quota, Some(20));
        assert!(updated.can_archive_messages);
        assert_eq!(updated.responsibilities.len(), 2);

        assert!(store
            .list_roles()
            .unwrap()
            .iter()
            .any(|role| role.id == "reviewer"));
    }

    #[test]
    fn delete_role_rejects_builtin_and_still_assigned_roles() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store
            .upsert_role(
                "planner",
                "Planner",
                Some(5),
                Some(3),
                false,
                false,
                false,
                &[],
            )
            .unwrap();
        store.assign_agent_role("grok", "planner").unwrap();

        assert!(store.delete_role("planner").is_err());
        store.unassign_agent_role("grok", "planner").unwrap();
        assert!(store.delete_role("planner").is_ok());
        assert!(store.get_role("planner").unwrap().is_none());
    }
}

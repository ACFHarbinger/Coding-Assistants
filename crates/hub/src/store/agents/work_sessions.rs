//! Work-session creation is kept separate from the agent record CRUD so the
//! store module stays under the repository's 500-line source-unit limit.

use super::*;

impl HubStore {
    /// Creates a named work-session chat. When `members` is supplied, it is
    /// the complete initial roster (and may be empty); otherwise the current
    /// persisted team is retained for backwards-compatible callers.
    pub fn create_work_session_with_members(
        &self,
        name: &str,
        members: Option<&[String]>,
    ) -> Result<WorkSessionRecord, HubError> {
        let name = name.trim();
        if name.is_empty() || name.len() > 120 {
            return Err(HubError::Invalid(
                "work session name must be between 1 and 120 characters".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO work_sessions(id, name, created_at) VALUES (?1, ?2, ?3)",
            params![id, name, created_at],
        )?;
        if let Some(members) = members {
            for member in members {
                let member = member.trim();
                if member.is_empty() {
                    return Err(HubError::Invalid(
                        "work-session member must not be empty".into(),
                    ));
                }
                let exists: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM agents WHERE id = ?1)",
                    params![member],
                    |row| row.get(0),
                )?;
                if !exists {
                    return Err(HubError::NotFound(member.to_string()));
                }
                tx.execute(
                    "INSERT OR IGNORE INTO work_session_members(session_id, agent_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![id, member, created_at],
                )?;
            }
        } else {
            tx.execute(
                "INSERT INTO work_session_members(session_id, agent_id, created_at)
                 SELECT ?1, id, ?2 FROM agents WHERE team_member = 1",
                params![id, created_at],
            )?;
        }
        tx.commit()?;
        self.get_work_session(&id)?
            .ok_or_else(|| HubError::NotFound(id))
    }

    /// Backwards-compatible team-seeded work-session creation.
    pub fn create_work_session(&self, name: &str) -> Result<WorkSessionRecord, HubError> {
        self.create_work_session_with_members(name, None)
    }
}

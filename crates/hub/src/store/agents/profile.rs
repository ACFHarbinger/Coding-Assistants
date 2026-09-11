//! Agent profile management: display name updates, validation, and Settings audit events (#314, U20).

use super::super::*;

impl HubStore {
    /// Updates `agent_id`'s display name, validating length and collisions,
    /// updating the store, and recording a Settings-style audit event on the
    /// shared Hub audit chain.
    ///
    /// Any roster identity (including `human`) can be renamed.
    pub fn set_agent_display_name(
        &self,
        id: &str,
        display_name: &str,
    ) -> Result<AgentRecord, HubError> {
        let trimmed = display_name.trim();
        if trimmed.is_empty() {
            return Err(HubError::Invalid("display name cannot be empty".into()));
        }
        if trimmed.chars().count() > 64 {
            return Err(HubError::Invalid(
                "display name exceeds maximum length of 64 characters".into(),
            ));
        }

        let agents = self.list_agents()?;
        let current = agents
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| HubError::NotFound(id.to_string()))?;

        // Collision check: prevent two identities having the same display name
        if let Some(collision) = agents
            .iter()
            .find(|a| a.id != id && a.display_name.eq_ignore_ascii_case(trimmed))
        {
            return Err(HubError::Invalid(format!(
                "display name '{trimmed}' is already in use by agent '{}'",
                collision.id
            )));
        }

        if current.display_name == trimmed {
            return Ok(current.clone());
        }

        let old_name = current.display_name.clone();
        self.conn.execute(
            "UPDATE agents SET display_name = ?1 WHERE id = ?2",
            params![trimmed, id],
        )?;

        // Record a Settings-style audit event on the shared Hub audit chain
        self.record_settings_audit_event(
            &format!("agent.{id}.display_name"),
            id,
            &format!("rename:{old_name}->{trimmed}"),
        )?;

        self.list_agents()?
            .into_iter()
            .find(|a| a.id == id)
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn set_agent_display_name_updates_record_and_records_audit_event() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        // Renaming human identity works cleanly
        let updated_human = store.set_agent_display_name("human", "Alice Dev").unwrap();
        assert_eq!(updated_human.id, "human");
        assert_eq!(updated_human.display_name, "Alice Dev");

        // Refetched from list_agents reflects the new name
        let refetched = store
            .list_agents()
            .unwrap()
            .into_iter()
            .find(|a| a.id == "human")
            .unwrap();
        assert_eq!(refetched.display_name, "Alice Dev");

        // Renaming an AI agent works
        let updated_claude = store
            .set_agent_display_name("claude", "Claude Architect")
            .unwrap();
        assert_eq!(updated_claude.display_name, "Claude Architect");

        // Audit events verify settings-scoped log
        let audit_events = store.list_settings_audit_events().unwrap();
        assert!(audit_events
            .iter()
            .any(|ev| ev.path == "agent.human.display_name"
                && ev.operation == "rename:Human->Alice Dev"));
        assert!(audit_events
            .iter()
            .any(|ev| ev.path == "agent.claude.display_name"
                && ev.operation == "rename:Claude Code->Claude Architect"));
    }

    #[test]
    fn set_agent_display_name_rejects_empty_or_whitespace() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        assert!(matches!(
            store.set_agent_display_name("human", ""),
            Err(HubError::Invalid(_))
        ));
        assert!(matches!(
            store.set_agent_display_name("human", "   "),
            Err(HubError::Invalid(_))
        ));
    }

    #[test]
    fn set_agent_display_name_rejects_excessive_length() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let long_name = "a".repeat(65);
        assert!(matches!(
            store.set_agent_display_name("human", &long_name),
            Err(HubError::Invalid(_))
        ));
    }

    #[test]
    fn set_agent_display_name_rejects_name_collision() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        // Attempting to rename human to "claude code" (which matches claude's display name)
        let err = store
            .set_agent_display_name("human", "claude code")
            .unwrap_err();
        assert!(
            matches!(err, HubError::Invalid(msg) if msg.contains("already in use by agent 'claude'"))
        );
    }

    #[test]
    fn set_agent_display_name_rejects_nonexistent_agent() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        assert!(matches!(
            store.set_agent_display_name("nonexistent", "Ghost"),
            Err(HubError::NotFound(_))
        ));
    }

    #[test]
    fn set_agent_display_name_same_name_is_noop() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let initial_audits = store.list_settings_audit_events().unwrap().len();
        let res = store.set_agent_display_name("human", "Human").unwrap();
        assert_eq!(res.display_name, "Human");
        let post_audits = store.list_settings_audit_events().unwrap().len();
        assert_eq!(initial_audits, post_audits);
    }
}

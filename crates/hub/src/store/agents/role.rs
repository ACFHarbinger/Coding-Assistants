//! Team role assignment: role updates, validation, and Settings audit events (#315, U21).

use super::super::*;

impl HubStore {
    /// Sets or clears `agent_id`'s team role.
    ///
    /// If `role` is `None`, empty, or whitespace-only, the agent's role is cleared (`NULL`).
    /// If `role` is `Some(r)`, the role is trimmed and capped at 64 characters.
    ///
    /// Preserves descriptive roles: preset conventions ("lead", "reviewer", "implementer", "observer")
    /// or any custom label.
    ///
    /// Emits a Settings-style audit event on the shared Hub audit chain:
    /// `field: agent.<id>.role`, `scope: <id>`, `action: set_role:<old>-><new>` or `clear_role:<old>`.
    /// Safe no-op if the role is unchanged.
    pub fn set_agent_role(&self, id: &str, role: Option<&str>) -> Result<AgentRecord, HubError> {
        let cleaned_role = match role {
            Some(r) => {
                let trimmed = r.trim();
                if trimmed.is_empty() {
                    None
                } else if trimmed.chars().count() > 64 {
                    return Err(HubError::Invalid(
                        "team role exceeds maximum length of 64 characters".into(),
                    ));
                } else {
                    Some(trimmed.to_string())
                }
            }
            None => None,
        };

        let agents = self.list_agents()?;
        let current = agents
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| HubError::NotFound(id.to_string()))?;

        if current.role == cleaned_role {
            return Ok(current.clone());
        }

        let old_role_desc = current.role.as_deref().unwrap_or("<none>");

        self.conn.execute(
            "UPDATE agents SET role = ?1 WHERE id = ?2",
            params![cleaned_role, id],
        )?;

        let action = match &cleaned_role {
            Some(new_role) => format!("set_role:{old_role_desc}->{new_role}"),
            None => format!("clear_role:{old_role_desc}"),
        };

        self.record_settings_audit_event(&format!("agent.{id}.role"), id, &action)?;

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
    fn set_agent_role_assigns_role_and_records_audit_event() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        // Assign role to claude
        let updated = store.set_agent_role("claude", Some("lead")).unwrap();
        assert_eq!(updated.id, "claude");
        assert_eq!(updated.role.as_deref(), Some("lead"));

        // Refetched from list_agents reflects the role
        let refetched = store
            .list_agents()
            .unwrap()
            .into_iter()
            .find(|a| a.id == "claude")
            .unwrap();
        assert_eq!(refetched.role.as_deref(), Some("lead"));

        // Audit events verify settings-scoped log
        let audit_events = store.list_settings_audit_events().unwrap();
        assert!(audit_events
            .iter()
            .any(|e| e.path == "agent.claude.role" && e.operation == "set_role:<none>->lead"));
    }

    #[test]
    fn set_agent_role_works_for_human_with_custom_label() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let updated = store
            .set_agent_role("human", Some("Lead Architect"))
            .unwrap();
        assert_eq!(updated.id, "human");
        assert_eq!(updated.role.as_deref(), Some("Lead Architect"));

        let audit_events = store.list_settings_audit_events().unwrap();
        assert!(audit_events
            .iter()
            .any(|e| e.path == "agent.human.role"
                && e.operation == "set_role:<none>->Lead Architect"));
    }

    #[test]
    fn set_agent_role_clears_role_with_none_or_empty() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        // Assign first
        store.set_agent_role("gemini", Some("reviewer")).unwrap();

        // Clear with None
        let cleared = store.set_agent_role("gemini", None).unwrap();
        assert_eq!(cleared.role, None);

        // Re-assign
        store.set_agent_role("gemini", Some("implementer")).unwrap();

        // Clear with whitespace
        let cleared_empty = store.set_agent_role("gemini", Some("   ")).unwrap();
        assert_eq!(cleared_empty.role, None);

        let audit_events = store.list_settings_audit_events().unwrap();
        assert!(audit_events
            .iter()
            .any(|e| e.path == "agent.gemini.role" && e.operation == "clear_role:reviewer"));
        assert!(audit_events
            .iter()
            .any(|e| e.path == "agent.gemini.role" && e.operation == "clear_role:implementer"));
    }

    #[test]
    fn set_agent_role_no_op_on_same_value() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        store.set_agent_role("grok", Some("observer")).unwrap();
        let audit_count_before = store.list_settings_audit_events().unwrap().len();

        // Setting identical role does nothing
        let updated = store.set_agent_role("grok", Some("observer")).unwrap();
        assert_eq!(updated.role.as_deref(), Some("observer"));

        let audit_count_after = store.list_settings_audit_events().unwrap().len();
        assert_eq!(audit_count_before, audit_count_after);
    }

    #[test]
    fn set_agent_role_rejects_overlong_string() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let long_role = "a".repeat(65);
        let err = store.set_agent_role("claude", Some(&long_role));
        assert!(matches!(err, Err(HubError::Invalid(_))));
    }

    #[test]
    fn set_agent_role_fails_on_nonexistent_agent() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let err = store.set_agent_role("does_not_exist", Some("lead"));
        assert!(matches!(err, Err(HubError::NotFound(_))));
    }
}

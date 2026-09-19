//! Animated V-Tuber avatar presence: setting animated_avatar toggle, character,
//! and Settings audit events (U17 / #307).

use super::super::*;

impl HubStore {
    /// Sets or clears `agent_id`'s animated V-Tuber avatar preference.
    ///
    /// Preserves opt-in: `enabled = false` returns the identity to the static avatar path.
    /// Emits a Settings-style audit event on the shared Hub audit chain:
    /// `field: agent.<id>.animated_avatar`, `scope: <id>`, `action: set_animated_avatar:<old>-><new>,character:<c>`.
    pub fn set_agent_animated_avatar(
        &self,
        id: &str,
        enabled: bool,
        character: Option<&str>,
    ) -> Result<AgentRecord, HubError> {
        let cleaned_character = match character {
            Some(c) => {
                let trimmed = c.trim();
                if trimmed.is_empty() {
                    None
                } else if trimmed.chars().count() > 64 {
                    return Err(HubError::Invalid(
                        "character name exceeds maximum length of 64 characters".into(),
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

        if current.animated_avatar == enabled && current.vtuber_character == cleaned_character {
            return Ok(current.clone());
        }

        self.conn.execute(
            "UPDATE agents SET animated_avatar = ?1, vtuber_character = ?2 WHERE id = ?3",
            params![if enabled { 1 } else { 0 }, cleaned_character, id],
        )?;

        let char_desc = cleaned_character.as_deref().unwrap_or("<none>");
        let action = format!(
            "set_animated_avatar:{}->{},character:{}",
            current.animated_avatar, enabled, char_desc
        );
        self.record_settings_audit_event(&format!("agent.{id}.animated_avatar"), id, &action)?;

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
    fn set_agent_animated_avatar_toggles_and_audits() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let updated = store
            .set_agent_animated_avatar("claude", true, None)
            .unwrap();
        assert_eq!(updated.id, "claude");
        assert!(updated.animated_avatar);
        assert_eq!(updated.vtuber_character, None);

        let refetched = store
            .list_agents()
            .unwrap()
            .into_iter()
            .find(|a| a.id == "claude")
            .unwrap();
        assert!(refetched.animated_avatar);

        let audit_events = store.list_settings_audit_events().unwrap();
        assert!(audit_events.iter().any(|e| {
            e.path == "agent.claude.animated_avatar"
                && e.operation.contains("set_animated_avatar:false->true")
        }));

        let disabled = store
            .set_agent_animated_avatar("claude", false, None)
            .unwrap();
        assert!(!disabled.animated_avatar);
    }

    #[test]
    fn set_agent_animated_avatar_with_character() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let updated = store
            .set_agent_animated_avatar("gemini", true, Some("shizuku"))
            .unwrap();
        assert!(updated.animated_avatar);
        assert_eq!(updated.vtuber_character.as_deref(), Some("shizuku"));

        let refetched = store
            .list_agents()
            .unwrap()
            .into_iter()
            .find(|a| a.id == "gemini")
            .unwrap();
        assert_eq!(refetched.vtuber_character.as_deref(), Some("shizuku"));
    }

    #[test]
    fn set_agent_animated_avatar_rejects_nonexistent_or_oversized() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let not_found = store.set_agent_animated_avatar("nonexistent_id", true, None);
        assert!(matches!(not_found, Err(HubError::NotFound(_))));

        let long_name = "a".repeat(65);
        let oversized = store.set_agent_animated_avatar("claude", true, Some(&long_name));
        assert!(matches!(oversized, Err(HubError::Invalid(_))));
    }
}

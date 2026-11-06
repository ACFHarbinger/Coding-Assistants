//! Agent profile images. Reuses the same file-backed attachment store as
//! message attachments (`save_attachment`) — an avatar is just an
//! attachment an `agents` row points at.

use super::super::*;

impl HubStore {
    /// Sets `agent_id`'s profile image, storing the bytes as a durable
    /// attachment and pointing `agents.avatar_attachment_id` at it. Any
    /// caller may set any agent's avatar — including an agent setting its
    /// own, which is the point: there is no separate "avatar owner"
    /// identity check, same trust model as the rest of this local store.
    pub fn set_agent_avatar(
        &self,
        agent_id: &str,
        filename: &str,
        mime: &str,
        data: &[u8],
    ) -> Result<AgentRecord, HubError> {
        if !self.list_agents()?.iter().any(|agent| agent.id == agent_id) {
            return Err(HubError::NotFound(agent_id.to_string()));
        }
        let attachment = self.save_attachment(filename, mime, data)?;
        self.conn.execute(
            "UPDATE agents SET avatar_attachment_id = ?1 WHERE id = ?2",
            params![attachment.id, agent_id],
        )?;
        self.list_agents()?
            .into_iter()
            .find(|agent| agent.id == agent_id)
            .ok_or_else(|| HubError::NotFound(agent_id.to_string()))
    }

    /// Clears `agent_id`'s avatar reference. The underlying attachment file
    /// is left in place (attachments are never hard-deleted elsewhere in
    /// this store either) — only the pointer on `agents` is cleared.
    pub fn clear_agent_avatar(&self, agent_id: &str) -> Result<AgentRecord, HubError> {
        let updated = self.conn.execute(
            "UPDATE agents SET avatar_attachment_id = NULL WHERE id = ?1",
            params![agent_id],
        )?;
        if updated == 0 {
            return Err(HubError::NotFound(agent_id.to_string()));
        }
        self.list_agents()?
            .into_iter()
            .find(|agent| agent.id == agent_id)
            .ok_or_else(|| HubError::NotFound(agent_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn set_agent_avatar_persists_the_attachment_and_updates_the_pointer() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();

        let updated = store
            .set_agent_avatar("claude", "avatar.png", "image/png", b"fake-png-bytes")
            .unwrap();
        let attachment_id = updated
            .avatar_attachment_id
            .clone()
            .expect("avatar_attachment_id must be set after set_agent_avatar");

        let (record, data) = store.read_attachment(&attachment_id).unwrap().unwrap();
        assert_eq!(record.filename, "avatar.png");
        assert_eq!(data, b"fake-png-bytes");

        // list_agents must reflect the same pointer, not just the returned record.
        let refetched = store
            .list_agents()
            .unwrap()
            .into_iter()
            .find(|a| a.id == "claude")
            .unwrap();
        assert_eq!(refetched.avatar_attachment_id, Some(attachment_id));
    }

    #[test]
    fn set_agent_avatar_on_an_unknown_agent_is_not_found() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store
            .set_agent_avatar("nonexistent", "a.png", "image/png", b"x")
            .is_err());
    }

    #[test]
    fn clear_agent_avatar_nulls_the_pointer_without_deleting_the_attachment() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let updated = store
            .set_agent_avatar("grok", "avatar.png", "image/png", b"bytes")
            .unwrap();
        let attachment_id = updated.avatar_attachment_id.unwrap();

        let cleared = store.clear_agent_avatar("grok").unwrap();
        assert_eq!(cleared.avatar_attachment_id, None);

        // The attachment itself must still be readable — clearing the
        // pointer is not the same as deleting the file.
        assert!(store.read_attachment(&attachment_id).unwrap().is_some());
    }

    #[test]
    fn clear_agent_avatar_on_an_unknown_agent_is_not_found() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(store.clear_agent_avatar("nonexistent").is_err());
    }
}

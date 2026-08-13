use super::super::*;

impl HubStore {
    /// Wake every enrolled teammate except the sender and `system`.
    /// Messager/Orchestrate team sends must use this instead of waking a single harness.
    pub fn request_team_wakes(
        &self,
        from_agent: &str,
        reason: Option<&str>,
        message_id: Option<&str>,
        requires_human_gate: bool,
    ) -> Result<Vec<WakeRecord>, HubError> {
        let mut wakes = Vec::new();
        for member in self.list_team_members()? {
            if member.id != from_agent && member.id != "system" {
                wakes.push(self.request_wake(
                    &member.id,
                    reason,
                    message_id,
                    requires_human_gate,
                )?);
            }
        }
        Ok(wakes)
    }
}

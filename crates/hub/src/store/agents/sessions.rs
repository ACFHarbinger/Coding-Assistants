use super::super::*;

impl HubStore {
    /// Register a Hub-owned harness session with an explicit readiness
    /// state and optional pid. Use this when liveness is not a pid (Claude
    /// Channel keys off `is_channel_session_live`) so the row is not
    /// unconditionally stamped `ready`.
    pub fn register_managed_harness_session_with_state(
        &self,
        harness: &str,
        workspace: &str,
        disk_session_id: &str,
        managed_pid: Option<u32>,
        state: HarnessSessionState,
    ) -> Result<HarnessSessionRegistration, HubError> {
        let mut registration =
            self.register_harness_session(harness, workspace, disk_session_id, None)?;
        self.conn.execute(
            "UPDATE harness_session_registrations
             SET mode = 'managed', state = ?4, managed_pid = ?3
             WHERE harness = ?1 AND workspace = ?2",
            params![
                harness.trim(),
                workspace.trim(),
                managed_pid,
                state.as_str()
            ],
        )?;
        registration.mode = HarnessSessionMode::Managed;
        registration.state = state;
        registration.managed_pid = managed_pid;
        Ok(registration)
    }
}

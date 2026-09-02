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

    /// Clear the managed pid and writer lease and stamp `stopped`.
    /// No-op (returns `None`) when no row exists for this pair.
    pub fn mark_harness_session_stopped(
        &self,
        harness: &str,
        workspace: &str,
    ) -> Result<Option<HarnessSessionRegistration>, HubError> {
        let changed = self.conn.execute(
            "UPDATE harness_session_registrations
             SET state = 'stopped', managed_pid = NULL,
                 writer_owner = NULL, writer_acquired_at = NULL
             WHERE harness = ?1 AND workspace = ?2",
            params![harness.trim(), workspace.trim()],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        self.get_harness_session(harness, workspace)
    }

    /// Record completion of the exact worker process the Hub launched. The
    /// worker pid is not the provider session's liveness: a successful
    /// one-shot worker can finish while its managed provider session remains
    /// available for the next task. The pid guard prevents an older reaper
    /// from clearing a newer Start-managed run that replaced it.
    pub fn finish_managed_harness_process(
        &self,
        harness: &str,
        workspace: &str,
        pid: u32,
        succeeded: bool,
    ) -> Result<bool, HubError> {
        let state = if succeeded { "queued" } else { "unavailable" };
        let changed = self.conn.execute(
            "UPDATE harness_session_registrations
             SET state = ?4, managed_pid = NULL,
                 writer_owner = NULL, writer_acquired_at = NULL
             WHERE harness = ?1 AND workspace = ?2
               AND mode = 'managed' AND managed_pid = ?3",
            params![harness.trim(), workspace.trim(), pid, state],
        )?;
        Ok(changed == 1)
    }
}

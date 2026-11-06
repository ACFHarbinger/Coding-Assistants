use super::super::*;

/// Reconcile managed process IDs against currently running local processes via
/// `hub::proc::list_process_lines()` (#288 cross-platform process discovery).
/// Sets `pid_alive = Some(true)` if the pid is in the process table,
/// `Some(false)` if absent, or `None` if the process table cannot be read.
pub(crate) fn reconcile_pids(sessions: &mut [HarnessSessionRegistration]) {
    let has_managed_pid = sessions.iter().any(|s| s.managed_pid.is_some());
    if !has_managed_pid {
        return;
    }
    match crate::proc::list_process_lines() {
        Ok(lines) => {
            let active_pids: std::collections::HashSet<u32> =
                lines.into_iter().map(|(pid, _)| pid).collect();
            for session in sessions {
                if let Some(pid) = session.managed_pid {
                    session.pid_alive = Some(active_pids.contains(&pid));
                } else {
                    session.pid_alive = None;
                }
            }
        }
        Err(_) => {
            for session in sessions {
                if session.managed_pid.is_some() {
                    session.pid_alive = None;
                }
            }
        }
    }
}

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
        reconcile_pids(std::slice::from_mut(&mut registration));
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

    /// Update the provider chat/thread id on a managed row without resetting
    /// mode, writer lease, or managed_pid. Used after a one-shot worker exits.
    pub fn update_managed_harness_disk_session_id(
        &self,
        harness: &str,
        workspace: &str,
        disk_session_id: &str,
    ) -> Result<(), HubError> {
        let disk_session_id = disk_session_id.trim();
        if disk_session_id.is_empty() {
            return Err(HubError::Invalid(
                "managed harness disk session id must not be empty".into(),
            ));
        }
        let changed = self.conn.execute(
            "UPDATE harness_session_registrations
             SET disk_session_id = ?3, managed_pid = NULL
             WHERE harness = ?1 AND workspace = ?2 AND mode = 'managed'",
            params![harness.trim(), workspace.trim(), disk_session_id],
        )?;
        if changed == 0 {
            return Err(HubError::NotFound(format!(
                "managed {harness} harness session at {workspace}"
            )));
        }
        Ok(())
    }

    pub fn get_harness_session(
        &self,
        harness: &str,
        workspace: &str,
    ) -> Result<Option<HarnessSessionRegistration>, HubError> {
        let mut session = self
            .conn
            .query_row(
                "SELECT harness, workspace, disk_session_id, leader_socket, registered_at,
                        mode, state, managed_pid, writer_owner, writer_acquired_at
                 FROM harness_session_registrations
                 WHERE harness = ?1 AND workspace = ?2",
                params![harness, workspace],
                |row| {
                    Ok(HarnessSessionRegistration {
                        harness: row.get(0)?,
                        workspace: row.get(1)?,
                        disk_session_id: row.get(2)?,
                        leader_socket: row.get(3)?,
                        registered_at: row.get(4)?,
                        mode: HarnessSessionMode::parse(&row.get::<_, String>(5)?).map_err(
                            |error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)),
                        )?,
                        state: HarnessSessionState::parse(&row.get::<_, String>(6)?).map_err(
                            |error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)),
                        )?,
                        managed_pid: row.get(7)?,
                        writer_owner: row.get(8)?,
                        writer_acquired_at: row.get(9)?,
                        pid_alive: None,
                    })
                },
            )
            .optional()
            .map_err(HubError::from)?;
        if let Some(ref mut s) = session {
            reconcile_pids(std::slice::from_mut(s));
        }
        Ok(session)
    }

    pub fn list_harness_sessions(&self) -> Result<Vec<HarnessSessionRegistration>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT harness, workspace, disk_session_id, leader_socket, registered_at,
                    mode, state, managed_pid, writer_owner, writer_acquired_at
             FROM harness_session_registrations
             ORDER BY registered_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HarnessSessionRegistration {
                harness: row.get(0)?,
                workspace: row.get(1)?,
                disk_session_id: row.get(2)?,
                leader_socket: row.get(3)?,
                registered_at: row.get(4)?,
                mode: HarnessSessionMode::parse(&row.get::<_, String>(5)?)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                state: HarnessSessionState::parse(&row.get::<_, String>(6)?)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                managed_pid: row.get(7)?,
                writer_owner: row.get(8)?,
                writer_acquired_at: row.get(9)?,
                pid_alive: None,
            })
        })?;
        let mut sessions = rows.collect::<Result<Vec<_>, _>>()?;
        reconcile_pids(&mut sessions);
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn reconcile_pids_detects_live_and_dead_processes() {
        let current_pid = std::process::id();
        let dead_pid = 9_999_999;
        let mut sessions = vec![
            HarnessSessionRegistration {
                harness: "gemini".into(),
                workspace: "/abs/repo1".into(),
                disk_session_id: "s1".into(),
                leader_socket: None,
                registered_at: "now".into(),
                mode: HarnessSessionMode::Managed,
                state: HarnessSessionState::Ready,
                managed_pid: Some(current_pid),
                writer_owner: None,
                writer_acquired_at: None,
                pid_alive: None,
            },
            HarnessSessionRegistration {
                harness: "muse".into(),
                workspace: "/abs/repo2".into(),
                disk_session_id: "s2".into(),
                leader_socket: None,
                registered_at: "now".into(),
                mode: HarnessSessionMode::Managed,
                state: HarnessSessionState::Ready,
                managed_pid: Some(dead_pid),
                writer_owner: None,
                writer_acquired_at: None,
                pid_alive: None,
            },
            HarnessSessionRegistration {
                harness: "chat".into(),
                workspace: "/abs/repo3".into(),
                disk_session_id: "s3".into(),
                leader_socket: None,
                registered_at: "now".into(),
                mode: HarnessSessionMode::Observed,
                state: HarnessSessionState::Ready,
                managed_pid: None,
                writer_owner: None,
                writer_acquired_at: None,
                pid_alive: None,
            },
        ];

        reconcile_pids(&mut sessions);

        assert_eq!(sessions[0].pid_alive, Some(true));
        assert_eq!(sessions[1].pid_alive, Some(false));
        assert_eq!(sessions[2].pid_alive, None);
    }

    #[test]
    fn list_and_get_harness_sessions_surface_reconciled_pid_and_writer_lease() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = "/tmp/ca-reconcile-ws";
        let current_pid = std::process::id();

        let reg = store
            .register_managed_harness_session("cursor", workspace, "thread-live", current_pid)
            .unwrap();
        assert_eq!(reg.pid_alive, Some(true));
        assert_eq!(reg.mode, HarnessSessionMode::Managed);

        store
            .acquire_harness_writer("cursor", workspace, "worker-1")
            .unwrap();

        let fetched = store
            .get_harness_session("cursor", workspace)
            .unwrap()
            .expect("session found");
        assert_eq!(fetched.pid_alive, Some(true));
        assert_eq!(fetched.writer_owner.as_deref(), Some("worker-1"));
        assert!(fetched.writer_acquired_at.is_some());
        assert_eq!(fetched.state, HarnessSessionState::Busy);

        let other_workspace = "/tmp/ca-dead-ws";
        store
            .register_managed_harness_session("muse", other_workspace, "thread-dead", 9_999_999)
            .unwrap();

        let all = store.list_harness_sessions().unwrap();
        let cursor_row = all.iter().find(|s| s.harness == "cursor").unwrap();
        assert_eq!(cursor_row.pid_alive, Some(true));
        assert_eq!(cursor_row.writer_owner.as_deref(), Some("worker-1"));
        assert!(cursor_row.writer_acquired_at.is_some());
        assert_eq!(cursor_row.state, HarnessSessionState::Busy);

        let muse_row = all.iter().find(|s| s.harness == "muse").unwrap();
        assert_eq!(muse_row.pid_alive, Some(false));
        assert_eq!(muse_row.managed_pid, Some(9_999_999));
        assert!(muse_row.writer_owner.is_none());
    }
}

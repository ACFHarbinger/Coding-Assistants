use super::*;

mod avatar;
mod capture;
mod sessions;
mod team;
mod work_sessions;
impl HubStore {
    pub fn upsert_agent(&self, id: &str, display_name: &str) -> Result<(), HubError> {
        self.conn.execute(
            "INSERT INTO agents (id, display_name, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET display_name = ?2",
            params![id, display_name, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn upsert_agent_card(&self, id: &str, card: &AgentCard) -> Result<(), HubError> {
        let card_json =
            serde_json::to_string(card).map_err(|e| HubError::Invalid(e.to_string()))?;
        self.conn.execute(
            "INSERT INTO agents (id, display_name, created_at, card_json) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET display_name = ?2, card_json = ?4",
            params![id, card.name, Utc::now().to_rfc3339(), card_json],
        )?;
        Ok(())
    }

    pub fn list_agents(&self) -> Result<Vec<AgentRecord>, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, display_name, created_at, card_json, team_member, avatar_attachment_id
             FROM agents ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AgentRecord {
                id: r.get(0)?,
                display_name: r.get(1)?,
                created_at: r.get(2)?,
                card_json: r.get(3)?,
                team_member: r.get::<_, i64>(4)? != 0,
                avatar_attachment_id: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_team_members(&self) -> Result<Vec<AgentRecord>, HubError> {
        Ok(self
            .list_agents()?
            .into_iter()
            .filter(|agent| agent.team_member)
            .collect())
    }

    pub fn set_team_member(&self, id: &str, enrolled: bool) -> Result<AgentRecord, HubError> {
        let updated = self.conn.execute(
            "UPDATE agents SET team_member = ?1 WHERE id = ?2",
            params![if enrolled { 1 } else { 0 }, id],
        )?;
        if updated == 0 {
            return Err(HubError::NotFound(id.to_string()));
        }
        self.list_agents()?
            .into_iter()
            .find(|agent| agent.id == id)
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }

    pub fn list_work_sessions(&self) -> Result<Vec<WorkSessionRecord>, HubError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, created_at FROM work_sessions ORDER BY created_at DESC")?;
        let sessions = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        sessions
            .into_iter()
            .map(|(id, name, created_at)| self.work_session_record(id, name, created_at))
            .collect()
    }

    const BUILTIN_CHANNELS: &[(&str, &str)] = &[
        ("general", "Team-wide coordination and announcement hub"),
        (
            "team-coordination",
            "Inter-agent task claims, handoffs, and bus updates",
        ),
        (
            "agent-memory",
            "Shared memory insights, context tags, and audit events",
        ),
        (
            "wakes-alerts",
            "System wake requests and human approval gates",
        ),
    ];

    pub(super) fn seed_default_channels(&self) -> Result<(), HubError> {
        let now = Utc::now().to_rfc3339();
        for (id, topic) in Self::BUILTIN_CHANNELS {
            self.conn.execute(
                "INSERT OR IGNORE INTO chat_channels(id, name, topic, builtin, created_at, deleted_at)
                 VALUES (?1, ?2, ?3, 1, ?4, NULL)",
                params![id, format!("#{id}"), topic, now],
            )?;
        }
        Ok(())
    }

    pub fn list_channels(&self) -> Result<Vec<ChannelRecord>, HubError> {
        self.seed_default_channels()?;
        let mut stmt = self.conn.prepare(
            "SELECT id, name, topic, builtin, created_at FROM chat_channels
             WHERE deleted_at IS NULL
             ORDER BY builtin DESC, created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ChannelRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                topic: row.get(2)?,
                builtin: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn create_channel(
        &self,
        name: &str,
        topic: Option<&str>,
    ) -> Result<ChannelRecord, HubError> {
        let id = slug_channel_id(name)?;
        if Self::BUILTIN_CHANNELS
            .iter()
            .any(|(builtin, _)| *builtin == id)
        {
            return Err(HubError::Invalid(format!(
                "#{id} is a built-in channel and already exists"
            )));
        }
        if id.starts_with("session-") || id.starts_with("dm-") {
            return Err(HubError::Invalid(
                "channel names cannot start with session or dm".into(),
            ));
        }
        let existing: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT deleted_at FROM chat_channels WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        let now = Utc::now().to_rfc3339();
        let display = format!("#{id}");
        let topic = topic.map(str::trim).filter(|value| !value.is_empty());
        match existing {
            Some(None) => {
                return Err(HubError::Invalid(format!("#{id} already exists")));
            }
            Some(Some(_)) => {
                self.conn.execute(
                    "UPDATE chat_channels SET name = ?1, topic = ?2, deleted_at = NULL, created_at = ?3
                     WHERE id = ?4",
                    params![display, topic, now, id],
                )?;
            }
            None => {
                self.conn.execute(
                    "INSERT INTO chat_channels(id, name, topic, builtin, created_at, deleted_at)
                     VALUES (?1, ?2, ?3, 0, ?4, NULL)",
                    params![id, display, topic, now],
                )?;
            }
        }
        self.list_channels()?
            .into_iter()
            .find(|channel| channel.id == id)
            .ok_or_else(|| HubError::NotFound(id))
    }

    pub fn register_harness_session(
        &self,
        harness: &str,
        workspace: &str,
        disk_session_id: &str,
        leader_socket: Option<&str>,
    ) -> Result<HarnessSessionRegistration, HubError> {
        let harness = harness.trim();
        let workspace = workspace.trim();
        let disk_session_id = disk_session_id.trim();
        if harness.is_empty() || workspace.is_empty() || disk_session_id.is_empty() {
            return Err(HubError::Invalid(
                "harness session registration requires harness, absolute workspace, and disk session id"
                    .into(),
            ));
        }
        if !Path::new(workspace).is_absolute() {
            return Err(HubError::Invalid(
                "harness session workspace must be an absolute path".into(),
            ));
        }
        let registered_at = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO harness_session_registrations(
                harness, workspace, disk_session_id, leader_socket, registered_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(harness, workspace) DO UPDATE SET
                disk_session_id = excluded.disk_session_id,
                leader_socket = excluded.leader_socket,
                registered_at = excluded.registered_at,
                mode = 'observed',
                state = 'ready',
                managed_pid = NULL,
                writer_owner = NULL,
                writer_acquired_at = NULL",
            params![
                harness,
                workspace,
                disk_session_id,
                leader_socket,
                registered_at
            ],
        )?;
        Ok(HarnessSessionRegistration {
            harness: harness.into(),
            workspace: workspace.into(),
            disk_session_id: disk_session_id.into(),
            leader_socket: leader_socket.map(str::to_string),
            registered_at,
            mode: HarnessSessionMode::Observed,
            state: HarnessSessionState::Ready,
            managed_pid: None,
            writer_owner: None,
            writer_acquired_at: None,
        })
    }

    /// Register a process/session deliberately launched and owned by the Hub.
    /// Ownership is explicit; discovery must continue to use
    /// [`register_harness_session`] and therefore stays observed.
    pub fn register_managed_harness_session(
        &self,
        harness: &str,
        workspace: &str,
        disk_session_id: &str,
        managed_pid: u32,
    ) -> Result<HarnessSessionRegistration, HubError> {
        let mut registration =
            self.register_harness_session(harness, workspace, disk_session_id, None)?;
        self.conn.execute(
            "UPDATE harness_session_registrations
             SET mode = 'managed', state = 'ready', managed_pid = ?3
             WHERE harness = ?1 AND workspace = ?2",
            params![harness.trim(), workspace.trim(), managed_pid],
        )?;
        registration.mode = HarnessSessionMode::Managed;
        registration.managed_pid = Some(managed_pid);
        Ok(registration)
    }

    /// Acquire the only writer lease for a managed provider session.
    /// A live lease is never stolen: callers must surface it as busy/queued or
    /// release it after completion/cancellation.
    pub fn acquire_harness_writer(
        &self,
        harness: &str,
        workspace: &str,
        owner: &str,
    ) -> Result<(), HubError> {
        let owner = owner.trim();
        if owner.is_empty() {
            return Err(HubError::Invalid(
                "harness writer owner must not be empty".into(),
            ));
        }
        let changed = self.conn.execute(
            "UPDATE harness_session_registrations
             SET writer_owner = ?3, writer_acquired_at = ?4, state = 'busy'
             WHERE harness = ?1 AND workspace = ?2
               AND mode = 'managed' AND writer_owner IS NULL",
            params![harness, workspace, owner, Utc::now().to_rfc3339()],
        )?;
        if changed == 1 {
            return Ok(());
        }
        let registration = self.get_harness_session(harness, workspace)?;
        match registration {
            None => Err(HubError::NotFound(format!(
                "{harness} harness session at {workspace}"
            ))),
            Some(session) if session.mode != HarnessSessionMode::Managed => Err(HubError::Invalid(
                "cannot acquire a writer for an observed harness session".into(),
            )),
            Some(session) => Err(HubError::Invalid(format!(
                "harness session already has an active writer{}",
                session
                    .writer_owner
                    .as_deref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            ))),
        }
    }

    pub fn release_harness_writer(
        &self,
        harness: &str,
        workspace: &str,
        owner: &str,
        next_state: HarnessSessionState,
    ) -> Result<(), HubError> {
        let changed = self.conn.execute(
            "UPDATE harness_session_registrations
             SET writer_owner = NULL, writer_acquired_at = NULL, state = ?4
             WHERE harness = ?1 AND workspace = ?2 AND writer_owner = ?3",
            params![harness, workspace, owner.trim(), next_state.as_str()],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(HubError::Invalid(
                "harness writer lease is not held by this owner".into(),
            ))
        }
    }

    pub fn get_harness_session(
        &self,
        harness: &str,
        workspace: &str,
    ) -> Result<Option<HarnessSessionRegistration>, HubError> {
        self.conn
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
                    })
                },
            )
            .optional()
            .map_err(HubError::from)
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn delete_channel(&self, id: &str) -> Result<(), HubError> {
        let id = slug_channel_id(id)?;
        if Self::BUILTIN_CHANNELS
            .iter()
            .any(|(builtin, _)| *builtin == id)
        {
            return Err(HubError::Invalid(
                "built-in channels cannot be deleted".into(),
            ));
        }
        let updated = self.conn.execute(
            "UPDATE chat_channels SET deleted_at = ?1 WHERE id = ?2 AND builtin = 0 AND deleted_at IS NULL",
            params![Utc::now().to_rfc3339(), id],
        )?;
        if updated == 0 {
            return Err(HubError::NotFound(id));
        }
        Ok(())
    }

    pub fn add_work_session_member(
        &self,
        session_id: &str,
        agent_id: &str,
    ) -> Result<WorkSessionRecord, HubError> {
        if self.get_work_session(session_id)?.is_none() {
            return Err(HubError::NotFound(session_id.to_string()));
        }
        if !self.list_agents()?.iter().any(|agent| agent.id == agent_id) {
            return Err(HubError::NotFound(agent_id.to_string()));
        }
        self.conn.execute(
            "INSERT OR IGNORE INTO work_session_members(session_id, agent_id, created_at)
             VALUES (?1, ?2, ?3)",
            params![session_id, agent_id, Utc::now().to_rfc3339()],
        )?;
        self.get_work_session(session_id)?
            .ok_or_else(|| HubError::NotFound(session_id.to_string()))
    }

    pub(super) fn get_work_session(&self, id: &str) -> Result<Option<WorkSessionRecord>, HubError> {
        self.conn
            .query_row(
                "SELECT id, name, created_at FROM work_sessions WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .map(|(id, name, created_at)| self.work_session_record(id, name, created_at))
            .transpose()
    }

    fn work_session_record(
        &self,
        id: String,
        name: String,
        created_at: String,
    ) -> Result<WorkSessionRecord, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT agent_id FROM work_session_members WHERE session_id = ?1 ORDER BY agent_id",
        )?;
        let member_ids = stmt
            .query_map(params![id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WorkSessionRecord {
            id,
            name,
            created_at,
            member_ids,
        })
    }
}

//! Plain CRUD for `ScheduledTask` (S1). Dispatch/gate/tick logic is S2/S3.

use super::super::*;
use super::time::{instant_le, normalize_rfc3339, now_utc_rfc3339};
use super::types::{
    is_schedule_eligible_agent, BudgetPolicy, GateCombinator, GateCriterion, ScheduleStatus,
    ScheduleTarget, ScheduledTask,
};

struct ScheduleRow {
    id: String,
    name: String,
    instruction: String,
    target_json: String,
    workspace: String,
    fire_at: String,
    repeat_interval_secs: Option<i64>,
    response_timeout_secs: i64,
    gate_criteria_json: String,
    gate_combinator: String,
    budget_json: String,
    max_attempts: u32,
    deadline: Option<String>,
    status: String,
    created_by: String,
    created_at: String,
    updated_at: String,
    last_fired_at: Option<String>,
    next_fire_at: Option<String>,
    token_baseline_json: String,
    attempts: u32,
}

fn map_raw(r: &rusqlite::Row<'_>) -> Result<ScheduleRow, rusqlite::Error> {
    Ok(ScheduleRow {
        id: r.get(0)?,
        name: r.get(1)?,
        instruction: r.get(2)?,
        target_json: r.get(3)?,
        workspace: r.get(4)?,
        fire_at: r.get(5)?,
        repeat_interval_secs: r.get(6)?,
        response_timeout_secs: r.get::<_, i64>(7).unwrap_or(3600),
        gate_criteria_json: r.get(8)?,
        gate_combinator: r.get(9)?,
        budget_json: r.get(10)?,
        max_attempts: r.get::<_, i64>(11)? as u32,
        deadline: r.get(12)?,
        status: r.get(13)?,
        created_by: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
        last_fired_at: r.get(17)?,
        next_fire_at: r.get(18)?,
        token_baseline_json: r.get(19)?,
        attempts: r.get::<_, i64>(20)? as u32,
    })
}

fn decode_json<T: serde::de::DeserializeOwned>(label: &str, raw: &str) -> Result<T, HubError> {
    serde_json::from_str(raw)
        .map_err(|error| HubError::Invalid(format!("scheduled task {label} is corrupt: {error}")))
}

fn encode_json<T: serde::Serialize>(label: &str, value: &T) -> Result<String, HubError> {
    serde_json::to_string(value)
        .map_err(|error| HubError::Invalid(format!("schedule {label} encode: {error}")))
}

fn decode_schedule(row: &ScheduleRow) -> Result<ScheduledTask, HubError> {
    let status = ScheduleStatus::parse(&row.status).ok_or_else(|| {
        HubError::Invalid(format!("scheduled task status is corrupt: {}", row.status))
    })?;
    Ok(ScheduledTask {
        id: row.id.clone(),
        name: row.name.clone(),
        instruction: row.instruction.clone(),
        target: decode_json("target", &row.target_json)?,
        workspace: row.workspace.clone(),
        fire_at: row.fire_at.clone(),
        repeat_interval_secs: row.repeat_interval_secs,
        response_timeout_secs: row.response_timeout_secs,
        gate_criteria: decode_json("gate_criteria", &row.gate_criteria_json)?,
        gate_combinator: GateCombinator::parse_strict(&row.gate_combinator).ok_or_else(|| {
            HubError::Invalid(format!(
                "scheduled task gate_combinator is corrupt: {}",
                row.gate_combinator
            ))
        })?,
        budget: decode_json("budget", &row.budget_json)?,
        max_attempts: row.max_attempts,
        deadline: row.deadline.clone(),
        status,
        created_by: row.created_by.clone(),
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
        last_fired_at: row.last_fired_at.clone(),
        next_fire_at: row.next_fire_at.clone(),
        attempts: row.attempts,
        token_baseline: decode_json("token_baseline", &row.token_baseline_json)?,
    })
}

const SELECT_COLUMNS: &str = "id, name, instruction, target_json, workspace, fire_at,
    repeat_interval_secs, response_timeout_secs, gate_criteria_json, gate_combinator,
    budget_json, max_attempts, deadline, status, created_by, created_at, updated_at,
    last_fired_at, next_fire_at, token_baseline_json, attempts";

impl HubStore {
    /// Create a new scheduled task. `fire_at`/`deadline` are RFC3339 and
    /// stored as UTC. `max_attempts` is mandatory and must be >= 1.
    #[allow(clippy::too_many_arguments)]
    pub fn create_scheduled_task(
        &self,
        name: &str,
        instruction: &str,
        target: ScheduleTarget,
        workspace: &str,
        fire_at: &str,
        repeat_interval_secs: Option<i64>,
        response_timeout_secs: i64,
        gate_criteria: Vec<GateCriterion>,
        gate_combinator: GateCombinator,
        budget: BudgetPolicy,
        max_attempts: u32,
        deadline: Option<&str>,
        created_by: &str,
    ) -> Result<ScheduledTask, HubError> {
        self.ensure_mutable()?;
        if name.trim().is_empty() {
            return Err(HubError::Invalid("schedule name must not be empty".into()));
        }
        if instruction.trim().is_empty() {
            return Err(HubError::Invalid(
                "schedule instruction must not be empty".into(),
            ));
        }
        if !Path::new(workspace).is_absolute() {
            return Err(HubError::Invalid(
                "schedule workspace must be an absolute path".into(),
            ));
        }
        if let ScheduleTarget::Agent { agent_id } = &target {
            if agent_id.trim().is_empty() {
                return Err(HubError::Invalid(
                    "scheduled agent target must not be empty".into(),
                ));
            }
        }
        let fire_at = normalize_rfc3339("fire_at", fire_at)?;
        let deadline = deadline
            .map(|value| normalize_rfc3339("deadline", value))
            .transpose()?;
        if max_attempts == 0 {
            return Err(HubError::Invalid(
                "max_attempts must be at least 1 — it is a mandatory stop condition, \
                 not an optional one (see roadmaps/schedule.md)"
                    .into(),
            ));
        }
        if response_timeout_secs <= 0 {
            return Err(HubError::Invalid(
                "response_timeout_secs must be positive".into(),
            ));
        }
        if let Some(interval) = repeat_interval_secs {
            if interval <= 0 {
                return Err(HubError::Invalid(
                    "repeat_interval_secs must be positive when set".into(),
                ));
            }
        }
        for criterion in &gate_criteria {
            if let GateCriterion::Command { program, .. } = criterion {
                if program.trim().is_empty() {
                    return Err(HubError::Invalid(
                        "gate command program must not be empty".into(),
                    ));
                }
            }
        }

        let token_baseline = self.schedule_token_baseline(&target)?;
        let id = Uuid::new_v4().to_string();
        let now = now_utc_rfc3339();
        self.conn.execute(
            "INSERT INTO scheduled_tasks(
                id, name, instruction, target_json, workspace, fire_at,
                repeat_interval_secs, response_timeout_secs, gate_criteria_json,
                gate_combinator, budget_json, max_attempts, deadline, status,
                created_by, created_at, updated_at, last_fired_at, next_fire_at,
                token_baseline_json, attempts
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,NULL,?18,?19,0)",
            params![
                id,
                name,
                instruction,
                encode_json("target", &target)?,
                workspace,
                fire_at,
                repeat_interval_secs,
                response_timeout_secs,
                encode_json("gate_criteria", &gate_criteria)?,
                gate_combinator.as_str(),
                encode_json("budget", &budget)?,
                max_attempts,
                deadline,
                ScheduleStatus::Scheduled.as_str(),
                created_by,
                now,
                now,
                fire_at,
                encode_json("token_baseline", &token_baseline)?,
            ],
        )?;
        let task = self
            .get_scheduled_task(&id)?
            .ok_or_else(|| HubError::NotFound(id.clone()))?;
        let _ = self.record_audit_event(
            Path::new(workspace),
            Path::new(workspace),
            "schedule.created",
            &serde_json::json!({
                "schedule_id": id, "name": name, "max_attempts": max_attempts,
            })
            .to_string(),
            None,
        );
        Ok(task)
    }

    /// Snapshot measured `tokens_used` for eligible identities only.
    /// Missing metrics are omitted (unknown), never recorded as zero.
    fn schedule_token_baseline(
        &self,
        target: &ScheduleTarget,
    ) -> Result<std::collections::HashMap<String, i64>, HubError> {
        let metrics = self.list_agent_metrics()?;
        let mut baseline = std::collections::HashMap::new();
        for agent_id in self.schedule_target_agent_ids(target)? {
            if let Some(tokens) = metrics
                .iter()
                .find(|metric| metric.agent_id == agent_id)
                .map(|metric| metric.tokens_used)
            {
                baseline.insert(agent_id, tokens);
            }
        }
        Ok(baseline)
    }

    pub fn schedule_target_agent_ids(
        &self,
        target: &ScheduleTarget,
    ) -> Result<Vec<String>, HubError> {
        Ok(match target {
            ScheduleTarget::Agent { agent_id } => vec![agent_id.clone()],
            ScheduleTarget::Team => self
                .list_team_members()?
                .into_iter()
                .map(|agent| agent.id)
                .filter(|id| is_schedule_eligible_agent(id))
                .collect(),
        })
    }

    pub fn get_scheduled_task(&self, id: &str) -> Result<Option<ScheduledTask>, HubError> {
        let raw = self
            .conn
            .query_row(
                &format!("SELECT {SELECT_COLUMNS} FROM scheduled_tasks WHERE id = ?1"),
                params![id],
                map_raw,
            )
            .optional()?;
        match raw {
            None => Ok(None),
            Some(row) => match decode_schedule(&row) {
                Ok(task) => Ok(Some(task)),
                Err(error) => {
                    self.quarantine_scheduled_task(&row, &error.to_string());
                    Err(error)
                }
            },
        }
    }

    pub fn list_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>, HubError> {
        self.load_scheduled_rows(&format!(
            "SELECT {SELECT_COLUMNS} FROM scheduled_tasks ORDER BY created_at DESC"
        ))
    }

    /// Tasks due to fire/re-fire now, in due-first order. Instant compare,
    /// not lexical text — offsets and fractional precision are normalized.
    pub fn due_scheduled_tasks(&self, now: &str) -> Result<Vec<ScheduledTask>, HubError> {
        let now = normalize_rfc3339("now", now)?;
        let mut due = self.load_scheduled_rows(&format!(
            "SELECT {SELECT_COLUMNS} FROM scheduled_tasks
             WHERE status IN ('scheduled', 'active') AND next_fire_at IS NOT NULL"
        ))?;
        due.retain(|task| {
            task.next_fire_at
                .as_deref()
                .and_then(|fire| instant_le(fire, &now).ok())
                .unwrap_or(false)
        });
        due.sort_by(|a, b| a.next_fire_at.cmp(&b.next_fire_at));
        Ok(due)
    }

    pub fn cancel_scheduled_task(&self, id: &str) -> Result<ScheduledTask, HubError> {
        self.ensure_mutable()?;
        let task = self
            .get_scheduled_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        if task.status.is_terminal() {
            return Ok(task);
        }
        // Paused may be cancelled; ordinary tick transitions must not.
        let n = self.conn.execute(
            "UPDATE scheduled_tasks SET status = ?1, next_fire_at = NULL, updated_at = ?2
             WHERE id = ?3 AND status IN ('scheduled', 'active', 'paused')",
            params![ScheduleStatus::Cancelled.as_str(), now_utc_rfc3339(), id],
        )?;
        if n == 0 {
            return self
                .get_scheduled_task(id)?
                .ok_or_else(|| HubError::NotFound(id.into()));
        }
        let _ = self.record_audit_event(
            Path::new(&task.workspace),
            Path::new(&task.workspace),
            "schedule.cancelled",
            &serde_json::json!({ "schedule_id": id }).to_string(),
            None,
        );
        self.get_scheduled_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    pub(crate) fn transition_scheduled_task(
        &self,
        id: &str,
        status: ScheduleStatus,
        next_fire_at: Option<&str>,
    ) -> Result<bool, HubError> {
        self.transition_live_schedule(id, status, next_fire_at, None)
    }

    /// Tick/pause CAS: never matches `paused`, so a stale active snapshot
    /// cannot resurrect a quota-paused row. Optional `expected_attempts`
    /// rejects a snapshot that already lost a later claim.
    pub(crate) fn transition_live_schedule(
        &self,
        id: &str,
        status: ScheduleStatus,
        next_fire_at: Option<&str>,
        expected_attempts: Option<u32>,
    ) -> Result<bool, HubError> {
        self.ensure_mutable()?;
        let n = match expected_attempts {
            Some(attempts) => self.conn.execute(
                "UPDATE scheduled_tasks SET status = ?1, next_fire_at = ?2, updated_at = ?3
                 WHERE id = ?4 AND status IN ('scheduled', 'active') AND attempts = ?5",
                params![
                    status.as_str(),
                    next_fire_at,
                    now_utc_rfc3339(),
                    id,
                    attempts as i64
                ],
            )?,
            None => self.conn.execute(
                "UPDATE scheduled_tasks SET status = ?1, next_fire_at = ?2, updated_at = ?3
                 WHERE id = ?4 AND status IN ('scheduled', 'active')",
                params![status.as_str(), next_fire_at, now_utc_rfc3339(), id],
            )?,
        };
        Ok(n > 0)
    }

    fn load_scheduled_rows(&self, sql: &str) -> Result<Vec<ScheduledTask>, HubError> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], map_raw)?;
        let mut out = Vec::new();
        for row in rows {
            let raw = row?;
            match decode_schedule(&raw) {
                Ok(task) => out.push(task),
                Err(error) => self.quarantine_scheduled_task(&raw, &error.to_string()),
            }
        }
        Ok(out)
    }

    fn quarantine_scheduled_task(&self, row: &ScheduleRow, reason: &str) {
        let _ = self.conn.execute(
            "UPDATE scheduled_tasks SET status = 'failed', next_fire_at = NULL, updated_at = ?1
             WHERE id = ?2 AND status IN ('scheduled', 'active')",
            params![now_utc_rfc3339(), row.id],
        );
        let _ = self.record_audit_event(
            Path::new(&row.workspace),
            Path::new(&row.workspace),
            "schedule.quarantined",
            &serde_json::json!({ "schedule_id": row.id, "reason": reason }).to_string(),
            None,
        );
    }
}

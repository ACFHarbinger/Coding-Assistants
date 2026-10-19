//! Plain CRUD for `ScheduledTask` (S1). Dispatch/gate/tick logic is S2/S3.

use super::super::*;
use super::types::{
    BudgetPolicy, GateCombinator, GateCriterion, ScheduleStatus, ScheduleTarget, ScheduledTask,
};

fn parse_rfc3339(label: &str, value: &str) -> Result<(), HubError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|_| ())
        .map_err(|_| HubError::Invalid(format!("{label} must be an RFC3339 timestamp")))
}

fn map_row(r: &rusqlite::Row<'_>) -> Result<ScheduledTask, rusqlite::Error> {
    let target_json: String = r.get(3)?;
    let gate_criteria_json: String = r.get(8)?;
    let gate_combinator: String = r.get(9)?;
    let budget_json: String = r.get(10)?;
    let token_baseline_json: String = r.get(19)?;
    Ok(ScheduledTask {
        id: r.get(0)?,
        name: r.get(1)?,
        instruction: r.get(2)?,
        target: serde_json::from_str::<ScheduleTarget>(&target_json)
            .unwrap_or(ScheduleTarget::Team),
        workspace: r.get(4)?,
        fire_at: r.get(5)?,
        repeat_interval_secs: r.get(6)?,
        response_timeout_secs: r.get::<_, i64>(7).unwrap_or(3600),
        gate_criteria: serde_json::from_str::<Vec<GateCriterion>>(&gate_criteria_json)
            .unwrap_or_default(),
        gate_combinator: GateCombinator::parse(&gate_combinator),
        budget: serde_json::from_str::<BudgetPolicy>(&budget_json).unwrap_or_default(),
        max_attempts: r.get::<_, i64>(11)? as u32,
        deadline: r.get(12)?,
        status: ScheduleStatus::parse(&r.get::<_, String>(13)?).unwrap_or(ScheduleStatus::Failed),
        created_by: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
        last_fired_at: r.get(17)?,
        next_fire_at: r.get(18)?,
        attempts: r.get::<_, i64>(20)? as u32,
        token_baseline: serde_json::from_str(&token_baseline_json).unwrap_or_default(),
    })
}

const SELECT_COLUMNS: &str = "id, name, instruction, target_json, workspace, fire_at,
    repeat_interval_secs, response_timeout_secs, gate_criteria_json, gate_combinator,
    budget_json, max_attempts, deadline, status, created_by, created_at, updated_at,
    last_fired_at, next_fire_at, token_baseline_json, attempts";

impl HubStore {
    /// Create a new scheduled task. `fire_at`/`deadline` are RFC3339.
    /// `max_attempts` is mandatory (see module docs on why) — must be >= 1.
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
        parse_rfc3339("fire_at", fire_at)?;
        if let Some(d) = deadline {
            parse_rfc3339("deadline", d)?;
        }
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
        let now = Utc::now().to_rfc3339();
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
                serde_json::to_string(&target).unwrap_or_default(),
                workspace,
                fire_at,
                repeat_interval_secs,
                response_timeout_secs,
                serde_json::to_string(&gate_criteria).unwrap_or_else(|_| "[]".into()),
                gate_combinator.as_str(),
                serde_json::to_string(&budget).unwrap_or_else(|_| "{}".into()),
                max_attempts,
                deadline,
                ScheduleStatus::Scheduled.as_str(),
                created_by,
                now,
                now,
                fire_at,
                serde_json::to_string(&token_baseline).unwrap_or_else(|_| "{}".into()),
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

    /// Snapshot each in-scope agent's current `agent_metrics.tokens_used`
    /// so `MinTokensWritten` measures a delta from schedule-creation time.
    fn schedule_token_baseline(
        &self,
        target: &ScheduleTarget,
    ) -> Result<std::collections::HashMap<String, i64>, HubError> {
        let metrics = self.list_agent_metrics()?;
        let agents: Vec<String> = match target {
            ScheduleTarget::Agent { agent_id } => vec![agent_id.clone()],
            ScheduleTarget::Team => self
                .list_team_members()?
                .into_iter()
                .map(|a| a.id)
                .collect(),
        };
        Ok(agents
            .into_iter()
            .map(|agent_id| {
                let tokens = metrics
                    .iter()
                    .find(|m| m.agent_id == agent_id)
                    .map(|m| m.tokens_used)
                    .unwrap_or(0);
                (agent_id, tokens)
            })
            .collect())
    }

    pub fn get_scheduled_task(&self, id: &str) -> Result<Option<ScheduledTask>, HubError> {
        self.conn
            .query_row(
                &format!("SELECT {SELECT_COLUMNS} FROM scheduled_tasks WHERE id = ?1"),
                params![id],
                map_row,
            )
            .optional()
            .map_err(HubError::from)
    }

    pub fn list_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>, HubError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM scheduled_tasks ORDER BY created_at DESC"
        ))?;
        let rows = stmt.query_map([], map_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(HubError::from)
    }

    /// Tasks due to fire/re-fire now, in due-first order. Used by the S3
    /// tick and S4's `ca schedule tick`.
    pub fn due_scheduled_tasks(&self, now: &str) -> Result<Vec<ScheduledTask>, HubError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {SELECT_COLUMNS} FROM scheduled_tasks
             WHERE status IN ('scheduled', 'active') AND next_fire_at IS NOT NULL
               AND next_fire_at <= ?1
             ORDER BY next_fire_at ASC"
        ))?;
        let rows = stmt.query_map(params![now], map_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(HubError::from)
    }

    pub fn cancel_scheduled_task(&self, id: &str) -> Result<ScheduledTask, HubError> {
        self.ensure_mutable()?;
        let task = self
            .get_scheduled_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))?;
        if task.status.is_terminal() {
            return Ok(task);
        }
        self.set_scheduled_task_status(id, ScheduleStatus::Cancelled, None)?;
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

    pub(crate) fn set_scheduled_task_status(
        &self,
        id: &str,
        status: ScheduleStatus,
        next_fire_at: Option<&str>,
    ) -> Result<(), HubError> {
        self.ensure_mutable()?;
        self.conn.execute(
            "UPDATE scheduled_tasks SET status = ?1, next_fire_at = ?2, updated_at = ?3
             WHERE id = ?4",
            params![status.as_str(), next_fire_at, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }
}

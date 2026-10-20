//! Guardrail rule storage and screening (P16 / #360). The primitive itself
//! lives in `crate::guardrails`; this module persists rules and runs them
//! with an audit trail. See that module's docs for the halt semantics.

use super::*;
use crate::guardrails::{
    run_rules, GuardrailMode, GuardrailRule, GuardrailSpec, GuardrailStage, GuardrailTrip,
    GuardrailVerdict,
};

const SELECT: &str =
    "SELECT id, name, workspace_path, stage, mode, spec_json, enabled, created_at FROM guardrail_rules";

fn map_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<GuardrailRule> {
    let stage: String = row.get(3)?;
    let mode: String = row.get(4)?;
    let spec_json: String = row.get(5)?;
    let spec: GuardrailSpec = serde_json::from_str(&spec_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(GuardrailRule {
        id: row.get(0)?,
        name: row.get(1)?,
        workspace: row.get(2)?,
        stage: GuardrailStage::parse(&stage).unwrap_or(GuardrailStage::Input),
        mode: GuardrailMode::parse(&mode).unwrap_or_default(),
        spec,
        enabled: row.get::<_, i64>(6)? > 0,
        created_at: row.get(7)?,
    })
}

pub(crate) fn trip_message(stage: GuardrailStage, trip: &GuardrailTrip) -> String {
    format!(
        "{} guardrail '{}' tripped: {}",
        stage.as_str(),
        trip.name,
        trip.detail
    )
}

fn normalized_workspace(workspace: Option<&str>) -> Option<String> {
    workspace
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_string)
}

impl HubStore {
    pub fn ensure_guardrail_table(&self) -> Result<(), HubError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS guardrail_rules (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                workspace_path TEXT,
                stage TEXT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'blocking',
                spec_json TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_guardrail_rules_scope
                ON guardrail_rules(stage, workspace_path, created_at);
            "#,
        )?;
        Ok(())
    }

    /// Store a rule. `workspace = None` applies hub-wide; otherwise only to
    /// dispatches/replies carrying that workspace.
    pub fn add_guardrail(
        &self,
        name: &str,
        workspace: Option<&str>,
        stage: GuardrailStage,
        mode: GuardrailMode,
        spec: GuardrailSpec,
    ) -> Result<GuardrailRule, HubError> {
        self.ensure_mutable()?;
        let name = name.trim();
        if name.is_empty() {
            return Err(HubError::Invalid("guardrail name must not be empty".into()));
        }
        spec.validate().map_err(HubError::Invalid)?;
        let workspace = normalized_workspace(workspace);
        if let Some(ws) = workspace.as_deref() {
            if !Path::new(ws).is_absolute() {
                return Err(HubError::Invalid(
                    "guardrail workspace must be an absolute path".into(),
                ));
            }
        }
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let spec_json = serde_json::to_string(&spec)
            .map_err(|error| HubError::Invalid(format!("guardrail spec serialize: {error}")))?;
        self.conn.execute(
            "INSERT INTO guardrail_rules(id, name, workspace_path, stage, mode, spec_json, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)",
            params![
                id,
                name,
                workspace,
                stage.as_str(),
                mode.as_str(),
                spec_json,
                created_at
            ],
        )?;
        self.get_guardrail(&id)?
            .ok_or_else(|| HubError::NotFound(id))
    }

    pub fn get_guardrail(&self, id: &str) -> Result<Option<GuardrailRule>, HubError> {
        Ok(self
            .conn
            .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], map_rule)
            .optional()?)
    }

    /// `None` lists every rule (CLI inventory); `Some(ws)` lists hub-wide
    /// rules plus that workspace's, in creation order. Screening uses
    /// [`Self::effective_guardrails`], which never applies another
    /// workspace's rules to an unscoped task.
    pub fn list_guardrails(&self, workspace: Option<&str>) -> Result<Vec<GuardrailRule>, HubError> {
        let workspace = normalized_workspace(workspace);
        let mut stmt;
        let rows = match workspace.as_deref() {
            None => {
                stmt = self
                    .conn
                    .prepare(&format!("{SELECT} ORDER BY created_at ASC, id ASC"))?;
                stmt.query_map([], map_rule)?
            }
            Some(ws) => {
                stmt = self.conn.prepare(&format!(
                    "{SELECT} WHERE workspace_path IS NULL OR workspace_path = ?1
                     ORDER BY created_at ASC, id ASC"
                ))?;
                stmt.query_map(params![ws], map_rule)?
            }
        };
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn set_guardrail_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<GuardrailRule, HubError> {
        self.ensure_mutable()?;
        let changed = self.conn.execute(
            "UPDATE guardrail_rules SET enabled = ?1 WHERE id = ?2",
            params![if enabled { 1 } else { 0 }, id],
        )?;
        if changed == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        self.get_guardrail(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    pub fn remove_guardrail(&self, id: &str) -> Result<(), HubError> {
        self.ensure_mutable()?;
        let removed = self
            .conn
            .execute("DELETE FROM guardrail_rules WHERE id = ?1", params![id])?;
        if removed == 0 {
            return Err(HubError::NotFound(id.into()));
        }
        Ok(())
    }

    /// Enabled rules for one stage that apply to `workspace`. Hub-wide
    /// rules always apply; an unscoped (`None`) subject inherits *only*
    /// hub-wide rules, never another workspace's restrictions.
    pub fn effective_guardrails(
        &self,
        stage: GuardrailStage,
        workspace: Option<&str>,
    ) -> Result<Vec<GuardrailRule>, HubError> {
        let workspace = normalized_workspace(workspace);
        Ok(self
            .list_guardrails(workspace.as_deref())?
            .into_iter()
            .filter(|rule| rule.enabled && rule.stage == stage)
            .filter(|rule| workspace.is_some() || rule.workspace.is_none())
            .collect())
    }

    /// Screen `content` and audit a tripwire. Never records the content.
    pub fn screen_content(
        &self,
        stage: GuardrailStage,
        workspace: Option<&str>,
        subject_kind: &str,
        subject_id: &str,
        content: &str,
    ) -> Result<GuardrailVerdict, HubError> {
        let rules = self.effective_guardrails(stage, workspace)?;
        if rules.is_empty() {
            return Ok(GuardrailVerdict::default());
        }
        let verdict = run_rules(&rules, content);
        if let Some(trip) = verdict.tripped() {
            let rule = rules.iter().find(|rule| rule.id == trip.rule_id);
            let root = normalized_workspace(workspace)
                .map(PathBuf::from)
                .unwrap_or_else(|| self.data_dir.clone());
            let _ = self.record_audit_event(
                &root,
                &root,
                "guardrail.tripped",
                &serde_json::json!({
                    "stage": stage.as_str(),
                    "subject": { "kind": subject_kind, "id": subject_id },
                    "rule_id": trip.rule_id,
                    "rule": trip.name,
                    "mode": trip.mode.as_str(),
                    "spec_kind": rule.map(|rule| rule.spec.kind()),
                    "detail": trip.detail,
                    "content_chars": content.chars().count(),
                    "checked": verdict.checked,
                    "trips": verdict.trips.len(),
                })
                .to_string(),
                None,
            );
        }
        Ok(verdict)
    }

    /// [`Self::screen_content`] as a hard stop: `Err(Invalid)` on a trip.
    pub(crate) fn enforce_guardrails(
        &self,
        stage: GuardrailStage,
        workspace: Option<&str>,
        subject_kind: &str,
        subject_id: &str,
        content: &str,
    ) -> Result<(), HubError> {
        let verdict = self.screen_content(stage, workspace, subject_kind, subject_id, content)?;
        match verdict.tripped() {
            Some(trip) => Err(HubError::Invalid(trip_message(stage, trip))),
            None => Ok(()),
        }
    }

    /// Replies on `task_id` from `agents`, created strictly after `since`
    /// (RFC3339). Both workflow stages and Schedule attempts correlate
    /// replies this way.
    pub(crate) fn replies_since(
        &self,
        task_id: &str,
        agents: &[String],
        since: Option<&str>,
    ) -> Result<Vec<MessageRecord>, HubError> {
        let since = since
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let mut stmt = self.conn.prepare(
            "SELECT id, from_agent, to_agent, workspace_path, task_id, kind, status,
                    subject, body, created_at, acked_at
             FROM messages WHERE task_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![task_id], |r| {
            Ok(MessageRecord {
                id: r.get(0)?,
                from_agent: r.get(1)?,
                to_agent: r.get(2)?,
                workspace_path: r.get(3)?,
                task_id: r.get(4)?,
                kind: r.get(5)?,
                status: r.get(6)?,
                subject: r.get(7)?,
                body: r.get(8)?,
                created_at: r.get(9)?,
                acked_at: r.get(10)?,
            })
        })?;
        let mut out = Vec::new();
        for message in rows {
            let message = message?;
            if !agents.iter().any(|agent| agent == &message.from_agent) {
                continue;
            }
            if let Some(since) = since {
                let created = chrono::DateTime::parse_from_rfc3339(&message.created_at)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
                if created.is_none_or(|at| at <= since) {
                    continue;
                }
            }
            out.push(message);
        }
        Ok(out)
    }

    /// Output guardrails for a workflow stage: the finishing agents'
    /// replies since the stage was dispatched, plus the hand-back note.
    pub(crate) fn enforce_stage_output_guardrails(
        &self,
        task: &TaskRecord,
        agents: &[String],
        note: Option<&str>,
    ) -> Result<(), HubError> {
        let workspace = task.workspace_path.as_deref();
        if self
            .effective_guardrails(GuardrailStage::Output, workspace)?
            .is_empty()
        {
            return Ok(());
        }
        let since = match task.last_message_id.as_deref() {
            Some(id) => self.get_message(id)?.map(|m| m.created_at),
            None => None,
        };
        let mut contents: Vec<String> = self
            .replies_since(&task.id, agents, since.as_deref())?
            .into_iter()
            .map(|m| m.body)
            .collect();
        if let Some(note) = note.map(str::trim).filter(|n| !n.is_empty()) {
            contents.push(note.to_string());
        }
        for content in &contents {
            self.enforce_guardrails(GuardrailStage::Output, workspace, "task", &task.id, content)?;
        }
        Ok(())
    }

    /// First output-guardrail tripwire among the replies to this attempt,
    /// as the detail string the tick reports. `None` when nothing tripped
    /// (or no output rule applies to this workspace).
    pub(crate) fn schedule_output_trip(
        &self,
        task: &ScheduledTask,
        since: &str,
    ) -> Result<Option<String>, HubError> {
        if self
            .effective_guardrails(GuardrailStage::Output, Some(&task.workspace))?
            .is_empty()
        {
            return Ok(None);
        }
        let agents = self.schedule_target_agent_ids(&task.target)?;
        for reply in self.replies_since(&task.id, &agents, Some(since))? {
            let verdict = self.screen_content(
                GuardrailStage::Output,
                Some(&task.workspace),
                "schedule",
                &task.id,
                &reply.body,
            )?;
            if let Some(trip) = verdict.tripped() {
                return Ok(Some(trip_message(GuardrailStage::Output, trip)));
            }
        }
        Ok(None)
    }
}

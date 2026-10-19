//! Standing oversight role and temporary swaps (#342 / T2).
//!
//! `agents.oversight_role` is separate from U21's display `role` and from
//! the permission-roles store. A swap is an `oversight.exception` audit row,
//! not a second table and not a badge edit. Advisory only: a row never
//! grants tool, vault, or network rights, and never authorizes self-review.

mod oversight_rules;

use super::super::*;
use oversight_rules::{
    clean_expiry, clean_oversight_role, clean_reason, clean_scope, closed_ids, lapse_reason,
    parse_exception, require_role,
};
use rusqlite::OptionalExtension;

const OVERSIGHT_ROOT: &str = "oversight";
const OP_EXCEPTION: &str = "oversight.exception";
const OP_GRADUATED: &str = "oversight.trial_graduated";
const OP_AUTHORITY: &str = "oversight.authority";
const AUTHORITY_META: &str = "oversight_exception_authority";
const OWNER: &str = "human";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OversightExceptionScope {
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub issue: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OversightExceptionRequest {
    pub agent_id: String,
    pub to_role: String,
    pub scope: OversightExceptionScope,
    pub expires_at: Option<String>,
    pub reason: String,
}

#[derive(Clone)]
pub(super) struct ExceptionFact {
    action: String,
    grant_id: String,
    agent: String,
    from_role: String,
    to_role: String,
    scope: OversightExceptionScope,
    granted_by: String,
    started_at: String,
    expires_at: Option<String>,
    reason: String,
}

impl HubStore {
    pub fn set_oversight_role(
        &self,
        actor: &str,
        id: &str,
        role: Option<&str>,
    ) -> Result<AgentRecord, HubError> {
        self.require_owner(actor)?;
        self.ensure_mutable()?;
        let cleaned = clean_oversight_role(role)?;
        let current = self.standing_oversight_role(id)?;
        if current.as_deref() == Some("trial") && cleaned.as_deref() != Some("trial") {
            return Err(HubError::Invalid(
                "a trial agent leaves trial only through oversight.trial_graduated".into(),
            ));
        }
        if current == cleaned {
            return self.agent_record(id);
        }
        self.conn.execute(
            "UPDATE agents SET oversight_role = ?1 WHERE id = ?2",
            params![cleaned, id],
        )?;
        let action = match &cleaned {
            Some(new_role) => format!(
                "set_oversight_role:{}->{new_role}",
                current.as_deref().unwrap_or("<none>")
            ),
            None => format!(
                "clear_oversight_role:{}",
                current.as_deref().unwrap_or("<none>")
            ),
        };
        self.record_settings_audit_event(&format!("agent.{id}.oversight_role"), id, &action)?;
        self.agent_record(id)
    }

    pub fn oversight_exception_authority(&self) -> Result<String, HubError> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![AUTHORITY_META],
                |row| row.get(0),
            )
            .optional()?;
        match raw.as_deref() {
            None | Some("owner") => Ok("owner".into()),
            Some("delegated_lead") => Ok("delegated_lead".into()),
            Some(other) => Err(HubError::Invalid(format!(
                "unknown oversight exception authority: {other}"
            ))),
        }
    }

    /// Owner path only. A lead, including one with delegated grant
    /// authority, cannot flip this setting.
    pub fn set_oversight_exception_authority(
        &self,
        actor: &str,
        delegated_lead: bool,
    ) -> Result<(), HubError> {
        self.require_owner(actor)?;
        self.ensure_mutable()?;
        let next = if delegated_lead {
            "delegated_lead"
        } else {
            "owner"
        };
        let current = self.oversight_exception_authority()?;
        if current == next {
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![AUTHORITY_META, next],
        )?;
        let body = serde_json::json!({
            "actor": actor,
            "from": current,
            "to": next,
        })
        .to_string();
        self.record_oversight_event("exception_authority", OP_AUTHORITY, &body)?;
        Ok(())
    }

    pub fn grant_oversight_exception(
        &self,
        granted_by: &str,
        request: &OversightExceptionRequest,
    ) -> Result<AuditEvent, HubError> {
        self.grantor_allowed(granted_by)?;
        let _ = self.sweep_oversight_exceptions_at(Utc::now())?;
        let to_role = require_role(&request.to_role)?;
        if to_role == "trial" {
            return Err(HubError::Invalid(
                "an exception cannot assign the trial role".into(),
            ));
        }
        let standing = self.standing_oversight_role(&request.agent_id)?;
        if standing.as_deref() == Some("trial") {
            return Err(HubError::Invalid(
                "a trial agent cannot hold an oversight exception".into(),
            ));
        }
        let scope = clean_scope(&request.scope)?;
        let reason = clean_reason(&request.reason)?;
        let expires_at = clean_expiry(request.expires_at.as_deref(), Utc::now())?;
        if let Some(task_id) = &scope.task_id {
            if self.get_task(task_id)?.is_none() {
                return Err(HubError::NotFound(format!("task {task_id}")));
            }
        }
        let from_role = self
            .effective_oversight_role(&request.agent_id)?
            .unwrap_or_else(|| "<none>".into());
        let fact = ExceptionFact {
            action: "grant".into(),
            grant_id: Uuid::new_v4().to_string(),
            agent: request.agent_id.clone(),
            from_role,
            to_role: to_role.into(),
            scope,
            granted_by: granted_by.into(),
            started_at: Utc::now().to_rfc3339(),
            expires_at,
            reason,
        };
        self.write_exception(&fact, granted_by)
    }

    pub fn revoke_oversight_exception(
        &self,
        actor: &str,
        grant_id: &str,
        reason: &str,
    ) -> Result<AuditEvent, HubError> {
        self.grantor_allowed(actor)?;
        let reason = clean_reason(reason)?;
        let _ = self.sweep_oversight_exceptions_at(Utc::now())?;
        let facts = self.exception_facts()?;
        let active = self.active_grants(&facts, Utc::now())?;
        let grant = active
            .into_iter()
            .find(|fact| fact.grant_id == grant_id)
            .ok_or_else(|| HubError::NotFound(format!("oversight exception {grant_id}")))?;
        let mut revoked = grant;
        revoked.action = "revoke".into();
        revoked.reason = reason;
        revoked.granted_by = actor.into();
        self.write_exception(&revoked, actor)
    }

    pub fn graduate_trial(
        &self,
        actor: &str,
        agent_id: &str,
        to_role: &str,
        reason: &str,
    ) -> Result<AgentRecord, HubError> {
        self.grantor_allowed(actor)?;
        let to_role = require_role(to_role)?;
        self.ensure_mutable()?;
        if to_role == "trial" {
            return Err(HubError::Invalid(
                "graduation must assign a non-trial oversight role".into(),
            ));
        }
        let reason = clean_reason(reason)?;
        let standing = self.standing_oversight_role(agent_id)?;
        if standing.as_deref() != Some("trial") {
            return Err(HubError::Invalid("only a trial agent can graduate".into()));
        }
        self.conn.execute(
            "UPDATE agents SET oversight_role = ?1 WHERE id = ?2",
            params![to_role, agent_id],
        )?;
        let body = serde_json::json!({
            "agent": agent_id,
            "granted_by": actor,
            "reason": reason,
            "to": to_role,
        })
        .to_string();
        self.record_oversight_event(&format!("agent.{agent_id}"), OP_GRADUATED, &body)?;
        self.agent_record(agent_id)
    }

    pub fn effective_oversight_role(&self, id: &str) -> Result<Option<String>, HubError> {
        let standing = self.standing_oversight_role(id)?;
        let facts = self.exception_facts()?;
        let active = self.active_grants(&facts, Utc::now())?;
        if let Some(grant) = active.into_iter().rev().find(|fact| fact.agent == id) {
            return Ok(Some(grant.to_role));
        }
        Ok(standing)
    }

    pub fn sweep_oversight_exceptions(&self) -> Result<usize, HubError> {
        self.sweep_oversight_exceptions_at(Utc::now())
    }

    pub(crate) fn sweep_oversight_exceptions_at(
        &self,
        now: chrono::DateTime<Utc>,
    ) -> Result<usize, HubError> {
        self.ensure_mutable()?;
        let facts = self.exception_facts()?;
        let mut closed = closed_ids(&facts);
        let mut written = 0usize;
        for fact in facts {
            if fact.action != "grant" || closed.contains(&fact.grant_id) {
                continue;
            }
            let Some(reason) = lapse_reason(self, &fact, now)? else {
                continue;
            };
            let mut expired = fact;
            expired.action = "expire".into();
            expired.reason = reason;
            expired.granted_by = OWNER.into();
            self.write_exception(&expired, OWNER)?;
            closed.insert(expired.grant_id);
            written += 1;
        }
        Ok(written)
    }

    fn require_owner(&self, actor: &str) -> Result<(), HubError> {
        if actor == OWNER {
            Ok(())
        } else {
            Err(HubError::Invalid(
                "only the owner can change this oversight setting; a lead cannot enable it".into(),
            ))
        }
    }

    fn grantor_allowed(&self, actor: &str) -> Result<(), HubError> {
        if actor == OWNER {
            return Ok(());
        }
        if self.oversight_exception_authority()? != "delegated_lead" {
            return Err(HubError::Invalid(
                "only the owner can grant oversight exceptions".into(),
            ));
        }
        if self.standing_oversight_role(actor)?.as_deref() != Some("lead") {
            return Err(HubError::Invalid(
                "delegated exception authority requires a standing lead".into(),
            ));
        }
        Ok(())
    }

    fn standing_oversight_role(&self, id: &str) -> Result<Option<String>, HubError> {
        self.conn
            .query_row(
                "SELECT oversight_role FROM agents WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }

    fn agent_record(&self, id: &str) -> Result<AgentRecord, HubError> {
        self.list_agents()?
            .into_iter()
            .find(|agent| agent.id == id)
            .ok_or_else(|| HubError::NotFound(id.to_string()))
    }

    fn exception_facts(&self) -> Result<Vec<ExceptionFact>, HubError> {
        Ok(self
            .list_audit_events(false)?
            .into_iter()
            .filter(|event| event.root_path == OVERSIGHT_ROOT && event.operation == OP_EXCEPTION)
            .filter_map(|event| parse_exception(&event.process_json))
            .collect())
    }

    fn active_grants(
        &self,
        facts: &[ExceptionFact],
        now: chrono::DateTime<Utc>,
    ) -> Result<Vec<ExceptionFact>, HubError> {
        let closed = closed_ids(facts);
        let mut active = Vec::new();
        for fact in facts {
            if fact.action != "grant" || closed.contains(&fact.grant_id) {
                continue;
            }
            if lapse_reason(self, fact, now)?.is_none() {
                active.push(fact.clone());
            }
        }
        Ok(active)
    }

    fn write_exception(&self, fact: &ExceptionFact, actor: &str) -> Result<AuditEvent, HubError> {
        let body = serde_json::json!({
            "action": fact.action,
            "grant_id": fact.grant_id,
            "agent": fact.agent,
            "from": fact.from_role,
            "to": fact.to_role,
            "scope": {
                "task_id": fact.scope.task_id,
                "issue": fact.scope.issue,
                "branch": fact.scope.branch,
            },
            "granted_by": fact.granted_by,
            "started_at": fact.started_at,
            "expires_at": fact.expires_at,
            "reason": fact.reason,
            "actor": actor,
            "grants_p5": false,
            "grants_vault": false,
            "grants_network": false,
            "authorizes_self_review": false,
        })
        .to_string();
        self.record_oversight_event(&format!("agent.{}", fact.agent), OP_EXCEPTION, &body)
    }

    fn record_oversight_event(
        &self,
        path: &str,
        operation: &str,
        process_json: &str,
    ) -> Result<AuditEvent, HubError> {
        let event = self.record_audit_event(
            Path::new(OVERSIGHT_ROOT),
            Path::new(path),
            operation,
            process_json,
            None,
        )?;
        self.set_audit_status(&event.id, "approved")?;
        Ok(AuditEvent {
            status: "approved".into(),
            ..event
        })
    }
}

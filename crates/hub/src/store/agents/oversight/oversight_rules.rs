use super::super::*;
use super::{ExceptionFact, OversightExceptionScope};
use std::collections::HashSet;

pub(super) fn clean_oversight_role(role: Option<&str>) -> Result<Option<String>, HubError> {
    match role.map(str::trim).filter(|role| !role.is_empty()) {
        None => Ok(None),
        Some(role) => Ok(Some(require_role(role)?.to_string())),
    }
}

pub(super) fn require_role(role: &str) -> Result<&str, HubError> {
    match role.trim() {
        "implementer" | "reviewer" | "lead" | "observer" | "trial" => Ok(role.trim()),
        other => Err(HubError::Invalid(format!(
            "oversight role must be implementer, reviewer, lead, observer, or trial (got {other})"
        ))),
    }
}

pub(super) fn clean_reason(reason: &str) -> Result<String, HubError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(HubError::Invalid("oversight reason is required".into()));
    }
    if reason.chars().count() > 500 {
        return Err(HubError::Invalid(
            "oversight reason exceeds 500 characters".into(),
        ));
    }
    Ok(reason.to_string())
}

pub(super) fn clean_scope(
    scope: &OversightExceptionScope,
) -> Result<OversightExceptionScope, HubError> {
    let task_id = blank_to_none(scope.task_id.as_deref());
    let issue = blank_to_none(scope.issue.as_deref());
    let branch = blank_to_none(scope.branch.as_deref());
    if task_id.is_none() && issue.is_none() && branch.is_none() {
        return Err(HubError::Invalid(
            "an oversight exception needs a task, issue, or branch scope".into(),
        ));
    }
    for (label, value) in [("issue", &issue), ("branch", &branch)] {
        if value
            .as_ref()
            .is_some_and(|text| text.chars().count() > 200)
        {
            return Err(HubError::Invalid(format!(
                "oversight scope {label} exceeds 200 characters"
            )));
        }
    }
    Ok(OversightExceptionScope {
        task_id,
        issue,
        branch,
    })
}

fn blank_to_none(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

pub(super) fn clean_expiry(
    expires_at: Option<&str>,
    now: chrono::DateTime<Utc>,
) -> Result<Option<String>, HubError> {
    let Some(raw) = expires_at.map(str::trim).filter(|text| !text.is_empty()) else {
        return Ok(None);
    };
    let when = chrono::DateTime::parse_from_rfc3339(raw)
        .map_err(|err| HubError::Invalid(format!("expires_at must be RFC3339: {err}")))?
        .with_timezone(&Utc);
    if when <= now {
        return Err(HubError::Invalid(
            "expires_at must be later than the grant".into(),
        ));
    }
    Ok(Some(when.to_rfc3339()))
}

pub(super) fn closed_ids(facts: &[ExceptionFact]) -> HashSet<String> {
    facts
        .iter()
        .filter(|fact| fact.action == "revoke" || fact.action == "expire")
        .map(|fact| fact.grant_id.clone())
        .collect()
}

pub(super) fn lapse_reason(
    store: &HubStore,
    fact: &ExceptionFact,
    now: chrono::DateTime<Utc>,
) -> Result<Option<String>, HubError> {
    if let Some(expires_at) = &fact.expires_at {
        let when = chrono::DateTime::parse_from_rfc3339(expires_at)
            .map_err(|err| HubError::Invalid(format!("stored expires_at is invalid: {err}")))?
            .with_timezone(&Utc);
        if when <= now {
            return Ok(Some("deadline".into()));
        }
    }
    if let Some(task_id) = &fact.scope.task_id {
        if store
            .get_task(task_id)?
            .is_some_and(|task| task.status == TaskStatus::Done.as_str())
        {
            return Ok(Some("task completed".into()));
        }
    }
    Ok(None)
}

pub(super) fn parse_exception(process_json: &str) -> Option<ExceptionFact> {
    let value: serde_json::Value = serde_json::from_str(process_json).ok()?;
    Some(ExceptionFact {
        action: value.get("action")?.as_str()?.to_string(),
        grant_id: value.get("grant_id")?.as_str()?.to_string(),
        agent: value.get("agent")?.as_str()?.to_string(),
        from_role: value.get("from")?.as_str()?.to_string(),
        to_role: value.get("to")?.as_str()?.to_string(),
        scope: OversightExceptionScope {
            task_id: json_string(&value, &["scope", "task_id"]),
            issue: json_string(&value, &["scope", "issue"]),
            branch: json_string(&value, &["scope", "branch"]),
        },
        granted_by: value.get("granted_by")?.as_str()?.to_string(),
        started_at: value.get("started_at")?.as_str()?.to_string(),
        expires_at: value
            .get("expires_at")
            .and_then(|item| item.as_str())
            .map(str::to_string),
        reason: value.get("reason")?.as_str()?.to_string(),
    })
}

fn json_string(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut cursor = value;
    for key in path {
        cursor = cursor.get(*key)?;
    }
    cursor.as_str().map(str::to_string)
}

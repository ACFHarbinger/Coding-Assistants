//! P17 / #361 on Schedule dispatch.
//!
//! The outbox body is unchanged when `context_filter` is unset. A filter
//! rebuilds the prior section (earlier non-system messages on this
//! schedule id, plus the re-prompt detail) and leaves the instruction
//! intact. Metadata is audit-only.

use std::path::Path;

use super::super::handoff_filter::{
    apply_context_filter, validate_context_filter, validate_handoff_metadata,
};
use super::super::tasks::render_filtered;
use super::super::*;
use super::time::now_utc_rfc3339;

impl HubStore {
    pub fn set_schedule_handoff(
        &self,
        id: &str,
        metadata: Option<serde_json::Value>,
        context_filter: Option<&str>,
    ) -> Result<ScheduledTask, HubError> {
        self.ensure_mutable()?;
        if self.get_scheduled_task(id)?.is_none() {
            return Err(HubError::NotFound(id.into()));
        }
        if let Some(meta) = &metadata {
            validate_handoff_metadata(meta)?;
        }
        let filter = match context_filter
            .map(str::trim)
            .filter(|spec| !spec.is_empty())
        {
            Some(spec) => {
                validate_context_filter(spec)?;
                Some(spec.to_string())
            }
            None => None,
        };
        let meta_json = metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|err| HubError::Invalid(format!("handoff_metadata encode: {err}")))?;
        self.conn.execute(
            "UPDATE scheduled_tasks
             SET handoff_metadata_json = ?1, context_filter = ?2, updated_at = ?3
             WHERE id = ?4",
            params![meta_json, filter, now_utc_rfc3339(), id],
        )?;
        self.get_scheduled_task(id)?
            .ok_or_else(|| HubError::NotFound(id.into()))
    }

    /// Filter `body` when this schedule opted in. Otherwise return it unchanged.
    pub(crate) fn schedule_handoff_body(
        &self,
        task: &ScheduledTask,
        body: &str,
    ) -> Result<String, HubError> {
        let Some(filter) = task
            .context_filter
            .as_deref()
            .map(str::trim)
            .filter(|spec| !spec.is_empty())
        else {
            return Ok(body.to_string());
        };
        validate_context_filter(filter)?;
        let mut prior = self.prior_context_text(&task.id, None)?;
        let detail = schedule_detail(&task.instruction, body);
        if !detail.is_empty() {
            if !prior.is_empty() {
                prior.push('\n');
            }
            prior.push_str("note: ");
            prior.push_str(detail);
        }
        let filtered = apply_context_filter(&prior, filter)?;
        Ok(render_filtered(&task.instruction, &filtered))
    }

    pub(crate) fn audit_schedule_handoff(
        &self,
        task: &ScheduledTask,
        to_agent: &str,
    ) -> Result<(), HubError> {
        let filter = task
            .context_filter
            .as_deref()
            .map(str::trim)
            .filter(|spec| !spec.is_empty());
        if task.handoff_metadata.is_none() && filter.is_none() {
            return Ok(());
        }
        if let Some(meta) = &task.handoff_metadata {
            validate_handoff_metadata(meta)?;
            self.record_audit_event(
                Path::new(&task.workspace),
                Path::new(&task.workspace),
                "handoff.metadata",
                &serde_json::json!({
                    "schedule_id": task.id,
                    "to_agent": to_agent,
                    "metadata": meta,
                })
                .to_string(),
                None,
            )?;
        }
        if let Some(filter) = filter {
            self.record_audit_event(
                Path::new(&task.workspace),
                Path::new(&task.workspace),
                "handoff.context_filtered",
                &serde_json::json!({
                    "schedule_id": task.id,
                    "to_agent": to_agent,
                    "filter": filter,
                })
                .to_string(),
                None,
            )?;
        }
        Ok(())
    }
}

fn schedule_detail<'a>(instruction: &str, body: &'a str) -> &'a str {
    let prefix = format!("{instruction}\n\n---\n");
    body.strip_prefix(&prefix).unwrap_or("")
}

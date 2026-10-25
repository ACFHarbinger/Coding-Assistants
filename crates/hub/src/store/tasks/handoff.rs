//! Wire P17 / #361 onto task dispatch.
//!
//! No `context_filter`: the body is the instruction plus the advance note,
//! same bytes as before. A filter seeds a reduced prior section instead:
//! upstream non-system messages on this task, then the note, then the
//! filter. The instruction is never filtered and metadata never selects
//! the recipient.

use std::path::Path;

use super::super::handoff_filter::{
    apply_context_filter, validate_context_filter, validate_handoff_metadata,
};
use super::super::*;

pub(super) struct PreparedHandoff {
    pub body: String,
    pub filtered: bool,
    pub prior_before: usize,
    pub prior_after: usize,
}

impl HubStore {
    pub(super) fn validate_step_handoff(step: &WorkflowStep, index: usize) -> Result<(), HubError> {
        if let Some(meta) = &step.handoff_metadata {
            prefix_step(index, validate_handoff_metadata(meta))?;
        }
        if let Some(filter) = step.context_filter.as_deref() {
            prefix_step(index, validate_context_filter(filter))?;
        }
        Ok(())
    }

    pub(super) fn prepare_handoff(
        &self,
        task_id: &str,
        step: &WorkflowStep,
        note: Option<&str>,
    ) -> Result<PreparedHandoff, HubError> {
        let filter = step
            .context_filter
            .as_deref()
            .map(str::trim)
            .filter(|spec| !spec.is_empty());
        let Some(filter) = filter else {
            return Ok(PreparedHandoff {
                body: Self::step_dispatch_body(step, note),
                filtered: false,
                prior_before: 0,
                prior_after: 0,
            });
        };
        validate_context_filter(filter)?;
        let prior = self.prior_context_text(task_id, note)?;
        let filtered = apply_context_filter(&prior, filter)?;
        Ok(PreparedHandoff {
            body: render_filtered(&step.instruction, &filtered),
            filtered: true,
            prior_before: prior.chars().count(),
            prior_after: filtered.chars().count(),
        })
    }

    /// Record metadata and filter stats. Never includes the instruction
    /// or the prior text. No-op when the step opted into neither.
    pub(super) fn record_handoff_audit(
        &self,
        task: &TaskRecord,
        step: &WorkflowStep,
        stage_label: &str,
        prepared: &PreparedHandoff,
    ) -> Result<(), HubError> {
        let root = task.workspace_path.as_deref().unwrap_or(".");
        if let Some(meta) = &step.handoff_metadata {
            validate_handoff_metadata(meta)?;
            self.record_audit_event(
                Path::new(root),
                Path::new(root),
                "handoff.metadata",
                &serde_json::json!({
                    "task_id": task.id,
                    "to_agent": step.agent,
                    "stage": stage_label,
                    "metadata": meta,
                })
                .to_string(),
                None,
            )?;
        }
        if prepared.filtered {
            self.record_audit_event(
                Path::new(root),
                Path::new(root),
                "handoff.context_filtered",
                &serde_json::json!({
                    "task_id": task.id,
                    "to_agent": step.agent,
                    "stage": stage_label,
                    "filter": step.context_filter,
                    "before_chars": prepared.prior_before,
                    "after_chars": prepared.prior_after,
                })
                .to_string(),
                None,
            )?;
        }
        Ok(())
    }

    pub(crate) fn prior_context_text(
        &self,
        task_id: &str,
        note: Option<&str>,
    ) -> Result<String, HubError> {
        let mut stmt = self.conn.prepare(
            "SELECT from_agent, to_agent, kind, body FROM messages
             WHERE task_id = ?1 AND kind != 'system'
             ORDER BY created_at ASC, rowid ASC",
        )?;
        let rows = stmt.query_map(params![task_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut parts = Vec::new();
        for row in rows {
            let (from, to, kind, body) = row?;
            parts.push(format!("{from} -> {to} ({kind}): {body}"));
        }
        if let Some(note) = note.map(str::trim).filter(|text| !text.is_empty()) {
            parts.push(format!("note: {note}"));
        }
        Ok(parts.join("\n"))
    }
}

fn prefix_step(index: usize, result: Result<(), HubError>) -> Result<(), HubError> {
    result.map_err(|err| match err {
        HubError::Invalid(msg) => HubError::Invalid(format!("step {index}: {msg}")),
        other => other,
    })
}

pub(crate) fn render_filtered(instruction: &str, filtered: &str) -> String {
    if filtered.trim().is_empty() {
        instruction.to_string()
    } else {
        format!("{instruction}\n\n---\nPrior context:\n{filtered}")
    }
}

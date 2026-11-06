//! Pending-approval links for `GateCriterion::Command` (S2, #352).
//!
//! Proposals are linked through the existing hash-chained audit rows rather
//! than a new table: `process_json.schedule_gate` names the schedule, the
//! criterion key, the attempt, and (for workflow branches) the stage that
//! produced the proposal. A gate never resubmits while that link is pending,
//! and consumes the matching `os_tool.ran` once a human approves it.

use super::gate_subject::GateSubject;
use crate::store::AuditEvent;
use serde_json::{json, Value};

pub(crate) const GATE_FIELD: &str = "schedule_gate";

pub(crate) fn metadata(
    task_id: &str,
    criterion_index: usize,
    criterion_key: &str,
    attempt: u32,
    stage: Option<u32>,
) -> Value {
    json!({
        GATE_FIELD: {
            "task_id": task_id,
            "criterion_index": criterion_index,
            "criterion_key": criterion_key,
            "attempt": attempt,
            "stage": stage,
        }
    })
}

pub(crate) fn metadata_for(
    subject: &GateSubject<'_>,
    criterion_index: usize,
    criterion_key: &str,
) -> Value {
    metadata(
        subject.id,
        criterion_index,
        criterion_key,
        subject.attempt,
        subject.stage,
    )
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn stage_field(gate: &Value) -> Option<u32> {
    gate.get("stage").and_then(Value::as_u64).map(|n| n as u32)
}

/// `(task_id, criterion_key, attempt, stage)` of a gate-owned os_tool row.
pub(crate) fn link(event: &AuditEvent) -> Option<(String, String, u32, Option<u32>)> {
    let value: Value = serde_json::from_str(&event.process_json).ok()?;
    let gate = value.get(GATE_FIELD)?;
    Some((
        string_field(gate, "task_id")?.to_string(),
        string_field(gate, "criterion_key")?.to_string(),
        gate.get("attempt")?.as_u64()? as u32,
        stage_field(gate),
    ))
}

/// Latest proposal for this task/criterion/attempt/stage, if any.
pub(crate) fn proposal<'a>(
    events: &'a [AuditEvent],
    task_id: &str,
    criterion_key: &str,
    attempt: u32,
    stage: Option<u32>,
) -> Option<&'a AuditEvent> {
    events.iter().rev().find(|event| {
        event.operation == "os_tool.proposed"
            && link(event).is_some_and(|(task, key, number, linked_stage)| {
                task == task_id
                    && key == criterion_key
                    && number == attempt
                    && linked_stage == stage
            })
    })
}

pub(crate) fn proposal_for<'a>(
    events: &'a [AuditEvent],
    subject: &GateSubject<'_>,
    criterion_key: &str,
) -> Option<&'a AuditEvent> {
    proposal(
        events,
        subject.id,
        criterion_key,
        subject.attempt,
        subject.stage,
    )
}

/// Result recorded when a human approved `proposal_id`.
pub(crate) fn approved_run<'a>(
    events: &'a [AuditEvent],
    proposal_id: &str,
) -> Option<&'a AuditEvent> {
    events.iter().rev().find(|event| {
        event.operation == "os_tool.ran"
            && serde_json::from_str::<Value>(&event.process_json)
                .ok()
                .and_then(|value| string_field(&value, "proposed_id").map(str::to_string))
                .as_deref()
                == Some(proposal_id)
    })
}

/// `(exit_code, timed_out, stdout_preview)` recorded on an os_tool.ran row.
pub(crate) fn run_output(event: &AuditEvent) -> Option<(Option<i32>, bool, String)> {
    let value: Value = serde_json::from_str(&event.process_json).ok()?;
    let exit_code = value
        .get("exit_code")
        .and_then(Value::as_i64)
        .map(|code| code as i32);
    let timed_out = value
        .get("timed_out")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let stdout = value
        .get("stdout_preview")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some((exit_code, timed_out, stdout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::AuditEvent;

    fn event(operation: &str, process_json: Value) -> AuditEvent {
        AuditEvent {
            id: "e".into(),
            root_path: "/".into(),
            path: "/".into(),
            operation: operation.into(),
            observed_at: String::new(),
            process_json: process_json.to_string(),
            content_hash: None,
            previous_hash: None,
            event_hash: String::new(),
            status: "pending".into(),
        }
    }

    #[test]
    fn proposal_lookup_includes_stage_identity() {
        let stage0 = event(
            "os_tool.proposed",
            metadata("task", 0, "command:0", 1, Some(0)),
        );
        let events = [stage0];
        assert!(proposal(&events, "task", "command:0", 1, Some(0)).is_some());
        assert!(
            proposal(&events, "task", "command:0", 1, Some(1)).is_none(),
            "a later stage must not reuse an earlier stage's approval"
        );
        assert!(proposal(&events, "task", "command:0", 1, None).is_none());
    }

    #[test]
    fn schedule_rows_without_stage_still_match_none() {
        let legacy = event(
            "os_tool.proposed",
            json!({
                GATE_FIELD: {
                    "task_id": "sched",
                    "criterion_index": 0,
                    "criterion_key": "command:0",
                    "attempt": 2
                }
            }),
        );
        let events = [legacy];
        assert!(proposal(&events, "sched", "command:0", 2, None).is_some());
        assert!(proposal(&events, "sched", "command:0", 2, Some(0)).is_none());
    }
}

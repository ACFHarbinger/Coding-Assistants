//! Pending-approval links for `GateCriterion::Command` (S2, #352).
//!
//! Proposals are linked through the existing hash-chained audit rows rather
//! than a new table: `process_json.schedule_gate` names the schedule, the
//! criterion key, and the attempt that produced the proposal. A gate never
//! resubmits while that link is pending, and consumes the matching
//! `os_tool.ran` once a human approves it.

use crate::store::AuditEvent;
use serde_json::{json, Value};

pub(crate) const GATE_FIELD: &str = "schedule_gate";

pub(crate) fn metadata(
    task_id: &str,
    criterion_index: usize,
    criterion_key: &str,
    attempt: u32,
) -> Value {
    json!({
        GATE_FIELD: {
            "task_id": task_id,
            "criterion_index": criterion_index,
            "criterion_key": criterion_key,
            "attempt": attempt,
        }
    })
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// `(task_id, criterion_key, attempt)` of a gate-owned os_tool audit row.
pub(crate) fn link(event: &AuditEvent) -> Option<(String, String, u32)> {
    let value: Value = serde_json::from_str(&event.process_json).ok()?;
    let gate = value.get(GATE_FIELD)?;
    Some((
        string_field(gate, "task_id")?.to_string(),
        string_field(gate, "criterion_key")?.to_string(),
        gate.get("attempt")?.as_u64()? as u32,
    ))
}

/// Latest proposal for this task/criterion/attempt, if any.
pub(crate) fn proposal<'a>(
    events: &'a [AuditEvent],
    task_id: &str,
    criterion_key: &str,
    attempt: u32,
) -> Option<&'a AuditEvent> {
    events.iter().rev().find(|event| {
        event.operation == "os_tool.proposed"
            && link(event).is_some_and(|(task, key, number)| {
                task == task_id && key == criterion_key && number == attempt
            })
    })
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

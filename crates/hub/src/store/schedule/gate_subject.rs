//! What a gate evaluation is *about* (P20 / #367).
//!
//! Schedule's gate evaluator (`gate.rs`) was written against a
//! [`ScheduledTask`]. Conditional workflow branching needs the very same
//! criteria, combinator, P5 proposal reuse, and fail-closed rules — so the
//! evaluator reads this subject instead of a schedule row, and both
//! Schedule ticks and `advance_task` build one. One evaluator, one
//! expression language, two callers.

use super::super::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GateSubject<'a> {
    /// Correlation id for P5 proposals (`process_json.schedule_gate.task_id`)
    /// and the audit row: a schedule id or a workflow task id.
    pub id: &'a str,
    /// Absolute workspace every `Command` / `MaxLinesPerFile` runs in.
    pub workspace: &'a str,
    /// Attempt number the proposal link is keyed by, so a re-prompted
    /// schedule or a re-activated stage asks for a fresh approval.
    pub attempt: u32,
    /// Workflow stage index, so two stages of the same task that both sit
    /// on attempt 1 cannot reuse each other's P5 approvals. `None` for
    /// schedules (a schedule id + attempt is already unique).
    pub stage: Option<u32>,
    pub criteria: &'a [GateCriterion],
    pub combinator: GateCombinator,
    /// Per-agent `tokens_used` baseline for `MinTokensWritten`. Workflow
    /// tasks take no baseline today, so that criterion fails closed for
    /// them ("no token baseline") exactly as it does for a schedule
    /// created without one.
    pub token_baseline: HashMap<String, i64>,
    /// Agents whose token delta `MinTokensWritten` measures.
    pub agents: Vec<String>,
    /// Audit operation written once per evaluation.
    pub audit_operation: &'static str,
    /// Name of the id field in that audit row (`schedule_id` / `task_id`).
    pub audit_key: &'static str,
}

impl<'a> GateSubject<'a> {
    pub fn for_schedule(task: &'a ScheduledTask, agents: Vec<String>) -> Self {
        Self {
            id: &task.id,
            workspace: &task.workspace,
            attempt: task.attempts,
            stage: None,
            criteria: &task.gate_criteria,
            combinator: task.gate_combinator,
            token_baseline: task.token_baseline.clone(),
            agents,
            audit_operation: "schedule.gate_checked",
            audit_key: "schedule_id",
        }
    }

    /// A workflow stage's branch (`WorkflowStep.branch`), evaluated when
    /// the stage completes. `attempt` is the stage's dispatch count.
    pub fn for_task_branch(
        task_id: &'a str,
        workspace: &'a str,
        stage: u32,
        attempt: u32,
        branch: &'a StepBranch,
        agents: Vec<String>,
    ) -> Self {
        Self {
            id: task_id,
            workspace,
            attempt,
            stage: Some(stage),
            criteria: &branch.criteria,
            combinator: branch.combinator,
            token_baseline: HashMap::new(),
            agents,
            audit_operation: "task.branch_checked",
            audit_key: "task_id",
        }
    }
}

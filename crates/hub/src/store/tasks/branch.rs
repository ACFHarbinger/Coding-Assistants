//! Conditional branching on `WorkflowStep` (P20 / #367).
//!
//! Today `advance_task` moves to `step_index + 1`. A step may now carry a
//! [`StepBranch`]: when its stage completes, the branch's criteria are
//! evaluated with Schedule's own gate evaluator (`GateCriterion` /
//! `GateCombinator`, P5-sandboxed commands, proposal reuse, fail-closed
//! rules — nothing new) and the result picks the next stage. This is
//! dispatch control flow, harness-CLI compatible; no LLM is consulted.
//!
//! A back-edge (branching to a stage already dispatched) consumes that
//! stage's `max_retries` budget, the same rule `retry_task` enforces, so a
//! loop can never run unbounded.

use super::super::*;

/// Where a branch sends the workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BranchTarget {
    /// The linear next stage (or completion after the last one).
    #[default]
    Next,
    /// Jump to a stage by 0-based index — forward to skip, backward to loop.
    Stage { index: u32 },
    /// Finish the task now with status `done`.
    Done,
    /// Finish the task now with status `failed`.
    Fail,
}

/// Optional next-stage selector on a [`WorkflowStep`]. At most one step per
/// stage may define one; the task needs a `workspace_path` for the
/// criteria to run in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepBranch {
    pub criteria: Vec<GateCriterion>,
    #[serde(default)]
    pub combinator: GateCombinator,
    pub on_pass: BranchTarget,
    #[serde(default)]
    pub on_fail: BranchTarget,
}

pub(super) enum StageDecision {
    Activate(i64),
    Complete,
    Fail(String),
}

impl HubStore {
    pub(super) fn validate_branches(
        steps: &[WorkflowStep],
        workspace_path: Option<&str>,
    ) -> Result<(), HubError> {
        let stages = Self::workflow_stages(steps);
        for (stage_index, idxs) in stages.iter().enumerate() {
            let branched: Vec<usize> = idxs
                .iter()
                .copied()
                .filter(|i| steps[*i].branch.is_some())
                .collect();
            if branched.len() > 1 {
                return Err(HubError::Invalid(format!(
                    "stage {stage_index}: only one step may define a branch"
                )));
            }
            let Some(step_index) = branched.first().copied() else {
                continue;
            };
            let branch = steps[step_index].branch.as_ref().expect("filtered");
            if workspace_path.map(str::trim).unwrap_or("").is_empty() {
                return Err(HubError::Invalid(format!(
                    "step {step_index}: branch criteria run in the task workspace; set workspace_path"
                )));
            }
            if branch.criteria.is_empty() {
                return Err(HubError::Invalid(format!(
                    "step {step_index}: branch needs at least one criterion"
                )));
            }
            for target in [branch.on_pass, branch.on_fail] {
                if let BranchTarget::Stage { index } = target {
                    if index as usize >= stages.len() {
                        return Err(HubError::Invalid(format!(
                            "step {step_index}: branch targets stage {index} but the task has {} stage(s)",
                            stages.len()
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn stage_branch<'a>(
        steps: &'a [WorkflowStep],
        stage: &[usize],
    ) -> Option<(usize, &'a StepBranch)> {
        stage
            .iter()
            .copied()
            .find_map(|i| steps[i].branch.as_ref().map(|branch| (i, branch)))
    }

    /// Decide what follows the current (completed) stage.
    pub(super) fn decide_next_stage(
        &self,
        task: &TaskRecord,
        stages: &[Vec<usize>],
    ) -> Result<StageDecision, HubError> {
        let current = task.step_index.max(0) as usize;
        let linear = |current: usize| {
            if current + 1 >= stages.len() {
                StageDecision::Complete
            } else {
                StageDecision::Activate((current + 1) as i64)
            }
        };
        let Some(stage) = stages.get(current) else {
            return Ok(StageDecision::Complete);
        };
        let Some((step_index, branch)) = Self::stage_branch(&task.steps, stage) else {
            return Ok(linear(current));
        };
        let workspace = task
            .workspace_path
            .as_deref()
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .ok_or_else(|| {
                HubError::Invalid(format!(
                    "step {step_index}: branch criteria need a task workspace_path"
                ))
            })?;
        let attempt = *task.attempts.get(&current.to_string()).unwrap_or(&1);
        let agents = stage.iter().map(|i| task.steps[*i].agent.clone()).collect();
        let subject = GateSubject::for_task_branch(
            &task.id,
            workspace,
            current as u32,
            attempt,
            branch,
            agents,
        );
        let result = self.evaluate_gate_subject(&subject)?;
        let summary = result
            .outcomes
            .iter()
            .map(|outcome| outcome.detail.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        if result.pending {
            return Err(HubError::Invalid(format!(
                "step {step_index}: branch is awaiting human approval ({summary}); approve the os_tool proposal, then advance again"
            )));
        }
        let target = if result.passed {
            branch.on_pass
        } else {
            branch.on_fail
        };
        let decision = match target {
            BranchTarget::Next => linear(current),
            BranchTarget::Done => StageDecision::Complete,
            BranchTarget::Fail => StageDecision::Fail(format!(
                "step {step_index}: branch routed to fail ({summary})"
            )),
            BranchTarget::Stage { index } => {
                let index = index as usize;
                if index >= stages.len() {
                    return Err(HubError::Invalid(format!(
                        "step {step_index}: branch targets stage {index} out of range"
                    )));
                }
                match task.attempts.get(&index.to_string()) {
                    // Revisit: bounded by the target stage's max_retries,
                    // the same budget retry_task spends.
                    Some(prior) => {
                        let max_retries = stages[index]
                            .iter()
                            .map(|i| task.steps[*i].max_retries)
                            .max()
                            .unwrap_or(0);
                        if *prior > max_retries {
                            StageDecision::Fail(format!(
                                "step {step_index}: branch back-edge to stage {index} exhausted max_retries ({max_retries}); attempts={prior}"
                            ))
                        } else {
                            StageDecision::Activate(index as i64)
                        }
                    }
                    None => StageDecision::Activate(index as i64),
                }
            }
        };
        let routed_to = match &decision {
            StageDecision::Activate(index) => serde_json::json!({ "stage": index }),
            StageDecision::Complete => serde_json::json!("done"),
            StageDecision::Fail(reason) => serde_json::json!({ "failed": reason }),
        };
        let _ = self.record_audit_event(
            Path::new(workspace),
            Path::new(workspace),
            "task.branch_routed",
            &serde_json::json!({
                "task_id": task.id,
                "from_stage": current,
                "step_index": step_index,
                "passed": result.passed,
                "routed_to": routed_to,
            })
            .to_string(),
            None,
        );
        Ok(decision)
    }
}

//! Gate-criteria evaluation (`roadmaps/schedule.md` S2, #352).
//!
//! `Command`/`MaxLinesPerFile` go through the P5 `os_tool` sandbox —
//! same `SandboxStrictness` policy as any other `os_tool` call, no
//! bypass. A criterion that needs human approval is reported as
//! [`CriterionOutcome::pending`], not as a gate failure: the existing
//! proposal is reused on every tick and its approved `os_tool.ran`
//! result is consumed when it arrives.
//!
//! `MinTokensWritten` reads `agent_metrics` — see the module docs on
//! `types::ScheduledTask` for the accounting gap this has for
//! harness-CLI targets. Missing measurements and an empty baseline fail
//! closed instead of vacuously passing.

use super::super::*;
use super::gate_proposal as proposals;
use super::globset::{glob_match, walk_workspace_files};
use crate::os_tool::{self, OsToolRequest};
use crate::{SandboxStrictness, SettingsStore, SliceStamp};
use serde_json::json;
use std::path::Path;

const DETAIL_LIMIT: usize = 480;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CriterionOutcome {
    /// True only when the criterion definitely passed.
    pub passed: bool,
    /// True when a human approval is outstanding; `passed` is then false.
    pub pending: bool,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_id: Option<String>,
}

impl CriterionOutcome {
    fn passed(detail: impl Into<String>) -> Self {
        Self {
            passed: true,
            pending: false,
            detail: bounded(detail.into()),
            audit_id: None,
        }
    }

    fn failed(detail: impl Into<String>) -> Self {
        Self {
            passed: false,
            pending: false,
            detail: bounded(detail.into()),
            audit_id: None,
        }
    }

    fn awaiting(detail: impl Into<String>, audit_id: String) -> Self {
        Self {
            passed: false,
            pending: true,
            detail: bounded(detail.into()),
            audit_id: Some(audit_id),
        }
    }

    fn status(&self) -> &'static str {
        if self.passed {
            "passed"
        } else if self.pending {
            "pending_approval"
        } else {
            "failed"
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GateCheckResult {
    pub passed: bool,
    /// A distinct outcome from failure: at least one criterion is waiting
    /// for human P5 approval, and no criterion has definitively failed.
    pub pending: bool,
    pub outcomes: Vec<CriterionOutcome>,
}

impl GateCheckResult {
    fn from_outcomes(combinator: GateCombinator, outcomes: Vec<CriterionOutcome>) -> Self {
        let passed = match combinator {
            GateCombinator::All => outcomes.iter().all(|outcome| outcome.passed),
            GateCombinator::Any => outcomes.iter().any(|outcome| outcome.passed),
        };
        let pending = !passed
            && match combinator {
                // One definite failure ends an All gate regardless of an
                // outstanding approval elsewhere.
                GateCombinator::All => {
                    !outcomes
                        .iter()
                        .any(|outcome| !outcome.passed && !outcome.pending)
                        && outcomes.iter().any(|outcome| outcome.pending)
                }
                // An outstanding approval may still make an Any gate pass.
                GateCombinator::Any => outcomes.iter().any(|outcome| outcome.pending),
            };
        Self {
            passed,
            pending,
            outcomes,
        }
    }

    fn workspace_failure(detail: impl Into<String>) -> Self {
        Self {
            passed: false,
            pending: false,
            outcomes: vec![CriterionOutcome::failed(detail)],
        }
    }
}

fn bounded(mut text: String) -> String {
    if text.chars().count() <= DETAIL_LIMIT {
        return text;
    }
    text = text.chars().take(DETAIL_LIMIT).collect();
    text.push('…');
    text
}

fn workspace_ready(workspace: &Path) -> std::io::Result<()> {
    if !workspace.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "workspace is missing or not a directory",
        ));
    }
    std::fs::read_dir(workspace)?;
    Ok(())
}

fn command_passes(check: GateCheck, exit_code: Option<i32>, stdout: &str) -> bool {
    if exit_code != Some(0) {
        return false;
    }
    match check {
        GateCheck::ExitCodeZero => true,
        GateCheck::AtMost { max } => stdout
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|value| value <= max),
    }
}

impl HubStore {
    pub fn evaluate_gate_criteria(
        &self,
        task: &ScheduledTask,
    ) -> Result<GateCheckResult, HubError> {
        if let Err(error) = workspace_ready(Path::new(&task.workspace)) {
            return Ok(GateCheckResult::workspace_failure(format!(
                "workspace {}: {error}",
                task.workspace
            )));
        }
        if task.gate_criteria.is_empty() {
            return Ok(GateCheckResult {
                passed: true,
                pending: false,
                outcomes: Vec::new(),
            });
        }
        let strictness = SettingsStore::open(self.data_dir())
            .effective(Some(&task.workspace))
            .orchestration
            .sandbox_strictness;
        let stamp = SliceStamp::registered(Some(task.id.clone()), None, None);
        let audit_events = self.list_audit_events(false)?;

        let mut outcomes = Vec::with_capacity(task.gate_criteria.len());
        for (index, criterion) in task.gate_criteria.iter().enumerate() {
            outcomes.push(self.evaluate_one(
                criterion,
                task,
                strictness,
                &stamp,
                index,
                &audit_events,
            )?);
        }
        let result = GateCheckResult::from_outcomes(task.gate_combinator, outcomes);
        let details = result
            .outcomes
            .iter()
            .map(|outcome| {
                json!({
                    "status": outcome.status(),
                    "detail": &outcome.detail,
                    "audit_id": &outcome.audit_id,
                })
            })
            .collect::<Vec<_>>();
        let _ = self.record_audit_event(
            Path::new(&task.workspace),
            Path::new(&task.workspace),
            "schedule.gate_checked",
            &json!({
                "schedule_id": task.id,
                "passed": result.passed,
                "pending": result.pending,
                "combinator": task.gate_combinator.as_str(),
                "outcomes": details,
            })
            .to_string(),
            None,
        );
        Ok(result)
    }

    fn evaluate_one(
        &self,
        criterion: &GateCriterion,
        task: &ScheduledTask,
        strictness: SandboxStrictness,
        stamp: &SliceStamp,
        index: usize,
        audit_events: &[AuditEvent],
    ) -> Result<CriterionOutcome, HubError> {
        match criterion {
            GateCriterion::Command {
                program,
                args,
                check,
            } => self.evaluate_command(
                program,
                args,
                task,
                strictness,
                stamp,
                *check,
                index,
                &format!("command:{index}"),
                audit_events,
            ),
            GateCriterion::MaxLinesPerFile { max_lines, globs } => self
                .evaluate_max_lines_per_file(
                    *max_lines,
                    globs,
                    task,
                    strictness,
                    stamp,
                    index,
                    audit_events,
                ),
            GateCriterion::MinTokensWritten { min_tokens } => {
                self.evaluate_min_tokens_written(*min_tokens, task)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn evaluate_command(
        &self,
        program: &str,
        args: &[String],
        task: &ScheduledTask,
        strictness: SandboxStrictness,
        stamp: &SliceStamp,
        check: GateCheck,
        index: usize,
        criterion_key: &str,
        audit_events: &[AuditEvent],
    ) -> Result<CriterionOutcome, HubError> {
        let label = format!("{program} {}", args.join(" "));
        if let Some(proposal) =
            proposals::proposal(audit_events, &task.id, criterion_key, task.attempts)
        {
            if let Some(run) = proposals::approved_run(audit_events, &proposal.id) {
                return Ok(approved_outcome(program, args, check, run));
            }
            return Ok(match proposal.status.as_str() {
                "pending" => CriterionOutcome::awaiting(
                    format!("{label}: awaiting human approval"),
                    proposal.id.clone(),
                ),
                "quarantined" => CriterionOutcome::failed(format!("{label}: approval was denied")),
                "approved" => CriterionOutcome::failed(format!(
                    "{label}: proposal is approved but has no run result"
                )),
                other => CriterionOutcome::failed(format!("{label}: proposal status is {other}")),
            });
        }

        let request = OsToolRequest {
            program: program.to_string(),
            args: args.to_vec(),
            cwd: task.workspace.clone(),
            workspace: Some(task.workspace.clone()),
        };
        let metadata = proposals::metadata(&task.id, index, criterion_key, task.attempts);
        let result = os_tool::submit_with_metadata(
            self,
            request,
            strictness,
            os_tool::DEFAULT_TIMEOUT,
            stamp.clone(),
            Some(metadata),
        )
        .map_err(HubError::Invalid)?;
        match result.status.as_str() {
            "ran" => Ok(CriterionOutcome {
                passed: command_passes(check, result.exit_code, &result.stdout),
                pending: false,
                detail: bounded(format!(
                    "{label}: exit={:?} stdout={:?}",
                    result.exit_code,
                    result.stdout.trim()
                )),
                audit_id: None,
            }),
            "needs_approval" => Ok(CriterionOutcome::awaiting(
                format!(
                    "{label}: {}",
                    result.reason.as_deref().unwrap_or("approval required")
                ),
                result.audit_id,
            )),
            "denied" => Ok(CriterionOutcome::failed(format!(
                "{label}: {}",
                result.reason.as_deref().unwrap_or("denied")
            ))),
            other => Ok(CriterionOutcome::failed(format!(
                "{label}: os_tool status is {other}"
            ))),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn evaluate_max_lines_per_file(
        &self,
        max_lines: u64,
        globs: &[String],
        task: &ScheduledTask,
        strictness: SandboxStrictness,
        stamp: &SliceStamp,
        index: usize,
        audit_events: &[AuditEvent],
    ) -> Result<CriterionOutcome, HubError> {
        let workspace = Path::new(&task.workspace);
        let mut offenders = Vec::new();
        let mut pending: Option<CriterionOutcome> = None;
        let mut checked = 0u32;
        for file in walk_workspace_files(workspace)? {
            let Ok(relative) = file.strip_prefix(workspace) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            if !globs.iter().any(|glob| glob_match(glob, &relative)) {
                continue;
            }
            checked += 1;
            let criterion_key = format!("lines:{index}:{relative}");
            let outcome = self.evaluate_command(
                "wc",
                &["-l".to_string(), file.to_string_lossy().to_string()],
                task,
                strictness,
                stamp,
                GateCheck::AtMost { max: max_lines },
                index,
                &criterion_key,
                audit_events,
            )?;
            if !outcome.passed && !outcome.pending {
                offenders.push(relative);
            } else if outcome.pending {
                pending = Some(outcome);
            }
        }
        if !offenders.is_empty() {
            return Ok(CriterionOutcome::failed(format!(
                "{}/{checked} file(s) over {max_lines} lines: {}",
                offenders.len(),
                offenders.join(", ")
            )));
        }
        if let Some(outcome) = pending {
            return Ok(CriterionOutcome::awaiting(
                format!(
                    "{} file(s) matched; {max_lines}-line check {}",
                    checked, outcome.detail
                ),
                outcome.audit_id.unwrap_or_default(),
            ));
        }
        Ok(CriterionOutcome::passed(format!(
            "{checked} file(s) matched, all <= {max_lines} lines"
        )))
    }

    fn evaluate_min_tokens_written(
        &self,
        min_tokens: u64,
        task: &ScheduledTask,
    ) -> Result<CriterionOutcome, HubError> {
        if task.token_baseline.is_empty() {
            return Ok(CriterionOutcome::failed(
                "no token baseline; measurement is unavailable",
            ));
        }
        let metrics = self.list_agent_metrics()?;
        let mut short: Vec<String> = Vec::new();
        for (agent_id, baseline) in &task.token_baseline {
            let Some(metric) = metrics.iter().find(|metric| &metric.agent_id == agent_id) else {
                short.push(format!("{agent_id}: no measurement"));
                continue;
            };
            let delta = metric.tokens_used.saturating_sub(*baseline).max(0) as u64;
            if delta < min_tokens {
                short.push(format!("{agent_id}: {delta}/{min_tokens}"));
            }
        }
        Ok(if short.is_empty() {
            CriterionOutcome::passed(format!("all agents reached {min_tokens} tokens written"))
        } else {
            CriterionOutcome::failed(format!(
                "short of {min_tokens} tokens written: {}",
                short.join(", ")
            ))
        })
    }
}

fn approved_outcome(
    program: &str,
    args: &[String],
    check: GateCheck,
    run: &AuditEvent,
) -> CriterionOutcome {
    let Some((exit_code, timed_out, stdout)) = proposals::run_output(run) else {
        return CriterionOutcome::failed(format!(
            "{program} {}: approved run metadata is unreadable",
            args.join(" ")
        ));
    };
    let passed = !timed_out && command_passes(check, exit_code, &stdout);
    let detail = format!(
        "{program} {}: approved exit={exit_code:?} stdout={:?}",
        args.join(" "),
        stdout.trim()
    );
    if passed {
        CriterionOutcome::passed(detail)
    } else {
        CriterionOutcome::failed(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_or_timed_out_commands_cannot_pass_a_numeric_gate() {
        let check = GateCheck::AtMost { max: 500 };
        assert!(!command_passes(check, Some(1), "0"));
        assert!(!command_passes(check, None, "0"));
        assert!(command_passes(check, Some(0), "0"));
        assert!(!command_passes(check, Some(0), "501"));
    }
}

//! Gate-criteria evaluation (`roadmaps/schedule.md` S2).
//!
//! `Command`/`MaxLinesPerFile` go through the P5 `os_tool` sandbox —
//! same `SandboxStrictness` policy as any other `os_tool` call, no
//! bypass. `MinTokensWritten` reads `agent_metrics` — see the module
//! docs on `types::ScheduledTask` for the accounting gap this has for
//! harness-CLI targets.

use super::super::*;
use super::globset::{glob_match, walk_workspace_files};
use crate::os_tool::{self, OsToolRequest};
use crate::{SandboxStrictness, SettingsStore, SliceStamp};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CriterionOutcome {
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GateCheckResult {
    pub passed: bool,
    pub outcomes: Vec<CriterionOutcome>,
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
            .and_then(|s| s.parse::<u64>().ok())
            .is_some_and(|value| value <= max),
    }
}

impl HubStore {
    pub fn evaluate_gate_criteria(
        &self,
        task: &ScheduledTask,
    ) -> Result<GateCheckResult, HubError> {
        if task.gate_criteria.is_empty() {
            return Ok(GateCheckResult {
                passed: true,
                outcomes: Vec::new(),
            });
        }
        let strictness = SettingsStore::open(self.data_dir())
            .effective(Some(&task.workspace))
            .orchestration
            .sandbox_strictness;
        let stamp = SliceStamp::registered(Some(task.id.clone()), None, None);

        let mut outcomes = Vec::with_capacity(task.gate_criteria.len());
        for criterion in &task.gate_criteria {
            outcomes.push(self.evaluate_one(criterion, task, strictness, &stamp)?);
        }
        let passed = match task.gate_combinator {
            GateCombinator::All => outcomes.iter().all(|o| o.passed),
            GateCombinator::Any => outcomes.iter().any(|o| o.passed),
        };
        let _ = self.record_audit_event(
            Path::new(&task.workspace),
            Path::new(&task.workspace),
            "schedule.gate_checked",
            &serde_json::json!({
                "schedule_id": task.id,
                "passed": passed,
                "combinator": task.gate_combinator.as_str(),
                "outcomes": outcomes.iter().map(|o| &o.detail).collect::<Vec<_>>(),
            })
            .to_string(),
            None,
        );
        Ok(GateCheckResult { passed, outcomes })
    }

    fn evaluate_one(
        &self,
        criterion: &GateCriterion,
        task: &ScheduledTask,
        strictness: SandboxStrictness,
        stamp: &SliceStamp,
    ) -> Result<CriterionOutcome, HubError> {
        match criterion {
            GateCriterion::Command {
                program,
                args,
                check,
            } => self.evaluate_command(program, args, &task.workspace, strictness, stamp, *check),
            GateCriterion::MaxLinesPerFile { max_lines, globs } => {
                self.evaluate_max_lines_per_file(*max_lines, globs, task, strictness, stamp)
            }
            GateCriterion::MinTokensWritten { min_tokens } => {
                self.evaluate_min_tokens_written(*min_tokens, task)
            }
        }
    }

    fn evaluate_command(
        &self,
        program: &str,
        args: &[String],
        workspace: &str,
        strictness: SandboxStrictness,
        stamp: &SliceStamp,
        check: GateCheck,
    ) -> Result<CriterionOutcome, HubError> {
        let request = OsToolRequest {
            program: program.to_string(),
            args: args.to_vec(),
            cwd: workspace.to_string(),
            workspace: Some(workspace.to_string()),
        };
        let result = os_tool::submit(
            self,
            request,
            strictness,
            os_tool::DEFAULT_TIMEOUT,
            stamp.clone(),
        )
        .map_err(HubError::Invalid)?;
        if result.status != "ran" {
            return Ok(CriterionOutcome {
                passed: false,
                detail: format!(
                    "{program}: {} ({})",
                    result.status,
                    result.reason.unwrap_or_default()
                ),
            });
        }
        let passed = command_passes(check, result.exit_code, &result.stdout);
        Ok(CriterionOutcome {
            passed,
            detail: format!(
                "{program} {}: exit={:?} stdout={:?}",
                args.join(" "),
                result.exit_code,
                result.stdout.trim()
            ),
        })
    }

    fn evaluate_max_lines_per_file(
        &self,
        max_lines: u64,
        globs: &[String],
        task: &ScheduledTask,
        strictness: SandboxStrictness,
        stamp: &SliceStamp,
    ) -> Result<CriterionOutcome, HubError> {
        let workspace = Path::new(&task.workspace);
        let mut offenders = Vec::new();
        let mut checked = 0u32;
        for file in walk_workspace_files(workspace)? {
            let Ok(relative) = file.strip_prefix(workspace) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            if !globs.iter().any(|g| glob_match(g, &relative)) {
                continue;
            }
            checked += 1;
            let outcome = self.evaluate_command(
                "wc",
                &["-l".to_string(), file.to_string_lossy().to_string()],
                &task.workspace,
                strictness,
                stamp,
                GateCheck::AtMost { max: max_lines },
            )?;
            if !outcome.passed {
                offenders.push(relative);
            }
        }
        Ok(CriterionOutcome {
            passed: offenders.is_empty(),
            detail: if offenders.is_empty() {
                format!("{checked} file(s) matched, all <= {max_lines} lines")
            } else {
                format!(
                    "{}/{checked} file(s) over {max_lines} lines: {}",
                    offenders.len(),
                    offenders.join(", ")
                )
            },
        })
    }

    fn evaluate_min_tokens_written(
        &self,
        min_tokens: u64,
        task: &ScheduledTask,
    ) -> Result<CriterionOutcome, HubError> {
        let metrics = self.list_agent_metrics()?;
        let mut short: Vec<String> = Vec::new();
        for (agent_id, baseline) in &task.token_baseline {
            let current = metrics
                .iter()
                .find(|m| &m.agent_id == agent_id)
                .map(|m| m.tokens_used)
                .unwrap_or(0);
            let delta = (current - baseline).max(0) as u64;
            if delta < min_tokens {
                short.push(format!("{agent_id}: {delta}/{min_tokens}"));
            }
        }
        Ok(CriterionOutcome {
            passed: short.is_empty(),
            detail: if short.is_empty() {
                format!("all agents reached {min_tokens} tokens written")
            } else {
                format!("short of {min_tokens} tokens written: {}", short.join(", "))
            },
        })
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

//! `claimant_consistency` probe: unapproved action claims on a slice
//! (Team Scorecard T7 / #347).

use super::probes::PROBE_CLAIMANT_CONSISTENCY;
use super::types::{ProbeState, ScorecardProbe};
use crate::store::models::WorkflowStep;
use crate::store::{HubError, HubStore};
use rusqlite::params;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

/// Which audit operations constitute an *action claim* on a slice.
/// Communication (`tagged.send`), review findings, and wake/task routing
/// are collaboration or evidence — several agents legitimately share a
/// slice there — so they never feed the claimant probe. Only rows that
/// claim "this agent spent/executed on this slice" count.
fn is_claim_op(op: &str) -> bool {
    op == "provider.usage"
        || op.starts_with("os_tool.")
        || op.starts_with("budget.")
        || op.starts_with("mcp.")
}

impl HubStore {
    /// Probe 4 — unapproved action claims on a slice. Multiple actors on
    /// one task or work session is legitimate collaboration (implementer +
    /// reviewer + lead, delegated contributors), so actor plurality alone
    /// is NEVER a verdict. The probe compares stamped action claims against
    /// the slice's approved assignment record — task steps/open/pending
    /// agents, or work-session members — and only flags claimants outside
    /// the approved set. No assignment record → `unknown`, not disagreement.
    /// Task and work-session identifier namespaces stay separate.
    pub(crate) fn probe_claimant_consistency(&self) -> Result<Vec<ScorecardProbe>, HubError> {
        // (namespace, slice_id) -> claimant agent -> source audit row ids
        let mut by_slice: BTreeMap<(String, String), BTreeMap<String, Vec<String>>> =
            BTreeMap::new();
        let mut stmt = self
            .conn
            .prepare("SELECT id, operation, process_json FROM audit_events ORDER BY rowid ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for (id, operation, raw) in rows.flatten() {
            if !is_claim_op(&operation) {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            let agent = value
                .get("actor_agent")
                .or_else(|| value.get("agent"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|a| !a.is_empty())
                .map(str::to_string);
            let Some(agent) = agent else { continue };
            let task = value
                .get("task_id")
                .and_then(Value::as_str)
                .filter(|t| !t.is_empty())
                .map(str::to_string);
            if let Some(task) = task {
                by_slice
                    .entry(("task".to_string(), task))
                    .or_default()
                    .entry(agent)
                    .or_default()
                    .push(id);
                continue;
            }
            let session = value
                .get("work_session_id")
                .or_else(|| value.get("session_id"))
                .and_then(Value::as_str)
                .filter(|t| !t.is_empty())
                .map(str::to_string);
            let Some(session) = session else { continue };
            by_slice
                .entry(("session".to_string(), session))
                .or_default()
                .entry(agent)
                .or_default()
                .push(id);
        }

        let mut out = Vec::new();
        for ((namespace, slice), agents) in by_slice {
            if agents.len() < 2 {
                continue;
            }
            let names: Vec<String> = agents.keys().cloned().collect();
            let mut ids = Vec::new();
            for row_ids in agents.values() {
                ids.extend(row_ids.iter().cloned());
            }
            let subject = format!("{namespace}:{slice}");
            let approved = if namespace == "task" {
                self.task_assignments(&slice)?
            } else {
                self.session_assignments(&slice)?
            };
            match approved {
                None => out.push(ScorecardProbe {
                    probe: PROBE_CLAIMANT_CONSISTENCY.to_string(),
                    subject,
                    state: ProbeState::Unknown,
                    reason: Some(
                        "no approved assignment record for this slice — actor plurality is not an ownership verdict"
                            .to_string(),
                    ),
                    source_event_ids: ids,
                    linked_agents: names,
                }),
                Some(approved) => {
                    let unapproved: Vec<String> = names
                        .iter()
                        .filter(|n| !approved.contains(*n))
                        .cloned()
                        .collect();
                    if unapproved.is_empty() {
                        out.push(ScorecardProbe {
                            probe: PROBE_CLAIMANT_CONSISTENCY.to_string(),
                            subject,
                            state: ProbeState::Ok,
                            reason: None,
                            source_event_ids: ids,
                            linked_agents: names,
                        });
                    } else {
                        let approved_list: Vec<String> =
                            approved.iter().cloned().collect();
                        out.push(ScorecardProbe {
                            probe: PROBE_CLAIMANT_CONSISTENCY.to_string(),
                            subject,
                            state: ProbeState::Disagreement,
                            reason: Some(format!(
                                "unapproved claimant(s) {} on slice assigned to {}",
                                unapproved.join(", "),
                                approved_list.join(", ")
                            )),
                            source_event_ids: ids,
                            linked_agents: unapproved.clone(),
                        });
                    }
                }
            }
        }
        Ok(out)
    }

    /// Approved agent set for a Hub task: its workflow steps plus the
    /// open/pending agent columns. `None` when the task is not a Hub task —
    /// the probe must not invent ownership for slices it has no record of.
    fn task_assignments(&self, task_id: &str) -> Result<Option<HashSet<String>>, HubError> {
        let row: Option<(String, String, String)> = self
            .conn
            .query_row(
                "SELECT steps_json, open_agents_json, pending_agents_json FROM tasks WHERE id = ?1",
                params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();
        let Some((steps, open, pending)) = row else {
            return Ok(None);
        };
        let mut set = HashSet::new();
        if let Ok(steps) = serde_json::from_str::<Vec<WorkflowStep>>(&steps) {
            for step in steps {
                let agent = step.agent.trim();
                if !agent.is_empty() {
                    set.insert(agent.to_string());
                }
            }
        }
        for column in [open, pending] {
            if let Ok(agents) = serde_json::from_str::<Vec<String>>(&column) {
                for agent in agents {
                    let agent = agent.trim();
                    if !agent.is_empty() {
                        set.insert(agent.to_string());
                    }
                }
            }
        }
        Ok(Some(set))
    }

    /// Approved agent set for a work session: its enrolled members. A
    /// session with no member rows has no approved assignment to check
    /// against, so the probe reports `unknown` rather than guessing.
    fn session_assignments(&self, session_id: &str) -> Result<Option<HashSet<String>>, HubError> {
        let mut stmt = self
            .conn
            .prepare("SELECT agent_id FROM work_session_members WHERE session_id = ?1")?;
        let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;
        let members: HashSet<String> = rows
            .flatten()
            .map(|member| member.trim().to_string())
            .filter(|member| !member.is_empty())
            .collect();
        if members.is_empty() {
            Ok(None)
        } else {
            Ok(Some(members))
        }
    }
}

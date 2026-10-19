//! `claimant_consistency` probe: unapproved action claims on a slice
//! (Team Scorecard T7 / #347).

use super::probes::PROBE_CLAIMANT_CONSISTENCY;
use super::types::{ProbeState, ScorecardProbe};
use crate::store::models::WorkflowStep;
use crate::store::{HubError, HubStore};
use rusqlite::params;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

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

/// One claimed action row: (audit row id, observed_at).
type ClaimRow = (String, String);
/// Claimant agent -> their claimed rows.
type ClaimsByAgent = BTreeMap<String, Vec<ClaimRow>>;
/// (namespace, slice id) -> claims by agent.
type ClaimsBySlice = BTreeMap<(String, String), ClaimsByAgent>;

/// Evidence counts only when it predates the action: being handed a slice
/// after acting on it is not an assignment. Parsed as RFC 3339; falls back
/// to lexicographic order only if a timestamp fails to parse.
fn time_lte(evidence: &str, action: &str) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(evidence),
        chrono::DateTime::parse_from_rfc3339(action),
    ) {
        (Ok(evidence), Ok(action)) => evidence <= action,
        _ => evidence <= action,
    }
}

impl HubStore {
    /// Probe 4 — unapproved action claims on a slice. Multiple actors on
    /// one task or work session is legitimate collaboration, so actor
    /// plurality alone is NEVER a verdict. Approval has two independent
    /// grounds, both assignment facts:
    ///
    /// - the *current* assignment record (task steps/open/pending agents,
    ///   work-session members) — the agent is assigned now;
    /// - *historical* handoff evidence at or before the agent's earliest
    ///   claimed action on the slice — a durable message or wake carrying
    ///   the task id, or session membership, timestamped before the work.
    ///   This is what keeps a reassignment from falsely flagging earlier
    ///   legitimate work: an agent handed the slice then, and since
    ///   reassigned away, was approved *at the relevant time*.
    ///
    /// Only a claimant with NEITHER ground disagrees. A slice with no
    /// assignment facts at all → `unknown`, not disagreement. Task and
    /// work-session identifier namespaces stay separate.
    pub(crate) fn probe_claimant_consistency(&self) -> Result<Vec<ScorecardProbe>, HubError> {
        let mut by_slice: ClaimsBySlice = BTreeMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, operation, observed_at, process_json FROM audit_events ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for (id, operation, observed_at, raw) in rows.flatten() {
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
                    .push((id, observed_at));
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
                .push((id, observed_at));
        }

        let mut out = Vec::new();
        for ((namespace, slice), agents) in by_slice {
            let names: Vec<String> = agents.keys().cloned().collect();
            let mut ids = Vec::new();
            for rows in agents.values() {
                ids.extend(rows.iter().map(|(id, _)| id.clone()));
            }
            let subject = format!("{namespace}:{slice}");

            let current = if namespace == "task" {
                self.task_assignments(&slice)?
            } else {
                self.session_assignments(&slice)?
            };
            let evidence = self.assignment_evidence(&namespace, &slice)?;
            let facts_exist = current.is_some() || !evidence.is_empty();

            let unapproved: Vec<String> = names
                .iter()
                .filter(|name| {
                    if current.as_ref().is_some_and(|set| set.contains(*name)) {
                        return false;
                    }
                    // Earliest claimed action: once handed the slice before
                    // acting, later actions on it stay grounded.
                    let earliest = agents
                        .get(*name)
                        .map(|rows| {
                            rows.iter()
                                .map(|(_, at)| at.as_str())
                                .min()
                                .unwrap_or("")
                                .to_string()
                        })
                        .unwrap_or_default();
                    !evidence
                        .get(*name)
                        .is_some_and(|handoff| time_lte(handoff, &earliest))
                })
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
            } else if !facts_exist {
                out.push(ScorecardProbe {
                    probe: PROBE_CLAIMANT_CONSISTENCY.to_string(),
                    subject,
                    state: ProbeState::Unknown,
                    reason: Some(
                        "no assignment facts for this slice (no current assignment, no historical handoff) — actor plurality is not an ownership verdict"
                            .to_string(),
                    ),
                    source_event_ids: ids,
                    linked_agents: names,
                });
            } else {
                let grounds: Vec<String> = current
                    .as_ref()
                    .map(|set| {
                        let mut list: Vec<String> = set.iter().cloned().collect();
                        list.sort();
                        list
                    })
                    .unwrap_or_default();
                out.push(ScorecardProbe {
                    probe: PROBE_CLAIMANT_CONSISTENCY.to_string(),
                    subject,
                    state: ProbeState::Disagreement,
                    reason: Some(format!(
                        "unapproved claimant(s) {} on slice with assignment facts ({})",
                        unapproved.join(", "),
                        if grounds.is_empty() {
                            "historical handoffs only".to_string()
                        } else {
                            grounds.join(", ")
                        }
                    )),
                    source_event_ids: ids,
                    linked_agents: unapproved.clone(),
                });
            }
        }
        Ok(out)
    }

    /// Approved agent set for a Hub task: its workflow steps plus the
    /// open/pending agent columns. `None` when the task is not a Hub task.
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
    /// session with no member rows has no current assignment.
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

    /// Historical handoff evidence per agent: the earliest time the Hub
    /// handed them this slice. Tasks: durable messages carrying the task id,
    /// and wakes whose message carries it. Sessions: membership join time.
    /// These timestamped facts are what makes the check evaluate the
    /// assignment *at the time of the work*, not at read time.
    fn assignment_evidence(
        &self,
        namespace: &str,
        slice: &str,
    ) -> Result<HashMap<String, String>, HubError> {
        let mut evidence: HashMap<String, String> = HashMap::new();
        let absorb = |agent: String, at: String, evidence: &mut HashMap<String, String>| {
            let agent = agent.trim();
            if agent.is_empty() {
                return;
            }
            evidence
                .entry(agent.to_string())
                .and_modify(|existing| {
                    if at < *existing {
                        *existing = at.clone();
                    }
                })
                .or_insert(at);
        };
        if namespace == "task" {
            let mut stmt = self.conn.prepare(
                "SELECT to_agent, MIN(created_at) FROM messages \
                 WHERE task_id = ?1 AND to_agent IS NOT NULL AND to_agent != '' GROUP BY to_agent",
            )?;
            let rows = stmt.query_map(params![slice], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for (agent, at) in rows.flatten() {
                absorb(agent, at, &mut evidence);
            }
            let mut stmt = self.conn.prepare(
                "SELECT w.target_agent, MIN(w.created_at) FROM wake_requests w \
                 JOIN messages m ON m.id = w.message_id \
                 WHERE m.task_id = ?1 AND w.target_agent IS NOT NULL AND w.target_agent != '' \
                 GROUP BY w.target_agent",
            )?;
            let rows = stmt.query_map(params![slice], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for (agent, at) in rows.flatten() {
                absorb(agent, at, &mut evidence);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT agent_id, MIN(created_at) FROM work_session_members \
                 WHERE session_id = ?1 AND agent_id != '' GROUP BY agent_id",
            )?;
            let rows = stmt.query_map(params![slice], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for (agent, at) in rows.flatten() {
                absorb(agent, at, &mut evidence);
            }
        }
        Ok(evidence)
    }
}

#[cfg(test)]
#[path = "probe_claimant_tests.rs"]
mod probe_claimant_tests;

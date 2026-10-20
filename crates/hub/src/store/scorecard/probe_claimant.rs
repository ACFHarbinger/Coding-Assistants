//! `claimant_consistency` probe: action claims without time-valid authority
//! (Team Scorecard T7 / #347). Missing authority is `unknown`.

use super::probes::PROBE_CLAIMANT_CONSISTENCY;
use super::types::{ProbeState, ScorecardProbe};
use crate::store::{HubError, HubStore};
use rusqlite::params;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

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
/// after acting on it is not an assignment. Invalid timestamps cannot
/// prove authority.
fn time_lte(evidence: &str, action: &str) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(evidence),
        chrono::DateTime::parse_from_rfc3339(action),
    ) {
        (Ok(evidence), Ok(action)) => evidence <= action,
        _ => false,
    }
}

impl HubStore {
    /// Probe 4 — action claims checked against authority that existed when
    /// the work happened. Actor plurality is never a verdict, and the
    /// current task or session roster is not one either: it has no
    /// interval, so it cannot approve past work or prove someone was
    /// excluded. A missing interval is `unknown`, not a disagreement.
    ///
    /// Time-valid authority, per agent, at or before that agent's earliest
    /// claimed action:
    ///
    /// - tasks: a `handoff` message for the task, or a wake whose linked
    ///   message is that handoff. Ordinary messages and wakes attached to
    ///   them are communication, not an assignment;
    /// - sessions: a membership row whose `created_at` is at or before the
    ///   action. Joining later does not cover the action. A removed member
    ///   leaves no row, so their earlier work stays `unknown` rather than
    ///   a disagreement.
    ///
    /// Every claimant authorized → `ok`. Anyone without that authority →
    /// `unknown`, including a single claimant. Task and work-session
    /// identifier namespaces stay separate.
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
            let evidence = self.assignment_evidence(&namespace, &slice)?;
            let unproven: Vec<String> = names
                .iter()
                .filter(|name| {
                    // Check real instants for every action, not the lexical
                    // minimum of differently offset RFC3339 strings.
                    !evidence.get(*name).is_some_and(|handoff| {
                        agents
                            .get(*name)
                            .is_some_and(|rows| rows.iter().all(|(_, at)| time_lte(handoff, at)))
                    })
                })
                .cloned()
                .collect();

            if unproven.is_empty() {
                out.push(ScorecardProbe {
                    probe: PROBE_CLAIMANT_CONSISTENCY.to_string(),
                    subject,
                    state: ProbeState::Ok,
                    reason: None,
                    source_event_ids: ids,
                    linked_agents: names,
                });
            } else {
                out.push(ScorecardProbe {
                    probe: PROBE_CLAIMANT_CONSISTENCY.to_string(),
                    subject,
                    state: ProbeState::Unknown,
                    reason: Some(format!(
                        "no time-valid authorization for {} at the action — current assignment and ordinary messages are not historical authority; actor plurality is not an ownership verdict",
                        unproven.join(", ")
                    )),
                    source_event_ids: ids,
                    linked_agents: unproven,
                });
            }
        }
        Ok(out)
    }

    /// Earliest time-valid authorization instant per agent.
    ///
    /// Tasks accept only `kind = handoff` messages, plus wakes whose linked
    /// message is that handoff (the dispatch path). Any other message or
    /// wake that merely carries the task id is communication. Sessions
    /// accept the membership join time. The current roster is intentionally
    /// not read here.
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
                    if time_lte(&at, existing) && &at != existing {
                        *existing = at.clone();
                    }
                })
                .or_insert(at);
        };
        if namespace == "task" {
            let mut stmt = self.conn.prepare(
                "SELECT to_agent, created_at FROM messages \
                 WHERE task_id = ?1 AND kind = 'handoff' \
                   AND to_agent IS NOT NULL AND to_agent != ''",
            )?;
            let rows = stmt.query_map(params![slice], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for (agent, at) in rows.flatten() {
                absorb(agent, at, &mut evidence);
            }
            // A wake naming a different target cannot backdate that target's
            // authority to the older message's creation time.
            let mut stmt = self.conn.prepare(
                "SELECT w.target_agent, w.created_at, m.created_at FROM wake_requests w \
                 JOIN messages m ON m.id = w.message_id \
                 WHERE m.task_id = ?1 AND m.kind = 'handoff' \
                   AND m.to_agent = w.target_agent \
                   AND w.target_agent IS NOT NULL AND w.target_agent != ''",
            )?;
            let rows = stmt.query_map(params![slice], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            for (agent, at, handoff_at) in rows.flatten() {
                if time_lte(&handoff_at, &at) {
                    absorb(agent, at, &mut evidence);
                }
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

#[cfg(test)]
#[path = "probe_claimant_authority_tests.rs"]
mod probe_claimant_authority_tests;

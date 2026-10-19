//! Advisory consistency probes for Team Scorecard T7 (#347).
//!
//! Each probe compares two records that must agree by construction and
//! reports `ok` | `disagreement` | `unknown` per subject. A disagreement
//! persists one hash-chained `scorecard.disagreement` row linking both
//! sources (deduped against unresolved rows); `unknown` names the missing
//! source instead of pretending the check ran. Probes are pure queries over
//! already-stamped rows — no new collectors, no synthetic tasks.

use super::types::{ProbeState, ScorecardProbe};
use crate::store::{HubError, HubStore};
use rusqlite::params;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const PROBE_QUOTA_VS_COST: &str = "quota_vs_cost";
pub const PROBE_GIT_SCOPE_VS_TOUCH_SET: &str = "git_scope_vs_touch_set";
pub const PROBE_NETWORK_VS_PROVIDER_LOG: &str = "network_vs_provider_log";
pub const PROBE_CLAIMANT_CONSISTENCY: &str = "claimant_consistency";

pub const DISAGREEMENT_OP: &str = "scorecard.disagreement";
/// Audit-op family T6 (#346) writes network samples under. Probe 3 activates
/// the moment rows with this prefix exist; until then it reports `unknown`.
pub const NETWORK_SAMPLE_OP_PREFIX: &str = "network.sample";

/// Currency movements below this are float noise, not spend.
const CURRENCY_EPSILON: f64 = 0.005;
/// A usage sum more than double the balance drop (beyond epsilon) means the
/// records contradict, not that one is slightly stale. Coarse on purpose:
/// this probe flags inspection candidates, it does not do accounting.
const DIVERGENCE_FACTOR: f64 = 2.0;

impl HubStore {
    /// Compute every probe's per-subject state. Read-only; side effects live
    /// in [`Self::persist_probe_disagreements`].
    pub(crate) fn compute_probe_candidates(&self) -> Result<Vec<ScorecardProbe>, HubError> {
        let mut out = Vec::new();
        out.extend(self.probe_quota_vs_cost()?);
        out.extend(self.probe_network_vs_provider_log()?);
        out.extend(self.probe_claimant_consistency()?);
        // No git source exists in the Hub today: `os_tool.ran` rows record a
        // slice's touch-set (cmdline/cwd), but nothing records git-diff
        // scope. Report the gap instead of passing the probe vacuously.
        out.push(ScorecardProbe {
            probe: PROBE_GIT_SCOPE_VS_TOUCH_SET.to_string(),
            subject: "global".to_string(),
            state: ProbeState::Unknown,
            reason: Some(
                "no git-diff source in the Hub yet; the recorded touch-set half (os_tool.ran cmdline) awaits a git collector"
                    .to_string(),
            ),
            source_event_ids: Vec::new(),
            linked_agents: Vec::new(),
        });
        Ok(out)
    }

    /// Persist one deduped `scorecard.disagreement` row per new disagreement.
    /// Best-effort: a read-only store (e.g. `ca preflight`) skips writing but
    /// still served the computed view above.
    pub(crate) fn persist_probe_disagreements(
        &self,
        probes: &[ScorecardProbe],
    ) -> Result<(), HubError> {
        let fresh: Vec<&ScorecardProbe> = probes
            .iter()
            .filter(|p| p.state == ProbeState::Disagreement)
            .collect();
        if fresh.is_empty() {
            return Ok(());
        }
        if self.ensure_mutable().is_err() {
            return Ok(());
        }

        let mut existing: HashSet<(String, String)> = HashSet::new();
        let mut stmt = self.conn.prepare(
            "SELECT process_json FROM audit_events WHERE operation = ?1 AND status != 'approved'",
        )?;
        let rows = stmt.query_map(params![DISAGREEMENT_OP], |row| row.get::<_, String>(0))?;
        for raw in rows.flatten() {
            if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                let key = (
                    value
                        .get("probe")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    value
                        .get("subject")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                );
                existing.insert(key);
            }
        }

        for probe in fresh {
            let key = (probe.probe.clone(), probe.subject.clone());
            if existing.contains(&key) {
                continue;
            }
            let process = serde_json::json!({
                "kind": "scorecard.probe",
                "probe": probe.probe,
                "subject": probe.subject,
                "reason": probe.reason,
                "source_event_ids": probe.source_event_ids,
                "agents": probe.linked_agents,
            });
            let root = self.data_dir.clone();
            // One bad write must not poison the rest of the projection.
            let _ =
                self.record_audit_event(&root, &root, DISAGREEMENT_OP, &process.to_string(), None);
        }
        Ok(())
    }

    /// Unresolved disagreement rows naming each agent — the card badge.
    pub(crate) fn unresolved_disagreement_counts(&self) -> Result<HashMap<String, u64>, HubError> {
        let mut counts: HashMap<String, u64> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT process_json FROM audit_events WHERE operation = ?1 AND status != 'approved'",
        )?;
        let rows = stmt.query_map(params![DISAGREEMENT_OP], |row| row.get::<_, String>(0))?;
        for raw in rows.flatten() {
            if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                if let Some(agents) = value.get("agents").and_then(Value::as_array) {
                    for agent in agents.iter().filter_map(Value::as_str) {
                        *counts.entry(agent.to_string()).or_default() += 1;
                    }
                }
            }
        }
        Ok(counts)
    }

    /// Probe 1 — measured per-request cost vs account balance movement.
    /// `provider.usage` rows are `measured`; `quota.snapshot` rows (written on
    /// explicit quota refresh) carry the balance side. Percent-window
    /// providers can never satisfy this probe — they stay off the subject list.
    fn probe_quota_vs_cost(&self) -> Result<Vec<ScorecardProbe>, HubError> {
        let mut snapshots: BTreeMap<String, Vec<(String, f64, String)>> = BTreeMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, observed_at, process_json FROM audit_events WHERE operation = 'quota.snapshot' ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for (id, observed_at, raw) in rows.flatten() {
            if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                let provider = value
                    .get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                if let Some(balance) = value.get("balance").and_then(Value::as_f64) {
                    snapshots
                        .entry(provider)
                        .or_default()
                        .push((observed_at, balance, id));
                }
            }
        }

        let mut usage: Vec<(String, f64, String, String)> = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, observed_at, process_json FROM audit_events WHERE operation = 'provider.usage' ORDER BY rowid ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for (id, observed_at, raw) in rows.flatten() {
            if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                let provider = value
                    .get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                if let Some(cost) = value
                    .get("cost")
                    .and_then(Value::as_f64)
                    .filter(|c| c.is_finite() && *c >= 0.0)
                {
                    usage.push((provider, cost, observed_at, id));
                }
            }
        }

        if snapshots.is_empty() {
            return Ok(vec![ScorecardProbe {
                probe: PROBE_QUOTA_VS_COST.to_string(),
                subject: "global".to_string(),
                state: ProbeState::Unknown,
                reason: Some(
                    "no quota.snapshot rows yet — snapshots record on explicit quota refresh"
                        .to_string(),
                ),
                source_event_ids: Vec::new(),
                linked_agents: Vec::new(),
            }]);
        }

        let mut out = Vec::new();
        for (provider, points) in &snapshots {
            if points.len() < 2 {
                out.push(ScorecardProbe {
                    probe: PROBE_QUOTA_VS_COST.to_string(),
                    subject: provider.clone(),
                    state: ProbeState::Unknown,
                    reason: Some("need at least two balance snapshots to form a window".into()),
                    source_event_ids: points.iter().map(|p| p.2.clone()).collect(),
                    linked_agents: Vec::new(),
                });
                continue;
            }
            let mut disagreements = Vec::new();
            for window in points.windows(2) {
                let (prev_t, prev_b, prev_id) = &window[0];
                let (curr_t, curr_b, curr_id) = &window[1];
                let delta = prev_b - curr_b;
                // A top-up makes the window unreadable; skip, do not guess.
                if delta < 0.0 {
                    continue;
                }
                let (sum, ids): (f64, Vec<String>) = usage
                    .iter()
                    .filter(|(p, _, t, _)| p == provider && t > prev_t && t <= curr_t)
                    .fold((0.0, Vec::new()), |(s, mut v), (_, c, _, id)| {
                        v.push(id.clone());
                        (s + c, v)
                    });
                let mut sources = vec![prev_id.clone(), curr_id.clone()];
                sources.extend(ids.iter().cloned());
                let reason = if sum > CURRENCY_EPSILON && delta <= CURRENCY_EPSILON {
                    Some(format!(
                        "measured usage spend {sum:.4} in window with no balance movement"
                    ))
                } else if delta > CURRENCY_EPSILON && sum <= CURRENCY_EPSILON {
                    Some(format!(
                        "balance decreased {delta:.4} with no measured usage rows in window"
                    ))
                } else if sum > CURRENCY_EPSILON
                    && delta > CURRENCY_EPSILON
                    && sum > delta * DIVERGENCE_FACTOR
                {
                    Some(format!(
                        "measured usage spend {sum:.4} exceeds balance decrease {delta:.4} by more than {DIVERGENCE_FACTOR}x"
                    ))
                } else {
                    None
                };
                if let Some(reason) = reason {
                    disagreements.push(ScorecardProbe {
                        probe: PROBE_QUOTA_VS_COST.to_string(),
                        subject: provider.clone(),
                        state: ProbeState::Disagreement,
                        reason: Some(reason),
                        source_event_ids: sources,
                        linked_agents: Vec::new(),
                    });
                }
            }
            if !disagreements.is_empty() {
                out.extend(disagreements);
            } else {
                out.push(ScorecardProbe {
                    probe: PROBE_QUOTA_VS_COST.to_string(),
                    subject: provider.clone(),
                    state: ProbeState::Ok,
                    reason: None,
                    source_event_ids: Vec::new(),
                    linked_agents: Vec::new(),
                });
            }
        }
        Ok(out)
    }

    /// Probe 3 — network samples (T6 contract: `network.sample` audit rows
    /// with a session id) vs the provider call log. Until T6 lands there are
    /// no samples and the probe says so.
    fn probe_network_vs_provider_log(&self) -> Result<Vec<ScorecardProbe>, HubError> {
        let sample_op = format!("{NETWORK_SAMPLE_OP_PREFIX}%");
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM audit_events WHERE operation LIKE ?1",
            params![sample_op],
            |row| row.get(0),
        )?;
        if count == 0 {
            return Ok(vec![ScorecardProbe {
                probe: PROBE_NETWORK_VS_PROVIDER_LOG.to_string(),
                subject: "global".to_string(),
                state: ProbeState::Unknown,
                reason: Some(
                    "no network samples yet — the T6 (#346) sample store is not landed".to_string(),
                ),
                source_event_ids: Vec::new(),
                linked_agents: Vec::new(),
            }]);
        }
        let mut sampled_sessions: HashMap<String, Vec<String>> = HashMap::new();
        let mut stmt = self
            .conn
            .prepare("SELECT id, process_json FROM audit_events WHERE operation LIKE ?1")?;
        let rows = stmt.query_map(params![sample_op], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for (id, raw) in rows.flatten() {
            if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                let session = value
                    .get("work_session_id")
                    .or_else(|| value.get("session_id"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if let Some(session) = session {
                    sampled_sessions.entry(session).or_default().push(id);
                }
            }
        }
        let mut billed_sessions: HashSet<String> = HashSet::new();
        let mut stmt = self
            .conn
            .prepare("SELECT process_json FROM audit_events WHERE operation = 'provider.usage'")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for raw in rows.flatten() {
            if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                if let Some(session) = value
                    .get("work_session_id")
                    .or_else(|| value.get("session_id"))
                    .and_then(Value::as_str)
                {
                    billed_sessions.insert(session.to_string());
                }
            }
        }
        let mut out = Vec::new();
        for (session, ids) in sampled_sessions {
            if !billed_sessions.contains(&session) {
                out.push(ScorecardProbe {
                    probe: PROBE_NETWORK_VS_PROVIDER_LOG.to_string(),
                    subject: session.clone(),
                    state: ProbeState::Disagreement,
                    reason: Some(
                        "session has network samples but no provider.usage rows — sockets without a billed call"
                            .to_string(),
                    ),
                    source_event_ids: ids,
                    linked_agents: Vec::new(),
                });
            }
        }
        if out.is_empty() {
            out.push(ScorecardProbe {
                probe: PROBE_NETWORK_VS_PROVIDER_LOG.to_string(),
                subject: "global".to_string(),
                state: ProbeState::Ok,
                reason: None,
                source_event_ids: Vec::new(),
                linked_agents: Vec::new(),
            });
        }
        Ok(out)
    }

    /// Probe 4 — slice claimant consistency: rows stamped for one task (or
    /// work session) must not name different agents. This is the checkable
    /// core of "writer-lease owner vs slice claimant"; lease tokens are not
    /// roster identity (Codex, T3 review), so the probe never parses them.
    fn probe_claimant_consistency(&self) -> Result<Vec<ScorecardProbe>, HubError> {
        let mut by_slice: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, process_json FROM audit_events \
             WHERE process_json LIKE '%\"task_id\"%' OR process_json LIKE '%\"work_session_id\"%'",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for (id, raw) in rows.flatten() {
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
            let slice = value
                .get("task_id")
                .and_then(Value::as_str)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .or_else(|| {
                    value
                        .get("work_session_id")
                        .or_else(|| value.get("session_id"))
                        .and_then(Value::as_str)
                        .filter(|t| !t.is_empty())
                        .map(str::to_string)
                });
            let Some(slice) = slice else { continue };
            by_slice
                .entry(slice)
                .or_default()
                .entry(agent)
                .or_default()
                .push(id);
        }

        let mut out = Vec::new();
        for (slice, agents) in by_slice {
            if agents.len() < 2 {
                continue;
            }
            let mut ids = Vec::new();
            for row_ids in agents.values() {
                ids.extend(row_ids.iter().cloned());
            }
            let names: Vec<String> = agents.keys().cloned().collect();
            out.push(ScorecardProbe {
                probe: PROBE_CLAIMANT_CONSISTENCY.to_string(),
                subject: slice.clone(),
                state: ProbeState::Disagreement,
                reason: Some(format!(
                    "slice {slice} names multiple claimants: {}",
                    names.join(", ")
                )),
                source_event_ids: ids,
                linked_agents: names,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod probe_tests;

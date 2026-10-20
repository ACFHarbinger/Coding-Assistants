//! Advisory consistency probes for Team Scorecard T7 (#347).
//!
//! Each probe compares two records that must agree by construction and
//! reports `ok` | `disagreement` | `unknown` per subject. A disagreement
//! persists one hash-chained `scorecard.disagreement` row linking both
//! sources (deduped and inserted atomically); `unknown` names the missing
//! or incomparable source instead of pretending the check ran. Probes are
//! pure queries over already-stamped rows — no new collectors, no
//! synthetic tasks.

use super::types::{ProbeState, ScorecardProbe};
use crate::store::{HubError, HubStore};
use rusqlite::params;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const PROBE_QUOTA_VS_COST: &str = "quota_vs_cost";
pub const PROBE_GIT_SCOPE_VS_TOUCH_SET: &str = "git_scope_vs_touch_set";
pub const PROBE_NETWORK_VS_PROVIDER_LOG: &str = "network_vs_provider_log";
pub const PROBE_CLAIMANT_CONSISTENCY: &str = "claimant_consistency";

pub const DISAGREEMENT_OP: &str = "scorecard.disagreement";

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
        Ok(merge_candidates(out))
    }

    /// Persist deduped `scorecard.disagreement` rows. Candidate subjects are
    /// merged first, the existing-row check and the hash-chained inserts run
    /// in ONE transaction (concurrent readers cannot double-insert), and
    /// newly written keys join the dedup set immediately. Best-effort by
    /// design: a read-only connection (e.g. `ca preflight`) or any transient
    /// write failure skips the side-write rather than breaking the read
    /// projection the caller came for.
    pub(crate) fn persist_probe_disagreements(
        &self,
        probes: &[ScorecardProbe],
    ) -> Result<(), HubError> {
        let fresh: Vec<ScorecardProbe> = merge_candidates(
            probes
                .iter()
                .filter(|p| p.state == ProbeState::Disagreement)
                .cloned()
                .collect(),
        );
        if fresh.is_empty() {
            return Ok(());
        }
        if self.ensure_mutable().is_err() {
            return Ok(());
        }
        let Ok(tx) = self.conn.unchecked_transaction() else {
            return Ok(());
        };

        let mut existing: HashSet<(String, String)> = HashSet::new();
        {
            let Ok(mut stmt) = tx.prepare(
                "SELECT process_json FROM audit_events WHERE operation = ?1 AND status != 'approved'",
            ) else {
                return Ok(());
            };
            let Ok(rows) = stmt.query_map(params![DISAGREEMENT_OP], |row| row.get::<_, String>(0))
            else {
                return Ok(());
            };
            for raw in rows.flatten() {
                if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                    existing.insert((
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
                    ));
                }
            }
        }

        for probe in &fresh {
            let key = (probe.probe.clone(), probe.subject.clone());
            if !existing.insert(key) {
                continue; // known, or just written by this same transaction
            }
            let process = serde_json::json!({
                "kind": "scorecard.probe",
                "probe": probe.probe,
                "subject": probe.subject,
                "reason": probe.reason,
                "source_event_ids": probe.source_event_ids,
                "agents": probe.linked_agents,
            });
            // One bad insert must not poison the rest; the transaction only
            // commits rows that chained successfully.
            let _ = crate::store::policies::insert_audit_event(
                &tx,
                &self.data_dir.to_string_lossy(),
                &self.data_dir.to_string_lossy(),
                DISAGREEMENT_OP,
                &process.to_string(),
                None,
            );
        }
        let _ = tx.commit();
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

    /// Probe 3 — stays `unknown` until T6 (#346) lands AND the two crates
    /// agree a local read contract. T6's samples are local-only bounded
    /// rings; raw network samples must not join the synced audit chain, so
    /// the probe cannot assume an audit-op producer. Sockets without billed
    /// calls are also frequently legitimate (MCP servers, git, DNS), which
    /// is exactly why the comparison needs agreed endpoint/time/source
    /// coverage rather than a naive session join.
    fn probe_network_vs_provider_log(&self) -> Result<Vec<ScorecardProbe>, HubError> {
        Ok(vec![ScorecardProbe {
            probe: PROBE_NETWORK_VS_PROVIDER_LOG.to_string(),
            subject: "global".to_string(),
            state: ProbeState::Unknown,
            reason: Some(
                "awaiting the T6 (#346) local-only sample store and an agreed local read contract; sockets without billed calls (MCP, git, DNS) are often legitimate, so endpoint/time/source coverage must be defined before this probe enables"
                    .to_string(),
            ),
            source_event_ids: Vec::new(),
            linked_agents: Vec::new(),
        }])
    }
}

/// Merge candidate results per (probe, subject): disagreement beats unknown
/// beats ok, evidence ids and linked agents union, so one read never emits
/// duplicate subjects (multiple quota windows, concurrent windows, etc.).
fn merge_candidates(probes: Vec<ScorecardProbe>) -> Vec<ScorecardProbe> {
    let mut order: Vec<(String, String)> = Vec::new();
    let mut merged: HashMap<(String, String), ScorecardProbe> = HashMap::new();
    for probe in probes {
        let key = (probe.probe.clone(), probe.subject.clone());
        if !merged.contains_key(&key) {
            order.push(key.clone());
        }
        let entry = merged.entry(key).or_insert_with(|| ScorecardProbe {
            probe: probe.probe.clone(),
            subject: probe.subject.clone(),
            state: probe.state,
            reason: None,
            source_event_ids: Vec::new(),
            linked_agents: Vec::new(),
        });
        let rank = |state: ProbeState| match state {
            ProbeState::Disagreement => 2,
            ProbeState::Unknown => 1,
            ProbeState::Ok => 0,
        };
        if rank(probe.state) > rank(entry.state) {
            entry.state = probe.state;
        }
        if entry.reason.is_none() && probe.reason.is_some() {
            entry.reason = probe.reason.clone();
        }
        for id in &probe.source_event_ids {
            if !entry.source_event_ids.contains(id) {
                entry.source_event_ids.push(id.clone());
            }
        }
        for agent in &probe.linked_agents {
            if !entry.linked_agents.contains(agent) {
                entry.linked_agents.push(agent.clone());
            }
        }
    }
    order
        .into_iter()
        .filter_map(|key| merged.remove(&key))
        .collect()
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod probe_tests;

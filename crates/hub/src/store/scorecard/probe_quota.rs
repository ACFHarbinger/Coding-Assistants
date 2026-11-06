//! `quota_vs_cost` probe: measured per-request cost vs account balance
//! movement (Team Scorecard T7 / #347).

use super::probes::PROBE_QUOTA_VS_COST;
use super::types::{ProbeState, ScorecardProbe};
use crate::store::{HubError, HubStore};
use serde_json::Value;
use std::collections::BTreeMap;

/// Currency movements below this are float noise, not spend.
const CURRENCY_EPSILON: f64 = 0.005;
/// A divergence beyond 2x (in either direction) means the records
/// contradict within the declared coverage model, not that one is slightly
/// stale. Both sources only see CA-routed, balance-type provider activity;
/// out-of-band account use is a possible benign cause the owner can
/// adjudicate from the linked rows. Coarse on purpose: this probe flags
/// inspection candidates, it does not do accounting.
const DIVERGENCE_FACTOR: f64 = 2.0;

impl HubStore {
    /// Probe 1 — measured per-request cost vs account balance movement.
    /// `provider.usage` rows are `measured`; `quota.snapshot` rows (written on
    /// explicit quota refresh) carry the balance side. Percent-window
    /// providers can never satisfy this probe — they stay off the subject
    /// list. Top-up windows are incomparable, not ok; a provider whose
    /// windows were ALL skipped is `unknown`.
    pub(crate) fn probe_quota_vs_cost(&self) -> Result<Vec<ScorecardProbe>, HubError> {
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
            let all_snapshot_ids: Vec<String> = points.iter().map(|p| p.2.clone()).collect();
            if points.len() < 2 {
                out.push(ScorecardProbe {
                    probe: PROBE_QUOTA_VS_COST.to_string(),
                    subject: provider.clone(),
                    state: ProbeState::Unknown,
                    reason: Some("need at least two balance snapshots to form a window".into()),
                    source_event_ids: all_snapshot_ids,
                    linked_agents: Vec::new(),
                });
                continue;
            }
            let mut evaluated = 0usize;
            let mut evidence: Vec<String> = Vec::new();
            let mut disagreements = Vec::new();
            for window in points.windows(2) {
                let (prev_t, prev_b, prev_id) = &window[0];
                let (curr_t, curr_b, curr_id) = &window[1];
                let delta = prev_b - curr_b;
                // A top-up makes the window incomparable; skip, do not guess.
                if delta < 0.0 {
                    continue;
                }
                evaluated += 1;
                let (sum, mut ids): (f64, Vec<String>) = usage
                    .iter()
                    .filter(|(p, _, t, _)| p == provider && t > prev_t && t <= curr_t)
                    .fold((0.0, Vec::new()), |(s, mut v), (_, c, _, id)| {
                        v.push(id.clone());
                        (s + c, v)
                    });
                evidence.push(prev_id.clone());
                evidence.push(curr_id.clone());

                let bigger = sum.max(delta);
                let smaller = sum.min(delta);
                let reason = if bigger <= CURRENCY_EPSILON {
                    None // both sides silent in this window: comparable, quiet
                } else if smaller <= CURRENCY_EPSILON {
                    Some(if sum > delta {
                        format!("measured usage spend {sum:.4} in window with no balance movement")
                    } else {
                        format!(
                            "balance decreased {delta:.4} with no measured usage rows in window (out-of-band account use is a possible benign cause)"
                        )
                    })
                } else if bigger > smaller * DIVERGENCE_FACTOR {
                    Some(format!(
                        "measured usage spend {sum:.4} and balance decrease {delta:.4} diverge by more than {DIVERGENCE_FACTOR}x within the declared coverage model"
                    ))
                } else {
                    None
                };
                if let Some(reason) = reason {
                    let mut sources = vec![prev_id.clone(), curr_id.clone()];
                    sources.append(&mut ids);
                    disagreements.push((reason, sources));
                }
            }

            if evaluated == 0 {
                out.push(ScorecardProbe {
                    probe: PROBE_QUOTA_VS_COST.to_string(),
                    subject: provider.clone(),
                    state: ProbeState::Unknown,
                    reason: Some(
                        "no comparable windows — every interval contained a top-up".to_string(),
                    ),
                    source_event_ids: all_snapshot_ids,
                    linked_agents: Vec::new(),
                });
            } else if let Some((reason, sources)) = disagreements.into_iter().next() {
                let extra = evaluated.saturating_sub(1);
                out.push(ScorecardProbe {
                    probe: PROBE_QUOTA_VS_COST.to_string(),
                    subject: provider.clone(),
                    state: ProbeState::Disagreement,
                    reason: if extra > 0 {
                        Some(format!("{reason} ({extra} more window(s) evaluated)"))
                    } else {
                        Some(reason)
                    },
                    source_event_ids: sources,
                    linked_agents: Vec::new(),
                });
            } else {
                out.push(ScorecardProbe {
                    probe: PROBE_QUOTA_VS_COST.to_string(),
                    subject: provider.clone(),
                    state: ProbeState::Ok,
                    reason: None,
                    source_event_ids: evidence,
                    linked_agents: Vec::new(),
                });
            }
        }
        Ok(out)
    }
}

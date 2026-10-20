//! Schedule budget bridge: pause quota-exhausted due tasks, then run the
//! shared hub tick (S6 / #355, extracted for S5c / #358).
//!
//! This is the single full-fidelity tick entry point both callers use:
//! the desktop in-app poller (`src-tauri`'s `schedule_poller`) and the
//! headless `ca schedule tick` in `crates/cli`. Wake policy and P5 gate
//! approval are unchanged — the bridge does not dispatch itself; it only
//! pauses before handing off to `HubStore::tick_scheduled_tasks`.
//!
//! Hard-stop semantics (owner decision, preserved from S6):
//!
//! * Only tasks with `budget.stop_on_quota_exhausted` are probed.
//! * A probe is only a stop when [`classify_quota`] reports
//!   [`QuotaProbeState::Exhausted`] — a measurable session/weekly window
//!   with `remaining_percent <= 0`. `Unavailable` (missing credentials,
//!   no percent window, balance-only, local-usage-only) is an honest
//!   unknown and never pauses.
//! * The pause goes through `HubStore::pause_scheduled_task`, whose CAS
//!   the generic tick never matches — a concurrent or later plain
//!   `tick_scheduled_tasks` cannot resurrect a quota-paused row, and
//!   pending outbox rows are not sent after the pause.
//! * Probes are cached per agent id within one tick, so a team task and
//!   several tasks sharing an agent cost one probe per agent per tick,
//!   not one per task — provider API cost control is preserved.

use crate::codex::ProviderQuota;
use crate::schedule_probe::{classify_quota, target_quota_state, QuotaProbeState};
use hub::{HubStore, ScheduledTask, TickOutcome, TickSummary};
use std::collections::HashMap;

/// Pause exhausted `stop_on_quota_exhausted` due tasks, then run the
/// shared tick. `probe` maps an agent id to its live quota snapshot;
/// production callers pass [`crate::probe_agent_quota`] (or the
/// home-aware `probe_agent_quota_at`), tests pass a stub. Unknown /
/// unprobed agents stay `Unavailable` and never stop a task.
pub fn tick_with_quota_guard<F>(
    store: &HubStore,
    now: &str,
    mut probe: F,
) -> Result<TickSummary, String>
where
    F: FnMut(&str) -> ProviderQuota,
{
    let due = store.due_scheduled_tasks(now).map_err(|e| e.to_string())?;
    let mut outcomes = Vec::new();
    let mut cache: HashMap<String, ProviderQuota> = HashMap::new();
    for task in &due {
        if !task.budget.stop_on_quota_exhausted {
            continue;
        }
        if let Some(outcome) = pause_if_quota_exhausted(store, task, &mut probe, &mut cache)? {
            outcomes.push(outcome);
        }
    }
    let mut summary = store.tick_scheduled_tasks(now).map_err(|e| e.to_string())?;
    outcomes.append(&mut summary.outcomes);
    Ok(TickSummary { outcomes })
}

fn pause_if_quota_exhausted<F>(
    store: &HubStore,
    task: &ScheduledTask,
    probe: &mut F,
    cache: &mut HashMap<String, ProviderQuota>,
) -> Result<Option<TickOutcome>, String>
where
    F: FnMut(&str) -> ProviderQuota,
{
    let agent_ids = store
        .schedule_target_agent_ids(&task.target)
        .map_err(|e| e.to_string())?;
    let states: Vec<QuotaProbeState> = agent_ids
        .iter()
        .map(|id| {
            let quota = cache.entry(id.clone()).or_insert_with(|| probe(id));
            classify_quota(quota)
        })
        .collect();
    match target_quota_state(&states) {
        QuotaProbeState::Exhausted {
            window_label,
            remaining_percent,
        } => {
            let reason =
                format!("live quota exhausted ({window_label} remaining {remaining_percent}%)");
            Ok(Some(
                store
                    .pause_scheduled_task(&task.id, &reason)
                    .map_err(|e| e.to_string())?,
            ))
        }
        QuotaProbeState::Available | QuotaProbeState::Unavailable { .. } => Ok(None),
    }
}

#[cfg(test)]
#[path = "schedule_bridge_tests.rs"]
mod tests;

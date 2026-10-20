//! In-app Scheduled Goals poller + live-quota budget bridge (S6, #355).
//!
//! Calls the shared `HubStore::tick_scheduled_tasks` engine after pausing
//! due tasks whose live session/weekly % is exhausted. Headless
//! `ca schedule tick` stays credit-allowance only (S4 / S5c). Wake policy
//! and P5 gate approval are unchanged: this slice does not dispatch itself.

use super::quota_codex::ProviderQuota;
use super::quotas::probe_agent_quota;
use super::schedule_probe::{classify_quota, target_quota_state, QuotaProbeState};
use super::store::open_store;
use hub::{HubStore, ScheduledTask, TickOutcome, TickSummary};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, TryLockError,
};
use std::time::Duration;

/// Slower than the 1.5s capture poll. Independent of Settings
/// `quota_auto_refresh_*` (default 300s, off unless enabled). This is the
/// in-app fire cadence while the process is up.
pub const SCHEDULE_POLLER_INTERVAL: Duration = Duration::from_secs(15);

/// Skip overlapping ticks — some quota adapters block for tens of seconds.
static TICK_IN_FLIGHT: Mutex<()> = Mutex::new(());

pub fn spawn_in_app() {
    let cancel = Arc::new(AtomicBool::new(false));
    tauri::async_runtime::spawn(async move {
        run_poller_loop_with(cancel, SCHEDULE_POLLER_INTERVAL, || async {
            let _ = tauri::async_runtime::spawn_blocking(poller_tick_live).await;
        })
        .await;
    });
}

pub async fn run_poller_loop_with<F, Fut>(cancel: Arc<AtomicBool>, interval: Duration, mut tick: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    loop {
        tokio::time::sleep(interval).await;
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        tick().await;
        if cancel.load(Ordering::SeqCst) {
            break;
        }
    }
}

pub fn poller_tick_live() -> Result<TickSummary, String> {
    let _guard = match TICK_IN_FLIGHT.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::WouldBlock) => return Ok(TickSummary::default()),
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
    };
    let store = open_store()?;
    tick_in_app(&store, &chrono::Utc::now().to_rfc3339(), probe_agent_quota)
}

/// Pause exhausted `stop_on_quota_exhausted` tasks, then run the shared tick.
pub fn tick_in_app<F>(store: &HubStore, now: &str, mut probe: F) -> Result<TickSummary, String>
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

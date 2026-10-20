//! In-app Scheduled Goals poller + live-quota budget bridge (S6, #355).
//!
//! Runs the shared `quota::tick_with_quota_guard` engine (pause due tasks
//! whose live session/weekly % is exhausted, then
//! `HubStore::tick_scheduled_tasks`) on a 15s cadence while the app is up.
//! The same engine drives headless `ca schedule tick` (S5c / #358), so the
//! in-app and headless paths have identical pause semantics. Wake policy
//! and P5 gate approval are unchanged: this slice does not dispatch itself.

use super::store::open_store;
use hub::TickSummary;
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
    quota::tick_with_quota_guard(
        &store,
        &chrono::Utc::now().to_rfc3339(),
        quota::probe_agent_quota,
    )
}

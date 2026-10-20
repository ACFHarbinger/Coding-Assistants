//! Poller-loop test for the S6 in-app scheduler. The live-quota bridge
//! tests (pause on exhausted, never invent a stop, no resurrection after
//! pause) moved with the bridge itself to `crates/quota`'s
//! `schedule_bridge_tests` (S5c / #358).

use super::super::schedule_poller::{run_poller_loop_with, SCHEDULE_POLLER_INTERVAL};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

#[tokio::test]
async fn poller_loop_stops_after_cancel() {
    let cancel = Arc::new(AtomicBool::new(false));
    let ticks = Arc::new(AtomicUsize::new(0));
    let ticks_clone = ticks.clone();
    let cancel_clone = cancel.clone();
    let handle = tokio::spawn(async move {
        run_poller_loop_with(cancel_clone, Duration::from_millis(20), || {
            let ticks = ticks_clone.clone();
            async move {
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        })
        .await;
    });
    tokio::time::sleep(Duration::from_millis(70)).await;
    cancel.store(true, Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(60)).await;
    let after_cancel = ticks.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(ticks.load(Ordering::SeqCst), after_cancel);
    assert!(after_cancel >= 1, "loop should have ticked before cancel");
    handle.abort();
    assert_eq!(SCHEDULE_POLLER_INTERVAL, Duration::from_secs(15));
}

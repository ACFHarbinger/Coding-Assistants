//! Moonshot Kimi Code CLI transcript capture adapter (C14.14, #309).
//!
//! Thin Tauri-side shell: the parsing rules and the swarm-aware
//! (#363) multi-wire capture live in `hub::bridge::kimi_wire` so they are
//! unit-testable under `cargo test -p hub --lib` (this crate's tests need
//! desktop system libraries to link). What stays here is the
//! capture-identity gate (`super::resolve_capture_session_id`), the
//! sessions-root resolution, and the #84 `usage.record` spend pass on the
//! main wire.
//!
//! Reads the on-disk Kimi wire event logs:
//! `<kimi-sessions-root>/wd_<slug>_<hash>/session_<uuid>/agents/<name>/wire.jsonl`
//! — `main` for the primary loop plus one sibling per swarm subagent.
//! Subagent captures record under `kimi-<worker>`; the main loop keeps
//! recording under `kimi` exactly as #309 did.

use hub::HubStore;
use std::path::Path;

pub use hub::{kimi_session_log_path, KimiCaptureOutcome};

/// Resolve the target session (explicit id or the registered one) and
/// capture every wire log of that session into the Hub transcript.
pub fn capture_kimi_session(
    store: &HubStore,
    workspace: &Path,
    kimi_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<KimiCaptureOutcome, String> {
    let Some(session_id) =
        super::resolve_capture_session_id(store, "kimi", workspace, kimi_session_id)?
    else {
        return Ok(KimiCaptureOutcome {
            transcript_found: false,
            scanned: 0,
            captured: Vec::new(),
        });
    };
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    capture_kimi_session_from(
        &hub::kimi_sessions_root(),
        store,
        &workspace,
        Some(&session_id),
        hub_session_id,
    )
}

/// Text capture is swarm-aware in `hub::capture_kimi_session_from`.
/// Spend stays on the main wire: `usage.record` is not chat-text dedup,
/// and a repeated poll of the same line is a no-op via `usage_key` (#84).
pub fn capture_kimi_session_from(
    sessions_root: &Path,
    store: &HubStore,
    workspace: &Path,
    kimi_session_id: Option<&str>,
    hub_session_id: Option<&str>,
) -> Result<KimiCaptureOutcome, String> {
    let outcome = hub::capture_kimi_session_from(
        sessions_root,
        store,
        workspace,
        kimi_session_id,
        hub_session_id,
    )?;
    if outcome.transcript_found {
        let session_id = kimi_session_id.unwrap_or("");
        if let Some(path) = kimi_session_log_path(sessions_root, session_id) {
            for turn in super::kimi_spend::kimi_measured_turns(&path, 2000, Some(session_id)) {
                super::spend::record_harness_turn_usage(store, hub_session_id, &turn);
            }
        }
    }
    Ok(outcome)
}

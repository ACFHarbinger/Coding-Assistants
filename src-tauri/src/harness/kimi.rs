//! Moonshot Kimi Code CLI transcript capture adapter (C14.14, #309).
//!
//! Thin Tauri-side shell: the parsing rules and the swarm-aware
//! (#363) multi-wire capture live in `hub::bridge::kimi_wire` so they are
//! unit-testable under `cargo test -p hub --lib` (this crate's tests need
//! desktop system libraries to link). What stays here is only the
//! capture-identity gate (`super::resolve_capture_session_id`) and the
//! sessions-root resolution.
//!
//! Reads the on-disk Kimi wire event logs:
//! `<kimi-sessions-root>/wd_<slug>_<hash>/session_<uuid>/agents/<name>/wire.jsonl`
//! — `main` for the primary loop plus one sibling per swarm subagent.
//! Subagent captures record under `kimi-<worker>`; the main loop keeps
//! recording under `kimi` exactly as #309 did.

use hub::HubStore;
use std::path::Path;

pub use hub::{capture_kimi_session_from, kimi_session_log_path, KimiCaptureOutcome};

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

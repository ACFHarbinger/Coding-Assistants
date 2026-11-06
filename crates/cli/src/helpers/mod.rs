use hub::HubStore;
use notify::EventKind;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub(crate) fn tagged_dispatch_workspace(
    dispatch: bool,
    workspace: Option<&str>,
) -> anyhow::Result<Option<PathBuf>> {
    if !dispatch {
        return Ok(None);
    }
    let workspace =
        PathBuf::from(workspace.ok_or_else(|| anyhow::anyhow!("--dispatch requires --workspace"))?);
    if !workspace.is_absolute() {
        anyhow::bail!("--dispatch requires an absolute --workspace path");
    }
    Ok(Some(workspace))
}

pub(crate) fn audit_operation(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Create(_) => "created",
        EventKind::Remove(_) => "removed",
        EventKind::Modify(_) => "modified",
        EventKind::Access(_) => "accessed",
        EventKind::Other => "other",
        EventKind::Any => "other",
    }
}

pub(crate) fn audit_file_hash(path: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let digest = Sha256::digest(bytes);
    Some(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(crate) fn audit_process_context() -> String {
    let exe = std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string());
    serde_json::json!({
        "pid": std::process::id(),
        "exe": exe,
        "cmdline": std::env::args().collect::<Vec<_>>(),
        "attribution": "observer_process; originating_writer_not_available_in_user_space_notify"
    })
    .to_string()
}

// --- C12: `ca harness capture` ---------------------------------------------
//
// The desktop app's periodic refresh polls each harness's real capture
// adapter, which lives in `src-tauri/src/harness_*.rs` (a different crate
// this CLI does not and should not depend on). To make C13's "hub-native run
// without the desktop app" requirement possible, this re-implements the same
// four on-disk transcript formats independently here, against the shared
// `hub::HubStore::record_harness_capture` dedup path — so a headless `ca
// harness capture` run and the desktop's poll converge on the same durable
// state even though they don't share code across the crate boundary.
pub(crate) fn default_home() -> PathBuf {
    hub::default_hub_home()
}

/// CA-106/CA-109: only Harbinger may edit/delete a chat message, mirroring
/// the desktop `require_human_authored` check in
/// `src-tauri/src/hub/commands.rs`.
/// Checked against both the caller-supplied `--from` and the message's
/// actual `from_agent`, since only the latter is authoritative.
pub(crate) fn require_human_authored(
    store: &HubStore,
    from: &str,
    message_id: &str,
) -> anyhow::Result<()> {
    if from != "human" {
        anyhow::bail!("only Harbinger (--from human) may edit or delete a chat message");
    }
    let message = store
        .get_message(message_id)?
        .ok_or_else(|| anyhow::anyhow!("message not found: {message_id}"))?;
    if message.from_agent != "human" {
        anyhow::bail!("only Harbinger may edit or delete a chat message");
    }
    Ok(())
}

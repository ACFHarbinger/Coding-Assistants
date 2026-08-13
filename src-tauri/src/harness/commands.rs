//! C12 Tauri surface for harness start / inject / capture.
//! Claude and Gemini implement their adapters in sibling files; this file
//! only dispatches through the shared `hub` contract.

use crate::hub::commands::store::open_store;
use hub::{
    default_leader_socket, inject_harness_with_store, latest_grok_session_id, start_harness,
    HarnessInjectRequest, HarnessInjectResult, HarnessSessionRegistration, HarnessStartRequest,
    HarnessStartResult, MessageRecord, SandboxStrictness, SettingsStore,
};
use std::path::PathBuf;

/// S5 / #131: `vibe` unconditionally passes `--trust`/`--auto-approve`
/// (`crates/hub/src/harness/mod.rs::vibe_spawn_args`) — the one harness
/// identity that cannot run without bypassing approval. Strict sandbox
/// policy for the target workspace refuses to start or inject it; Standard
/// and Permissive are unchanged from today's behavior. This gates at the
/// shared C12 dispatch boundary rather than the adapter itself, so no
/// harness adapter file is touched.
fn sandbox_strictness_blocks(harness: &str, workspace: &str) -> bool {
    let strictness = SettingsStore::open(hub::default_hub_home())
        .effective(Some(workspace))
        .orchestration
        .sandbox_strictness;
    harness == "vibe" && strictness == SandboxStrictness::Strict
}

#[tauri::command]
pub fn hub_start_harness(
    harness: String,
    workspace: String,
    session_id: Option<String>,
    prompt: String,
) -> Result<HarnessStartResult, String> {
    if sandbox_strictness_blocks(&harness, &workspace) {
        return Err(format!(
            "{harness} requires bypassing approval and is blocked by this workspace's strict sandbox policy"
        ));
    }
    start_harness(&HarnessStartRequest {
        harness,
        workspace: PathBuf::from(workspace),
        session_id,
        prompt,
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_inject_harness(
    harness: String,
    workspace: String,
    session_id: Option<String>,
    message_id: Option<String>,
    body: String,
    is_task: bool,
    is_wake: bool,
) -> Result<HarnessInjectResult, String> {
    if sandbox_strictness_blocks(&harness, &workspace) {
        return Err(format!(
            "{harness} requires bypassing approval and is blocked by this workspace's strict sandbox policy"
        ));
    }
    inject_harness_with_store(
        &open_store()?,
        &HarnessInjectRequest {
            harness,
            workspace: PathBuf::from(workspace),
            session_id,
            message_id,
            body,
            is_task,
            is_wake,
        },
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_register_harness_session(
    harness: String,
    workspace: String,
    disk_session_id: Option<String>,
    leader_socket: Option<String>,
) -> Result<HarnessSessionRegistration, String> {
    let workspace_path = PathBuf::from(&workspace);
    let session_id = match disk_session_id.filter(|id| !id.trim().is_empty()) {
        Some(id) => id,
        None if harness == "grok" => latest_grok_session_id(&workspace_path).ok_or_else(|| {
            "no on-disk Grok session for this workspace; pass diskSessionId".to_string()
        })?,
        None => {
            return Err("diskSessionId is required unless Grok can infer the latest session".into())
        }
    };
    let socket = leader_socket.or_else(|| {
        let path = default_leader_socket();
        path.exists().then(|| path.display().to_string())
    });
    open_store()?
        .register_harness_session(&harness, &workspace, &session_id, socket.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_harness_sessions() -> Result<Vec<HarnessSessionRegistration>, String> {
    open_store()?
        .list_harness_sessions()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_capture_grok_session(
    workspace: String,
    grok_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::grok::GrokCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness::grok::capture_grok_session(
        &store,
        &PathBuf::from(workspace),
        grok_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[tauri::command]
pub fn hub_capture_claude_session(
    workspace: String,
    claude_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::claude::ClaudeCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness::claude::capture_claude_session(
        &store,
        &PathBuf::from(workspace),
        claude_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[tauri::command]
pub fn hub_capture_codex_session(
    workspace: String,
    codex_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::codex::CodexCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness::codex::capture_codex_session(
        &store,
        &PathBuf::from(workspace),
        codex_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[tauri::command]
pub fn hub_capture_gemini_session(
    workspace: String,
    gemini_session_id: Option<String>,
    hub_session_id: Option<String>,
) -> Result<crate::harness::gemini::GeminiCaptureOutcome, String> {
    let store = open_store()?;
    crate::harness::gemini::capture_gemini_session(
        &store,
        &PathBuf::from(workspace),
        gemini_session_id.as_deref(),
        hub_session_id.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::commands::tests::CA_HOME_ENV_LOCK;
    use hub::SettingsStore;

    fn with_ca_home<T>(prefix: &str, run: impl FnOnce() -> T) -> T {
        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "tauri-harness-{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::env::set_var("CA_HOME", &dir);
        let result = run();
        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
        result
    }

    #[test]
    fn strict_sandbox_policy_blocks_only_vibe() {
        with_ca_home("strict-blocks-vibe", || {
            let mut settings = SettingsStore::open(hub::default_hub_home());
            settings
                .set_sandbox_strictness(SandboxStrictness::Strict)
                .unwrap();
            settings.save().unwrap();

            assert!(sandbox_strictness_blocks("vibe", "/abs/repo"));
            assert!(!sandbox_strictness_blocks("claude", "/abs/repo"));
            assert!(!sandbox_strictness_blocks("grok", "/abs/repo"));
        });
    }

    #[test]
    fn standard_and_permissive_sandbox_policy_never_block() {
        with_ca_home("standard-allows-vibe", || {
            // Default policy is Standard.
            assert!(!sandbox_strictness_blocks("vibe", "/abs/repo"));

            let mut settings = SettingsStore::open(hub::default_hub_home());
            settings
                .set_sandbox_strictness(SandboxStrictness::Permissive)
                .unwrap();
            settings.save().unwrap();
            assert!(!sandbox_strictness_blocks("vibe", "/abs/repo"));
        });
    }

    #[test]
    fn workspace_override_can_relax_strict_global_policy_for_vibe() {
        with_ca_home("workspace-override-relaxes-strict", || {
            let mut settings = SettingsStore::open(hub::default_hub_home());
            settings
                .set_sandbox_strictness(SandboxStrictness::Strict)
                .unwrap();
            settings
                .set_workspace_sandbox_strictness("/abs/relaxed", SandboxStrictness::Permissive)
                .unwrap();
            settings.save().unwrap();

            assert!(sandbox_strictness_blocks("vibe", "/abs/other-repo"));
            assert!(!sandbox_strictness_blocks("vibe", "/abs/relaxed"));
        });
    }

    #[test]
    fn hub_start_harness_rejects_vibe_under_strict_policy_before_spawning() {
        with_ca_home("start-harness-rejects-vibe", || {
            let mut settings = SettingsStore::open(hub::default_hub_home());
            settings
                .set_sandbox_strictness(SandboxStrictness::Strict)
                .unwrap();
            settings.save().unwrap();

            let error = hub_start_harness(
                "vibe".into(),
                "/abs/repo".into(),
                None,
                "do something".into(),
            )
            .expect_err("strict policy must reject vibe before it ever spawns");
            assert!(error.contains("strict sandbox policy"), "{error}");
        });
    }
}

#[tauri::command]
pub fn hub_record_harness_capture(
    harness: String,
    agent_id: String,
    session_id: Option<String>,
    body: String,
    workspace: Option<String>,
) -> Result<Option<MessageRecord>, String> {
    open_store()?
        .record_harness_capture(
            &harness,
            &agent_id,
            session_id.as_deref(),
            &body,
            workspace.as_deref(),
        )
        .map_err(|error| error.to_string())
}

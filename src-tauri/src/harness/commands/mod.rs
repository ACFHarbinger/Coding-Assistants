//! C12 Tauri surface for harness start / inject / capture.
//! Claude and Gemini implement their adapters in sibling files; this file
//! only dispatches through the shared `hub` contract.

use crate::commands::commands::store::open_store;
use hub::{
    connect_grok_leader_session, default_leader_socket, delete_channel_workspace,
    grok_leader_status, inject_harness_with_store, is_channel_session_live, latest_grok_session_id,
    launch_claude_channel_session, list_active_grok_sessions, list_channel_workspaces,
    rename_channel_workspace, start_harness, ActiveGrokSession, ChannelWorkspace,
    GrokConnectResult, HarnessInjectRequest, HarnessInjectResult, HarnessSessionRegistration,
    HarnessStartRequest, HarnessStartResult, MessageRecord, SandboxStrictness, SettingsStore,
};
use std::path::{Path, PathBuf};

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

/// Relaunch/managed-harness commands (see `relaunch.rs`).
pub mod relaunch;

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

/// Mark a session as Hub-owned. Discovery stays observed via
/// `hub_register_harness_session`. Does not write a provider transport.
#[tauri::command]
pub fn hub_register_managed_harness_session(
    harness: String,
    workspace: String,
    disk_session_id: String,
    managed_pid: u32,
) -> Result<HarnessSessionRegistration, String> {
    let mut effective_session_id = disk_session_id;
    if (harness == "gemini" || harness == "agy")
        && (effective_session_id.trim().is_empty() || effective_session_id == "general")
    {
        if let Some(inferred) = hub::latest_gemini_session_id(&PathBuf::from(&workspace)) {
            effective_session_id = inferred;
        }
    }
    open_store()?
        .register_managed_harness_session(&harness, &workspace, &effective_session_id, managed_pid)
        .map_err(|error| error.to_string())
}

/// C14.3: every workspace previously configured with `--setup` for the
/// Claude Channel bridge (`crates/claude`), for a Shared Hub management
/// panel. Pure filesystem scan over `~/.coding-assistants/servers/` — see
/// `hub::list_channel_workspaces`.
#[tauri::command]
pub fn claude_channel_list_workspaces() -> Result<Vec<ChannelWorkspace>, String> {
    list_channel_workspaces(&open_store()?).map_err(|error| error.to_string())
}

/// Updates only the cosmetic display name; the workspace path and the
/// Hub-managed session registration are unaffected.
#[tauri::command]
pub fn claude_channel_rename_workspace(workspace: String, name: String) -> Result<(), String> {
    rename_channel_workspace(&open_store()?, Path::new(&workspace), &name)
        .map_err(|error| error.to_string())
}

/// Removes the canonical Channel config and downgrades the Hub
/// registration back to `observed`. Does not touch the workspace's own
/// `.mcp.json` — see `hub::delete_channel_workspace`.
#[tauri::command]
pub fn claude_channel_delete_workspace(workspace: String) -> Result<(), String> {
    delete_channel_workspace(&open_store()?, Path::new(&workspace))
        .map_err(|error| error.to_string())
}

/// Whether a live Claude Code session already has the Channel bridge
/// loaded for `workspace` — see `hub::is_channel_session_live`. The
/// Shared Hub Channels tab polls this to show a connected/not-connected
/// status per configured workspace.
///
/// Async + `spawn_blocking`: `is_channel_session_live` shells out to
/// `claude agents --json`, and a sync `#[tauri::command]` runs that
/// subprocess call inline on the same thread that dispatches IPC, which
/// can stall the whole window (see `hub_get_provider_quotas`'s doc comment
/// for the confirmed live repro of this class of bug). The Channels tab
/// calls this once per configured workspace on every load.
#[tauri::command]
pub async fn claude_channel_is_connected(workspace: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || is_channel_session_live(Path::new(&workspace)))
        .await
        .map_err(|error| format!("channel liveness check task panicked: {error}"))?
}

/// Launches a terminal running `claude` with the Channel bridge loaded for
/// `workspace`, for when no live session is already connected. Claude
/// Code's Channel research preview is an interactive TUI with no headless
/// daemon mode, so this always opens a real terminal window rather than a
/// detached background process — see `hub::launch_claude_channel_session`.
#[tauri::command]
pub fn claude_channel_connect(workspace: String) -> Result<(), String> {
    launch_claude_channel_session(Path::new(&workspace)).map(|_| ())
}

/// Whether `~/.grok/leader.sock` (or `$GROK_LEADER_SOCKET`) exists, plus
/// any live Grok TUI listed for `workspace`.
#[tauri::command]
pub fn hub_grok_leader_status(workspace: Option<String>) -> GrokConnectResult {
    grok_leader_status(workspace.as_deref().map(Path::new))
}

#[tauri::command]
pub fn hub_grok_list_live_sessions() -> Vec<ActiveGrokSession> {
    list_active_grok_sessions()
}

/// Start or attach to a documented Grok leader for `workspace`.
/// `resume` opens `grok --leader --resume <live-or-latest session>`.
#[tauri::command]
pub fn hub_grok_connect(workspace: String, resume: bool) -> Result<GrokConnectResult, String> {
    connect_grok_leader_session(&open_store()?, Path::new(&workspace), resume)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::commands::tests::CA_HOME_ENV_LOCK;
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

    #[test]
    fn hub_register_managed_session_is_distinct_from_observed() {
        with_ca_home("register-managed-vs-observed", || {
            let observed = hub_register_harness_session(
                "chat".into(),
                "/abs/repo".into(),
                Some("thread-observed".into()),
                None,
            )
            .expect("observed registration");
            assert_eq!(observed.mode.as_str(), "observed");
            assert!(observed.managed_pid.is_none());

            let managed = hub_register_managed_harness_session(
                "gemini".into(),
                "/abs/repo".into(),
                "conv-owned".into(),
                4242,
            )
            .expect("managed registration");
            assert_eq!(managed.mode.as_str(), "managed");
            assert_eq!(managed.managed_pid, Some(4242));
            assert_eq!(managed.state.as_str(), "ready");

            let listed = hub_list_harness_sessions().expect("list");
            assert!(listed
                .iter()
                .any(|row| row.harness == "chat" && row.mode.as_str() == "observed"));
            assert!(listed.iter().any(|row| {
                row.harness == "gemini"
                    && row.mode.as_str() == "managed"
                    && row.managed_pid == Some(4242)
            }));
        });
    }

    #[test]
    fn hub_register_managed_session_rejects_relative_workspace() {
        with_ca_home("register-managed-relative", || {
            let error = hub_register_managed_harness_session(
                "chat".into(),
                "relative/repo".into(),
                "thread-1".into(),
                7,
            )
            .expect_err("relative workspace must be rejected");
            assert!(error.contains("absolute"), "unexpected error: {error}");
        });
    }

    #[test]
    fn hub_inject_harness_rejects_vibe_under_strict_policy_before_delivery() {
        with_ca_home("inject-harness-rejects-vibe", || {
            let mut settings = SettingsStore::open(hub::default_hub_home());
            settings
                .set_sandbox_strictness(SandboxStrictness::Strict)
                .unwrap();
            settings.save().unwrap();

            let error = hub_inject_harness(
                "vibe".into(),
                "/abs/repo".into(),
                None,
                None,
                "do something".into(),
                false,
                false,
            )
            .expect_err("strict policy must reject vibe before delivery");
            assert!(error.contains("strict sandbox policy"), "{error}");
        });
    }

    #[test]
    fn claude_channel_workspace_commands_list_rename_and_delete() {
        with_ca_home("claude-channel-workspace-mgmt", || {
            let dir = tempfile::tempdir().unwrap();
            let workspace = dir.path().join("repo");
            std::fs::create_dir_all(&workspace).unwrap();
            let workspace = workspace.to_string_lossy().to_string();

            hub::setup_claude_channel(
                &open_store().unwrap(),
                std::path::Path::new(&workspace),
                std::path::Path::new("bridge"),
            )
            .expect("setup");

            let listed = claude_channel_list_workspaces().expect("list");
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].workspace, workspace);

            claude_channel_rename_workspace(workspace.clone(), "My Repo".into()).expect("rename");
            let renamed = claude_channel_list_workspaces().expect("list after rename");
            assert_eq!(renamed[0].display_name, "My Repo");

            claude_channel_delete_workspace(workspace).expect("delete");
            assert!(claude_channel_list_workspaces()
                .expect("list after delete")
                .is_empty());
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

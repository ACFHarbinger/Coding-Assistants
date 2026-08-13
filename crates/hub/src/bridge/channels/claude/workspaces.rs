//! The app-owned `~/.coding-assistants/servers/` Channel config registry:
//! setup (writes the canonical per-workspace file plus the merged
//! `.mcp.json`), and list/rename/delete for owner management (a Shared Hub
//! panel).

use super::CLAUDE_AGENT_ID;
use crate::{HubError, HubStore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const SERVERS_SUBDIR: &str = "servers";
const GLOBAL_SERVERS_FILE: &str = "global.mcp.json";
const CHANNEL_SERVER_KEY: &str = "coding-assistants-channel";

/// A previously-configured Channel workspace, for owner management
/// (list/rename/delete) — e.g. a Shared Hub panel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChannelWorkspace {
    pub workspace: String,
    pub display_name: String,
}

/// Every Channel `.mcp.json` this app owns lives here — never inside a
/// repository except the one file Claude Code itself requires (written
/// separately by the `claude-channel` binary's `--setup`). This directory
/// is the durable, app-owned record: it survives a repo being deleted or
/// re-cloned, and lets an owner list/rename/delete configured workspaces
/// without hunting through project directories.
///
/// Derived from `store.data_dir()` rather than the process-global
/// `default_hub_home()` — the same fix applied to the S5 orchestration
/// policy lookup — so a `HubStore` opened at a non-default path (every
/// test, and any future caller with a custom data dir) keeps its Channel
/// config alongside its own data instead of the host machine's real home.
pub fn servers_dir(store: &HubStore) -> PathBuf {
    store.data_dir().join(SERVERS_SUBDIR)
}

/// Servers merged as a base layer into *every* workspace's generated
/// config. Empty (`{"mcpServers": {}}`) until the owner adds something.
pub fn global_servers_path(store: &HubStore) -> PathBuf {
    servers_dir(store).join(GLOBAL_SERVERS_FILE)
}

/// Deterministic, collision-proof file stem for a workspace: its
/// directory name plus a short stable hash of the full absolute path, so
/// two differently-located repos that happen to share a directory name
/// never collide.
pub fn workspace_server_name(workspace: &Path) -> String {
    let base = workspace
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "workspace".to_string());
    let digest = Sha256::digest(workspace.to_string_lossy().as_bytes());
    let short: String = digest
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{base}-{short}")
}

/// The canonical, app-owned copy of one workspace's Channel config.
pub fn workspace_servers_path(store: &HubStore, workspace: &Path) -> PathBuf {
    servers_dir(store).join(format!("{}.mcp.json", workspace_server_name(workspace)))
}

fn read_mcp_config(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({ "mcpServers": {} }))
}

/// Merges `addition`'s `mcpServers` entries on top of `base`'s, keeping
/// every other server `base` already defines.
fn merge_mcp_configs(base: &Value, addition: &Value) -> Value {
    let mut merged = base.clone();
    if !merged.get("mcpServers").is_some_and(Value::is_object) {
        merged["mcpServers"] = json!({});
    }
    if let (Some(target), Some(source)) = (
        merged["mcpServers"].as_object_mut(),
        addition.get("mcpServers").and_then(Value::as_object),
    ) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    merged
}

fn channel_server_entry(bridge_binary: &Path, workspace: &Path) -> Value {
    json!({
        "mcpServers": {
            CHANNEL_SERVER_KEY: {
                "command": bridge_binary.to_string_lossy(),
                "args": ["--workspace", workspace.to_string_lossy()],
            }
        }
    })
}

/// Opt-in setup: registers `claude` as a Hub-managed harness session for
/// `workspace` (so the existing C14.1 single-writer lease applies to it
/// like any other managed provider), writes the canonical per-workspace
/// copy under [`servers_dir`], and returns the *effective* config (this
/// workspace's entry merged on top of [`global_servers_path`]'s base
/// layer) for the caller to write into the workspace's own `.mcp.json` —
/// the one file Claude Code actually reads. This never touches an
/// *existing*, non-opted-in Claude session; registration is a separate,
/// deliberate action the owner takes per workspace.
pub fn setup_claude_channel(
    store: &HubStore,
    workspace: &Path,
    bridge_binary: &Path,
) -> Result<Value, HubError> {
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "Claude Channel setup requires an absolute workspace".into(),
        ));
    }
    let label = format!("channel:{}", chrono::Utc::now().timestamp());
    store.register_managed_harness_session(
        CLAUDE_AGENT_ID,
        &workspace.to_string_lossy(),
        &label,
        std::process::id(),
    )?;

    std::fs::create_dir_all(servers_dir(store))?;
    let global_path = global_servers_path(store);
    if !global_path.exists() {
        std::fs::write(
            &global_path,
            serde_json::to_string_pretty(&json!({ "mcpServers": {} }))
                .expect("serialize empty config")
                + "\n",
        )?;
    }

    let mut canonical = channel_server_entry(bridge_binary, workspace);
    canonical["_workspace"] = json!(workspace.to_string_lossy());
    canonical["_display_name"] = json!(default_display_name(workspace));
    std::fs::write(
        workspace_servers_path(store, workspace),
        serde_json::to_string_pretty(&canonical).map_err(|e| HubError::Invalid(e.to_string()))?
            + "\n",
    )?;

    let global = read_mcp_config(&global_path);
    Ok(merge_mcp_configs(&global, &canonical))
}

fn default_display_name(workspace: &Path) -> String {
    workspace
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| workspace.to_string_lossy().to_string())
}

/// Every workspace previously configured for a Channel bridge. Pure
/// filesystem scan over [`servers_dir`] — the canonical files are the
/// durable record.
pub fn list_channel_workspaces(store: &HubStore) -> Result<Vec<ChannelWorkspace>, HubError> {
    let dir = servers_dir(store);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut workspaces = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(GLOBAL_SERVERS_FILE) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let config = read_mcp_config(&path);
        let Some(workspace) = config.get("_workspace").and_then(Value::as_str) else {
            continue;
        };
        let display_name = config
            .get("_display_name")
            .and_then(Value::as_str)
            .unwrap_or(workspace)
            .to_string();
        workspaces.push(ChannelWorkspace {
            workspace: workspace.to_string(),
            display_name,
        });
    }
    workspaces.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(workspaces)
}

/// Updates only the cosmetic display name of an already-configured
/// workspace; the canonical filename, the workspace path, and the Hub
/// registration are unaffected.
pub fn rename_channel_workspace(
    store: &HubStore,
    workspace: &Path,
    display_name: &str,
) -> Result<(), HubError> {
    let display_name = display_name.trim();
    if display_name.is_empty() {
        return Err(HubError::Invalid(
            "Channel workspace display name must not be empty".into(),
        ));
    }
    let path = workspace_servers_path(store, workspace);
    if !path.exists() {
        return Err(HubError::NotFound(format!(
            "no Channel configuration for {}",
            workspace.display()
        )));
    }
    let mut config = read_mcp_config(&path);
    config["_display_name"] = json!(display_name);
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&config).map_err(|e| HubError::Invalid(e.to_string()))? + "\n",
    )?;
    Ok(())
}

/// Removes the canonical Channel config for `workspace` and downgrades
/// its Hub registration back to `observed` (the same state a purely
/// discovered, non-managed session has) — it does not delete the
/// workspace's own `.mcp.json`; the owner (or a future setup run) manages
/// that file directly.
pub fn delete_channel_workspace(store: &HubStore, workspace: &Path) -> Result<(), HubError> {
    let path = workspace_servers_path(store, workspace);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    if let Some(registration) =
        store.get_harness_session(CLAUDE_AGENT_ID, &workspace.to_string_lossy())?
    {
        store.register_harness_session(
            CLAUDE_AGENT_ID,
            &workspace.to_string_lossy(),
            &registration.disk_session_id,
            registration.leader_socket.as_deref(),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn setup_registers_a_managed_session_and_returns_mcp_config() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        let bridge = Path::new("/usr/local/bin/claude-channel");

        let config = setup_claude_channel(&store, &workspace, bridge).unwrap();
        assert_eq!(
            config["mcpServers"]["coding-assistants-channel"]["command"],
            bridge.to_string_lossy().to_string()
        );

        let registration = store
            .get_harness_session("claude", &workspace.to_string_lossy())
            .unwrap()
            .expect("registered");
        assert_eq!(registration.mode, crate::HarnessSessionMode::Managed);
    }

    #[test]
    fn setup_rejects_a_relative_workspace() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let bridge = Path::new("claude-channel");
        assert!(setup_claude_channel(&store, Path::new("relative/path"), bridge).is_err());
    }

    #[test]
    fn setup_writes_a_canonical_copy_and_an_empty_global_base() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        let bridge = Path::new("/usr/local/bin/coding-assistants-claude-channel");

        setup_claude_channel(&store, &workspace, bridge).unwrap();

        assert!(global_servers_path(&store).exists());
        let canonical_path = workspace_servers_path(&store, &workspace);
        assert!(canonical_path.exists());
        let canonical = read_mcp_config(&canonical_path);
        assert_eq!(canonical["_workspace"], json!(workspace.to_string_lossy()));
        assert_eq!(
            canonical["mcpServers"][CHANNEL_SERVER_KEY]["command"],
            bridge.to_string_lossy().to_string()
        );
    }

    #[test]
    fn setup_merges_the_global_base_layer_into_the_effective_config() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(servers_dir(&store)).unwrap();
        std::fs::write(
            global_servers_path(&store),
            json!({ "mcpServers": { "shared-tool": { "command": "shared" } } }).to_string(),
        )
        .unwrap();

        let effective = setup_claude_channel(&store, &workspace, Path::new("bridge")).unwrap();
        assert_eq!(effective["mcpServers"]["shared-tool"]["command"], "shared");
        assert_eq!(
            effective["mcpServers"][CHANNEL_SERVER_KEY]["command"],
            "bridge"
        );
    }

    #[test]
    fn distinct_workspaces_sharing_a_directory_name_do_not_collide() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let a = dir.path().join("a/repo");
        let b = dir.path().join("b/repo");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();

        setup_claude_channel(&store, &a, Path::new("bridge")).unwrap();
        setup_claude_channel(&store, &b, Path::new("bridge")).unwrap();

        assert_ne!(
            workspace_servers_path(&store, &a),
            workspace_servers_path(&store, &b)
        );
        let workspaces = list_channel_workspaces(&store).unwrap();
        assert_eq!(workspaces.len(), 2);
    }

    #[test]
    fn list_rename_and_delete_manage_the_registry() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        setup_claude_channel(&store, &workspace, Path::new("bridge")).unwrap();

        let workspaces = list_channel_workspaces(&store).unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].workspace, workspace.to_string_lossy());
        assert_eq!(workspaces[0].display_name, "repo");

        rename_channel_workspace(&store, &workspace, "My Repo").unwrap();
        let renamed = list_channel_workspaces(&store).unwrap();
        assert_eq!(renamed[0].display_name, "My Repo");

        delete_channel_workspace(&store, &workspace).unwrap();
        assert!(list_channel_workspaces(&store).unwrap().is_empty());
        let registration = store
            .get_harness_session(CLAUDE_AGENT_ID, &workspace.to_string_lossy())
            .unwrap()
            .expect("registration record survives, downgraded");
        assert_eq!(registration.mode, crate::HarnessSessionMode::Observed);
    }

    #[test]
    fn rename_rejects_an_empty_name_and_an_unconfigured_workspace() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let workspace = dir.path().join("repo");
        std::fs::create_dir_all(&workspace).unwrap();
        setup_claude_channel(&store, &workspace, Path::new("bridge")).unwrap();

        assert!(rename_channel_workspace(&store, &workspace, "   ").is_err());
        assert!(rename_channel_workspace(&store, Path::new("/never/configured"), "x").is_err());
    }

    #[test]
    fn list_channel_workspaces_is_empty_before_any_setup() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        assert!(list_channel_workspaces(&store).unwrap().is_empty());
    }
}

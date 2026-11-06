//! The `--setup` / `--list` / `--rename` / `--delete` management
//! subcommands and their shared workspace-argument parsing.

use hub::{
    delete_channel_workspace, list_channel_workspaces, rename_channel_workspace,
    setup_claude_channel, HubStore,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Every workspace previously configured for a Channel bridge — the same
/// registry a future Shared Hub panel reads through the Tauri commands in
/// `src-tauri/src/harness/commands.rs`.
pub fn run_list() {
    let store = HubStore::open(hub::default_hub_home()).expect("open Hub store");
    match list_channel_workspaces(&store) {
        Ok(workspaces) if workspaces.is_empty() => {
            println!("No workspaces configured yet. Run --setup --workspace <abs path> first.");
        }
        Ok(workspaces) => {
            for workspace in workspaces {
                println!("{}\t{}", workspace.display_name, workspace.workspace);
            }
        }
        Err(error) => {
            eprintln!("failed to list Channel workspaces: {error}");
            std::process::exit(1);
        }
    }
}

pub fn run_rename(args: &[String]) {
    let workspace = canonical_workspace_arg(args);
    let Some(name) = args
        .windows(2)
        .find(|pair| pair[0] == "--name")
        .map(|pair| pair[1].clone())
    else {
        eprintln!("--rename requires --workspace <abs path> --name <new display name>");
        std::process::exit(1);
    };
    let store = HubStore::open(hub::default_hub_home()).expect("open Hub store");
    if let Err(error) = rename_channel_workspace(&store, &workspace, &name) {
        eprintln!("failed to rename: {error}");
        std::process::exit(1);
    }
    println!("Renamed {} to \"{name}\".", workspace.display());
}

pub fn run_delete(args: &[String]) {
    let workspace = canonical_workspace_arg(args);
    let store = HubStore::open(hub::default_hub_home()).expect("open Hub store");
    if let Err(error) = delete_channel_workspace(&store, &workspace) {
        eprintln!("failed to delete: {error}");
        std::process::exit(1);
    }
    println!(
        "Removed the Channel configuration for {}. Its own .mcp.json is untouched \
         — remove the \"coding-assistants-channel\" entry there yourself if you no longer want it.",
        workspace.display()
    );
}

pub fn workspace_arg(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|pair| pair[0] == "--workspace")
        .map(|pair| PathBuf::from(&pair[1]))
        .unwrap_or_else(|| std::env::current_dir().expect("current directory"))
}

/// Registrations are keyed by the canonicalized path (see `run_setup`), so
/// every command that looks one up must resolve the same way.
pub fn canonical_workspace_arg(args: &[String]) -> PathBuf {
    let requested = workspace_arg(args);
    requested.canonicalize().unwrap_or(requested)
}

pub fn run_setup(args: &[String]) {
    let workspace = canonical_workspace_arg(args);
    let store = HubStore::open(hub::default_hub_home()).expect("open Hub store");
    let bridge_binary = std::env::current_exe().expect("resolve this binary's own path");

    let config = match setup_claude_channel(&store, &workspace, &bridge_binary) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Claude Channel setup failed: {error}");
            std::process::exit(1);
        }
    };

    let mcp_path = workspace.join(".mcp.json");
    let merged = merge_mcp_config(&mcp_path, &config);
    let rendered = serde_json::to_string_pretty(&merged).expect("serialize .mcp.json");
    if let Err(error) = std::fs::write(&mcp_path, rendered + "\n") {
        eprintln!("failed to write {}: {error}", mcp_path.display());
        std::process::exit(1);
    }

    println!("Claude Channel configured for {}.", workspace.display());
    println!("Wrote {}.", mcp_path.display());
    println!("Start Claude Code in this workspace with: claude --channels");
    println!(
        "(research preview; requires Claude Code 2.1.231+, Anthropic authentication, and \
         --dangerously-load-development-channels until this bridge is allowlisted)"
    );
}

/// Merges the new `mcpServers` entry into an existing `.mcp.json` without
/// discarding any other server the owner already configured there.
fn merge_mcp_config(path: &Path, addition: &Value) -> Value {
    let mut existing: Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({ "mcpServers": {} }));
    if !existing.get("mcpServers").is_some_and(Value::is_object) {
        existing["mcpServers"] = json!({});
    }
    if let (Some(target), Some(source)) = (
        existing["mcpServers"].as_object_mut(),
        addition["mcpServers"].as_object(),
    ) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    existing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_arg_reads_the_flag() {
        let args = vec!["--workspace".to_string(), "/abs/repo".to_string()];
        assert_eq!(workspace_arg(&args), PathBuf::from("/abs/repo"));
    }

    #[test]
    fn workspace_arg_falls_back_to_cwd_when_absent() {
        let args: Vec<String> = vec![];
        assert_eq!(workspace_arg(&args), std::env::current_dir().unwrap());
    }

    #[test]
    fn merge_mcp_config_creates_a_fresh_file_when_none_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".mcp.json");
        let addition = json!({ "mcpServers": { "coding-assistants-channel": { "command": "x" } } });
        let merged = merge_mcp_config(&path, &addition);
        assert_eq!(
            merged["mcpServers"]["coding-assistants-channel"]["command"],
            "x"
        );
    }

    #[test]
    fn merge_mcp_config_preserves_other_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".mcp.json");
        std::fs::write(
            &path,
            json!({ "mcpServers": { "other-server": { "command": "y" } } }).to_string(),
        )
        .unwrap();
        let addition = json!({ "mcpServers": { "coding-assistants-channel": { "command": "x" } } });
        let merged = merge_mcp_config(&path, &addition);
        assert_eq!(merged["mcpServers"]["other-server"]["command"], "y");
        assert_eq!(
            merged["mcpServers"]["coding-assistants-channel"]["command"],
            "x"
        );
    }

    #[test]
    fn merge_mcp_config_overwrites_only_its_own_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".mcp.json");
        std::fs::write(
            &path,
            json!({ "mcpServers": { "coding-assistants-channel": { "command": "stale" } } })
                .to_string(),
        )
        .unwrap();
        let addition =
            json!({ "mcpServers": { "coding-assistants-channel": { "command": "fresh" } } });
        let merged = merge_mcp_config(&path, &addition);
        assert_eq!(
            merged["mcpServers"]["coding-assistants-channel"]["command"],
            "fresh"
        );
    }
}

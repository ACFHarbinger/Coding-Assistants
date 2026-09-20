//! Tauri commands for OS-level tool execution (`platform.md` P5 / #333).

use super::store::open_store;
use hub::os_tool::{self, OsToolRequest, OsToolResult};
use hub::{SandboxStrictness, SettingsStore};
use std::path::PathBuf;

fn require_absolute(path: &str, label: &str) -> Result<(), String> {
    if path.trim().is_empty() || !PathBuf::from(path).is_absolute() {
        return Err(format!("{label} must be an absolute path"));
    }
    Ok(())
}

fn effective_strictness(workspace: Option<&str>) -> SandboxStrictness {
    SettingsStore::open(hub::default_hub_home())
        .effective(workspace)
        .orchestration
        .sandbox_strictness
}

#[tauri::command]
pub fn hub_run_os_tool(
    program: String,
    args: Vec<String>,
    cwd: String,
    workspace: Option<String>,
) -> Result<OsToolResult, String> {
    require_absolute(&cwd, "cwd")?;
    if let Some(workspace) = workspace.as_deref() {
        require_absolute(workspace, "workspace")?;
    }
    let strictness = effective_strictness(workspace.as_deref());
    let request = OsToolRequest {
        program,
        args,
        cwd,
        workspace,
    };
    os_tool::submit(
        &open_store()?,
        request,
        strictness,
        os_tool::DEFAULT_TIMEOUT,
    )
}

#[tauri::command]
pub fn hub_approve_os_tool(audit_id: String) -> Result<OsToolResult, String> {
    os_tool::approve(&open_store()?, &audit_id, os_tool::DEFAULT_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub::HubStore;
    use tempfile::tempdir;

    #[test]
    fn echo_is_audited_under_standard_sandbox() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let cwd = dir.path().to_string_lossy().into_owned();
        let request = OsToolRequest {
            program: "echo".into(),
            args: vec!["p5-ok".into()],
            cwd: cwd.clone(),
            workspace: Some(cwd),
        };
        let result = os_tool::submit(
            &store,
            request,
            SandboxStrictness::Standard,
            os_tool::DEFAULT_TIMEOUT,
        )
        .unwrap();
        assert_eq!(result.status, "ran");
        assert!(result.stdout.contains("p5-ok"));
        let events = store.list_audit_events(false).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].operation, "os_tool.ran");
        assert_eq!(events[0].status, "approved");
        assert!(events[0].process_json.contains("p5-ok"));
    }

    #[test]
    fn bash_is_denied_and_quarantined() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let cwd = dir.path().to_string_lossy().into_owned();
        let request = OsToolRequest {
            program: "bash".into(),
            args: vec!["-c".into(), "echo pwned".into()],
            cwd: cwd.clone(),
            workspace: Some(cwd),
        };
        let result = os_tool::submit(
            &store,
            request,
            SandboxStrictness::Permissive,
            os_tool::DEFAULT_TIMEOUT,
        )
        .unwrap();
        assert_eq!(result.status, "denied");
        let events = store.list_audit_events(false).unwrap();
        assert_eq!(events[0].operation, "os_tool.denied");
        assert_eq!(events[0].status, "quarantined");
    }

    #[test]
    fn mutating_git_proposes_then_approve_runs() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let cwd = dir.path().to_string_lossy().into_owned();
        let request = OsToolRequest {
            program: "git".into(),
            args: vec!["commit".into(), "--help".into()],
            cwd: cwd.clone(),
            workspace: Some(cwd),
        };
        let proposed = os_tool::submit(
            &store,
            request,
            SandboxStrictness::Standard,
            os_tool::DEFAULT_TIMEOUT,
        )
        .unwrap();
        assert_eq!(proposed.status, "needs_approval");
        let ran = os_tool::approve(&store, &proposed.audit_id, os_tool::DEFAULT_TIMEOUT).unwrap();
        assert_eq!(ran.status, "ran");
        assert_eq!(ran.exit_code, Some(0));
    }
}

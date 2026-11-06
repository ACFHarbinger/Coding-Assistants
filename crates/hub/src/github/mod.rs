//! GitHub CLI (`gh`) helpers for U18/U19. Explicit argv, no shell strings.

mod branches;
mod project;
pub use branches::{
    attach_issue_metadata, collect_git_branches, fetch_issue_meta, list_workspace_branches,
    BranchList, GitBranch, IssueMeta, DEFAULT_ISSUE_REPO,
};
pub use project::{
    board_from_items, empty_project_board, list_project_board, parse_agent_issue_branch,
    set_project_item_status, BoardCard, BoardColumn, ProjectBoard, DEFAULT_BOARD_COLUMNS,
    DEFAULT_PROJECT_NUMBER, DEFAULT_PROJECT_OWNER,
};

use crate::HubError;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

const GH_TIMEOUT: Duration = Duration::from_secs(25);

pub(crate) fn gh_json(args: &[&str]) -> Result<Value, HubError> {
    gh_json_with(run_gh, args)
}

pub(crate) fn gh_json_with(
    runner: impl FnOnce(&[&str]) -> Result<String, String>,
    args: &[&str],
) -> Result<Value, HubError> {
    let stdout = runner(args).map_err(HubError::Invalid)?;
    serde_json::from_str(&stdout)
        .map_err(|error| HubError::Invalid(format!("gh JSON parse failed: {error}")))
}

/// Local + remote-tracking branches mapped to an issue number via
/// `agent/<name>-<issue>`.
pub fn list_issue_branches(workspace: &Path) -> Vec<(String, i64)> {
    if !workspace.is_absolute() {
        return Vec::new();
    }
    let output = Command::new("git")
        .args([
            "-C",
            &workspace.to_string_lossy(),
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads",
            "refs/remotes",
        ])
        .stdin(Stdio::null())
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let name = line.trim();
            let issue = parse_agent_issue_branch(name)?;
            Some((name.to_string(), issue))
        })
        .collect()
}

pub(crate) fn run_gh(args: &[&str]) -> Result<String, String> {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let output = Command::new("gh")
            .args(&owned)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        let _ = tx.send(output);
    });
    match rx.recv_timeout(GH_TIMEOUT) {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            if output.status.success() {
                Ok(stdout)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!(
                    "gh failed: {}",
                    stderr.trim().chars().take(240).collect::<String>()
                ))
            }
        }
        Ok(Err(error)) => Err(format!("gh unavailable: {error}")),
        Err(_) => Err("gh timed out".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_agent_issue_branch_reads_the_trailing_number() {
        assert_eq!(parse_agent_issue_branch("agent/grok-312"), Some(312));
        assert_eq!(parse_agent_issue_branch("origin/agent/grok-312"), Some(312));
        assert_eq!(parse_agent_issue_branch("agent/gemini-314-u20"), Some(314));
        assert_eq!(parse_agent_issue_branch("main"), None);
        assert_eq!(parse_agent_issue_branch("feat/mcp-ableton"), None);
    }

    #[test]
    fn gh_json_with_parses_a_fixture_payload() {
        let value =
            gh_json_with(|_| Ok(r#"{"items":[]}"#.into()), &["project", "item-list"]).unwrap();
        assert_eq!(value["items"].as_array().unwrap().len(), 0);
    }
}

//! Local + remote-tracking branch listing for U19 / #313.
//!
//! Uses explicit `git` argv (same style as [`super::list_issue_branches`]).
//! Issue title/status lookups go through [`super::gh_json`].

use super::{gh_json, parse_agent_issue_branch};
use crate::HubError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Stdio};

pub const DEFAULT_ISSUE_REPO: &str = "ACFHarbinger/Coding-Assistants";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitBranch {
    pub name: String,
    pub kind: String,
    pub tip: String,
    pub committed_at: i64,
    pub subject: String,
    pub ahead: i64,
    pub behind: i64,
    pub inferred_issue: Option<i64>,
    pub issue_number: Option<i64>,
    pub issue_title: Option<String>,
    pub issue_state: Option<String>,
    pub issue_url: Option<String>,
    pub override_issue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchList {
    pub branches: Vec<GitBranch>,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueMeta {
    pub title: String,
    pub state: String,
    pub url: String,
}

fn issue_repo() -> String {
    std::env::var("CA_GH_ISSUE_REPO")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_ISSUE_REPO.to_string())
}

fn git_stdout(workspace: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("git unavailable: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "git failed: {}",
            stderr.trim().chars().take(200).collect::<String>()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn resolve_base(workspace: &Path) -> Option<String> {
    for candidate in ["main", "master", "origin/main", "origin/master"] {
        if git_stdout(workspace, &["rev-parse", "--verify", "--quiet", candidate]).is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn ahead_behind(workspace: &Path, base: &str, branch: &str) -> (i64, i64) {
    if base == branch {
        return (0, 0);
    }
    let Ok(raw) = git_stdout(
        workspace,
        &[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{base}...{branch}"),
        ],
    ) else {
        return (0, 0);
    };
    let mut parts = raw.split_whitespace();
    let behind = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let ahead = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (ahead, behind)
}

/// Collect local + remote-tracking refs. No GitHub calls.
pub fn collect_git_branches(workspace: &Path) -> Result<Vec<GitBranch>, HubError> {
    if !workspace.is_absolute() {
        return Err(HubError::Invalid(
            "workspace must be an absolute path".into(),
        ));
    }
    let raw = git_stdout(
        workspace,
        &[
            "for-each-ref",
            "--format=%(refname)%01%(refname:short)%01%(objectname:short)%01%(committerdate:unix)%01%(contents:subject)",
            "refs/heads",
            "refs/remotes",
        ],
    )
    .map_err(HubError::Invalid)?;
    let base = resolve_base(workspace);
    let mut branches = Vec::new();
    for line in raw.lines() {
        let mut cols = line.split('\u{1}');
        let Some(refname) = cols.next() else { continue };
        let Some(name) = cols.next() else { continue };
        if name.is_empty() || name.ends_with("/HEAD") || name == "HEAD" {
            continue;
        }
        let tip = cols.next().unwrap_or("").to_string();
        let committed_at = cols.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let subject = cols.next().unwrap_or("").to_string();
        let kind = if refname.starts_with("refs/remotes/") {
            "remote"
        } else {
            "local"
        };
        let (ahead, behind) = match base.as_deref() {
            Some(base) => ahead_behind(workspace, base, name),
            None => (0, 0),
        };
        let inferred = parse_agent_issue_branch(name);
        branches.push(GitBranch {
            name: name.to_string(),
            kind: kind.into(),
            tip,
            committed_at,
            subject,
            ahead,
            behind,
            inferred_issue: inferred,
            issue_number: inferred,
            issue_title: None,
            issue_state: None,
            issue_url: None,
            override_issue: false,
        });
    }
    branches.sort_by(|a, b| {
        b.committed_at
            .cmp(&a.committed_at)
            .then(a.name.cmp(&b.name))
    });
    Ok(branches)
}

pub fn fetch_issue_meta(number: i64) -> Result<IssueMeta, HubError> {
    fetch_issue_meta_with(gh_json, number)
}

pub fn fetch_issue_meta_with(
    runner: impl FnOnce(&[&str]) -> Result<serde_json::Value, HubError>,
    number: i64,
) -> Result<IssueMeta, HubError> {
    let repo = issue_repo();
    let n = number.to_string();
    let value = runner(&[
        "issue",
        "view",
        &n,
        "--repo",
        &repo,
        "--json",
        "title,state,url",
    ])?;
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if title.is_empty() {
        return Err(HubError::Invalid(format!("issue #{number} has no title")));
    }
    Ok(IssueMeta {
        title,
        state: value
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("OPEN")
            .to_string(),
        url: value
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Apply persisted overrides, then attach GitHub title/state when `gh` works.
pub fn attach_issue_metadata(
    mut branches: Vec<GitBranch>,
    overrides: &BTreeMap<String, i64>,
    lookup: impl Fn(i64) -> Result<IssueMeta, HubError>,
) -> BranchList {
    for branch in &mut branches {
        if let Some(&issue) = overrides.get(&branch.name) {
            branch.issue_number = Some(issue);
            branch.override_issue = true;
        }
    }
    let wanted: BTreeSet<i64> = branches.iter().filter_map(|b| b.issue_number).collect();
    let mut metas = BTreeMap::new();
    let mut gh_error = None;
    for number in wanted {
        match lookup(number) {
            Ok(meta) => {
                metas.insert(number, meta);
            }
            Err(error) => {
                if gh_error.is_none() {
                    gh_error = Some(error.to_string());
                }
            }
        }
    }
    for branch in &mut branches {
        if let Some(number) = branch.issue_number {
            if let Some(meta) = metas.get(&number) {
                branch.issue_title = Some(meta.title.clone());
                branch.issue_state = Some(meta.state.clone());
                branch.issue_url = Some(meta.url.clone());
            }
        }
    }
    let notice = if metas.is_empty() {
        gh_error
            .map(|error| format!("GitHub issues unavailable ({error}). Showing branch data only."))
    } else {
        None
    };
    BranchList { branches, notice }
}

pub fn list_workspace_branches(
    workspace: &Path,
    overrides: &BTreeMap<String, i64>,
) -> Result<BranchList, HubError> {
    let branches = collect_git_branches(workspace)?;
    Ok(attach_issue_metadata(branches, overrides, fetch_issue_meta))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    fn init_repo(dir: &Path) {
        git(dir, &["init", "-b", "main"]);
        git(dir, &["config", "user.email", "u19@test"]);
        git(dir, &["config", "user.name", "U19"]);
        fs::write(dir.join("f"), "one").unwrap();
        git(dir, &["add", "f"]);
        git(dir, &["commit", "-m", "init"]);
        git(dir, &["checkout", "-b", "agent/cursor-313"]);
        fs::write(dir.join("f"), "two").unwrap();
        git(dir, &["commit", "-am", "link issue"]);
    }

    #[test]
    fn collect_lists_local_agent_branch_ahead_of_main() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        let branches = collect_git_branches(dir.path()).unwrap();
        let feature = branches
            .iter()
            .find(|b| b.name == "agent/cursor-313")
            .expect("feature branch");
        assert_eq!(feature.kind, "local");
        assert_eq!(feature.inferred_issue, Some(313));
        assert_eq!(feature.issue_number, Some(313));
        assert_eq!(feature.ahead, 1);
        assert_eq!(feature.behind, 0);
        assert!(feature.subject.contains("link issue"));
        assert!(branches.iter().any(|b| b.name == "main"));
    }

    #[test]
    fn collect_rejects_a_relative_workspace() {
        let err = collect_git_branches(Path::new("rel/repo")).unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn attach_prefers_override_and_fills_issue_meta() {
        let branches = vec![GitBranch {
            name: "feat/spike".into(),
            kind: "local".into(),
            tip: "abc".into(),
            committed_at: 1,
            subject: "wip".into(),
            ahead: 0,
            behind: 0,
            inferred_issue: None,
            issue_number: None,
            issue_title: None,
            issue_state: None,
            issue_url: None,
            override_issue: false,
        }];
        let mut overrides = BTreeMap::new();
        overrides.insert("feat/spike".into(), 313);
        let list = attach_issue_metadata(branches, &overrides, |_| {
            Ok(IssueMeta {
                title: "Git branches tab".into(),
                state: "OPEN".into(),
                url: "https://example.test/313".into(),
            })
        });
        assert!(list.notice.is_none());
        assert_eq!(list.branches[0].issue_number, Some(313));
        assert_eq!(
            list.branches[0].issue_title.as_deref(),
            Some("Git branches tab")
        );
        assert!(list.branches[0].override_issue);
    }

    #[test]
    fn attach_degrades_when_github_is_down() {
        let branches = vec![GitBranch {
            name: "agent/cursor-313".into(),
            kind: "local".into(),
            tip: "abc".into(),
            committed_at: 1,
            subject: "wip".into(),
            ahead: 0,
            behind: 0,
            inferred_issue: Some(313),
            issue_number: Some(313),
            issue_title: None,
            issue_state: None,
            issue_url: None,
            override_issue: false,
        }];
        let list = attach_issue_metadata(branches, &BTreeMap::new(), |_| {
            Err(HubError::Invalid("gh timed out".into()))
        });
        assert!(list.notice.as_deref().unwrap().contains("branch data only"));
        assert_eq!(list.branches[0].issue_number, Some(313));
        assert_eq!(list.branches[0].issue_title, None);
    }

    #[test]
    fn fetch_issue_meta_with_reads_title_state_url() {
        let meta = fetch_issue_meta_with(
            |_| {
                Ok(json!({
                    "title": "Git branches tab",
                    "state": "OPEN",
                    "url": "https://github.com/ACFHarbinger/Coding-Assistants/issues/313"
                }))
            },
            313,
        )
        .unwrap();
        assert_eq!(meta.title, "Git branches tab");
        assert_eq!(meta.state, "OPEN");
    }
}

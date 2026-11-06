//! GitHub Project v2 board via `gh` (U18 / #312).

use super::gh_json;
use crate::HubError;
use serde::{Deserialize, Serialize};

pub const DEFAULT_PROJECT_OWNER: &str = "ACFHarbinger";
pub const DEFAULT_PROJECT_NUMBER: u32 = 21;

pub const DEFAULT_BOARD_COLUMNS: &[&str] = &[
    "Backlog",
    "On hold",
    "Ready",
    "Rejected",
    "In progress",
    "In review",
    "Done",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardColumn {
    pub name: String,
    pub cards: Vec<BoardCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardCard {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub status: String,
    pub issue_number: Option<i64>,
    pub url: Option<String>,
    pub labels: Vec<String>,
    pub github_assignees: Vec<String>,
    pub roster_assignees: Vec<String>,
    pub deadline: Option<String>,
    pub linked_branches: Vec<String>,
    pub size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBoard {
    pub owner: String,
    pub project_number: u32,
    pub columns: Vec<BoardColumn>,
    pub notice: Option<String>,
}

/// `agent/<name>-<issue>` → issue number. Extra suffix after the number is
/// ignored (`agent/gemini-314-u20` → 314).
pub fn parse_agent_issue_branch(branch: &str) -> Option<i64> {
    let name = branch.rsplit('/').next().unwrap_or(branch);
    let rest = name.strip_prefix("agent/").or_else(|| {
        if name.starts_with("agent-") {
            Some(name)
        } else {
            name.split_once('/')
                .and_then(|(head, tail)| (head == "agent").then_some(tail))
        }
    });
    let rest = rest.unwrap_or(name);
    let rest = rest.strip_prefix("agent/").unwrap_or(rest);
    // agent/grok-312 or grok-312
    let after_hyphen = rest.split_once('-')?.1;
    let digits: String = after_hyphen
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn project_owner() -> String {
    std::env::var("CA_GH_PROJECT_OWNER")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_PROJECT_OWNER.to_string())
}

fn project_number() -> u32 {
    std::env::var("CA_GH_PROJECT_NUMBER")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(DEFAULT_PROJECT_NUMBER)
}

pub fn empty_project_board(notice: Option<String>) -> ProjectBoard {
    board_from_items(&project_owner(), project_number(), &[], &[], notice)
}

pub fn list_project_board(linked_branches: &[(String, i64)]) -> Result<ProjectBoard, HubError> {
    let owner = project_owner();
    let number = project_number();
    let args = [
        "project",
        "item-list",
        &number.to_string(),
        "--owner",
        &owner,
        "--limit",
        "400",
        "--format",
        "json",
    ];
    let value = gh_json(&args)?;
    let items = value
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(board_from_items(
        &owner,
        number,
        &items,
        linked_branches,
        None,
    ))
}

pub fn board_from_items(
    owner: &str,
    number: u32,
    items: &[serde_json::Value],
    linked_branches: &[(String, i64)],
    notice: Option<String>,
) -> ProjectBoard {
    let mut columns: Vec<BoardColumn> = DEFAULT_BOARD_COLUMNS
        .iter()
        .map(|name| BoardColumn {
            name: (*name).to_string(),
            cards: Vec::new(),
        })
        .collect();
    for item in items {
        let card = card_from_item(item, linked_branches);
        if let Some(col) = columns.iter_mut().find(|c| c.name == card.status) {
            col.cards.push(card);
        } else {
            columns.push(BoardColumn {
                name: card.status.clone(),
                cards: vec![card],
            });
        }
    }
    ProjectBoard {
        owner: owner.to_string(),
        project_number: number,
        columns,
        notice,
    }
}

fn content_url(content: &serde_json::Value) -> Option<String> {
    if let Some(url) = content.get("url").and_then(|v| v.as_str()) {
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }
    let number = content.get("number").and_then(|v| v.as_i64())?;
    let repo = content
        .get("repository")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("ACFHarbinger/Coding-Assistants");
    let slug = match content
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("Issue")
    {
        "PullRequest" => "pull",
        _ => "issues",
    };
    Some(format!("https://github.com/{repo}/{slug}/{number}"))
}

fn card_from_item(item: &serde_json::Value, linked_branches: &[(String, i64)]) -> BoardCard {
    let content = item
        .get("content")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let issue_number = content.get("number").and_then(|v| v.as_i64());
    let labels = item
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let github_assignees = item
        .get("assignees")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_str().map(|s| s.to_string()).or_else(|| {
                        v.get("login")
                            .and_then(|l| l.as_str())
                            .map(|s| s.to_string())
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let linked_branches = issue_number
        .map(|n| {
            linked_branches
                .iter()
                .filter(|(_, issue)| *issue == n)
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default();
    BoardCard {
        id: item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        kind: "github".into(),
        title: item
            .get("title")
            .and_then(|v| v.as_str())
            .or_else(|| content.get("title").and_then(|v| v.as_str()))
            .unwrap_or("untitled")
            .to_string(),
        status: item
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("Backlog")
            .to_string(),
        issue_number,
        url: content_url(&content),
        labels,
        github_assignees,
        roster_assignees: Vec::new(),
        deadline: None,
        linked_branches,
        size: item
            .get("size")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    }
}

pub fn set_project_item_status(item_url: &str, status: &str) -> Result<(), HubError> {
    if item_url.trim().is_empty() || status.trim().is_empty() {
        return Err(HubError::Invalid("item url and status are required".into()));
    }
    let owner = project_owner();
    let number = project_number().to_string();
    let args = [
        "project",
        "item-edit",
        &number,
        "--owner",
        &owner,
        "--url",
        item_url,
        "--field",
        "Status",
        "--value",
        status,
    ];
    super::run_gh(&args).map_err(HubError::Invalid)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_from_items_groups_known_and_unknown_status() {
        let items = vec![
            serde_json::json!({
                "id": "PVTI_1",
                "title": "Kanban board",
                "status": "Ready",
                "labels": ["enhancement"],
                "content": {"number": 312, "url": "https://github.com/ACFHarbinger/Coding-Assistants/issues/312", "title": "Kanban board"}
            }),
            serde_json::json!({
                "id": "PVTI_2",
                "title": "Mystery",
                "status": "Icebox",
                "content": {"number": 1}
            }),
        ];
        let board = board_from_items(
            "ACFHarbinger",
            21,
            &items,
            &[("agent/grok-312".into(), 312)],
            None,
        );
        let ready = board.columns.iter().find(|c| c.name == "Ready").unwrap();
        assert_eq!(ready.cards.len(), 1);
        assert_eq!(ready.cards[0].issue_number, Some(312));
        assert_eq!(ready.cards[0].linked_branches, vec!["agent/grok-312"]);
        assert_eq!(
            ready.cards[0].url.as_deref(),
            Some("https://github.com/ACFHarbinger/Coding-Assistants/issues/312")
        );
        assert!(board.columns.iter().any(|c| c.name == "Icebox"));
    }

    #[test]
    fn content_url_synthesizes_pull_requests_without_an_explicit_url() {
        let url = content_url(&serde_json::json!({
            "number": 99,
            "repository": "acme/app",
            "type": "PullRequest"
        }));
        assert_eq!(url.as_deref(), Some("https://github.com/acme/app/pull/99"));
    }
}

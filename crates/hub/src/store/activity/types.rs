use serde::{Deserialize, Serialize};

/// Query filter for D4 tool and workspace activity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityFilter {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub workspace_path: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// A file touch recorded for a task or work session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityFileTouch {
    pub path: String,
    pub operation: String,
    pub observed_at: String,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub status: String,
}

/// A command or tool execution identified for a task or work session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityCommandRun {
    pub cmdline: Vec<String>,
    pub raw: String,
    #[serde(default)]
    pub exe: Option<String>,
    pub observed_at: String,
    #[serde(default)]
    pub attribution: Option<String>,
    #[serde(default)]
    pub source: String,
}

/// Consolidated activity record per task or work session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub workspace_path: Option<String>,
    pub started_at: String,
    pub updated_at: String,
    pub agents: Vec<String>,
    pub commands: Vec<ActivityCommandRun>,
    pub files: Vec<ActivityFileTouch>,
    pub message_count: usize,
    pub capture_count: usize,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct ProcessJsonMeta {
    #[serde(default)]
    pub cmdline: Option<serde_json::Value>,
    #[serde(default)]
    pub exe: Option<String>,
    #[serde(default)]
    pub attribution: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

pub(super) fn parse_cmdline(val: &serde_json::Value) -> Vec<String> {
    match val {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        serde_json::Value::String(s) => s.split_whitespace().map(str::to_string).collect(),
        _ => Vec::new(),
    }
}

pub(super) fn parse_process_meta(json_str: &str) -> ProcessJsonMeta {
    serde_json::from_str(json_str).unwrap_or_default()
}

pub(super) fn extract_commands_from_text(
    text: &str,
    out: &mut Vec<ActivityCommandRun>,
    timestamp: &str,
) {
    for line in text.lines() {
        let trimmed = line.trim();
        let cmd = if let Some(stripped) = trimmed.strip_prefix("$ ") {
            Some(stripped.trim())
        } else if let Some(stripped) = trimmed.strip_prefix("> ") {
            let inner = stripped.trim();
            if inner.starts_with("cargo ") || inner.starts_with("git ") || inner.starts_with("npm ")
            {
                Some(inner)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(c) = cmd {
            if !c.is_empty() && !out.iter().any(|existing| existing.raw == c) {
                let parts: Vec<String> = c.split_whitespace().map(str::to_string).collect();
                out.push(ActivityCommandRun {
                    raw: c.to_string(),
                    cmdline: parts,
                    exe: None,
                    observed_at: timestamp.to_string(),
                    attribution: None,
                    source: "capture".into(),
                });
            }
        }
    }
}

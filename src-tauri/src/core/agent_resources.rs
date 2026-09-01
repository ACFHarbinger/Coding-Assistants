//! List `.agent` prompt/rule/workflow/skill files for Orchestrate and the
//! Android companion.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentResources {
    pub prompts: Vec<String>,
    pub rules: Vec<String>,
    pub workflows: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
}

/// Settings default workspace, else the desktop process current directory.
pub fn resolve_desktop_workspace() -> String {
    let from_settings = hub::SettingsStore::open(hub::default_hub_home())
        .snapshot()
        .default_workspace
        .clone()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(workspace) = from_settings {
        return workspace;
    }
    std::env::current_dir()
        .ok()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".".to_string())
}

pub fn list_agent_resources(work_dir: &str) -> AgentResources {
    let base = Path::new(work_dir).join(".agent");
    AgentResources {
        prompts: list_files(&base.join("prompts"), ".agent/prompts"),
        rules: list_files(&base.join("rules"), ".agent/rules"),
        workflows: list_files(&base.join("workflows"), ".agent/workflows"),
        skills: list_files(&base.join("skills"), ".agent/skills"),
    }
}

fn list_files(dir: &Path, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let filename = entry.file_name().to_string_lossy().to_string();
            out.push(format!("{prefix}/{filename}"));
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::{list_agent_resources, AgentResources};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn list_agent_resources_reads_four_kinds_and_ignores_missing_dirs() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ca-agent-resources-{stamp}"));
        fs::create_dir_all(root.join(".agent/prompts")).unwrap();
        fs::create_dir_all(root.join(".agent/rules")).unwrap();
        fs::create_dir_all(root.join(".agent/workflows")).unwrap();
        fs::create_dir_all(root.join(".agent/skills")).unwrap();
        fs::write(root.join(".agent/prompts/planner.md"), "p").unwrap();
        fs::write(root.join(".agent/rules/rust.md"), "r").unwrap();
        fs::write(root.join(".agent/workflows/gui_dev.md"), "w").unwrap();
        fs::write(root.join(".agent/skills/debug.md"), "s").unwrap();
        fs::create_dir_all(root.join(".agent/prompts/nested")).unwrap();

        let listed = list_agent_resources(root.to_str().unwrap());
        let _ = fs::remove_dir_all(&root);

        assert_eq!(
            listed,
            AgentResources {
                prompts: vec![".agent/prompts/planner.md".into()],
                rules: vec![".agent/rules/rust.md".into()],
                workflows: vec![".agent/workflows/gui_dev.md".into()],
                skills: vec![".agent/skills/debug.md".into()],
            }
        );
        assert_eq!(
            list_agent_resources("/no/such/coding-assistants-workspace"),
            AgentResources::default()
        );
    }
}

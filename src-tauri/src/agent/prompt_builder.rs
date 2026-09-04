use super::memory_recall::{append_recalled_memories, RecalledMemory};
use crate::client::llm::ModelConfig;
use crate::core::file_tools::FileTools;

async fn read(file_tools: &FileTools, path: &str) -> Result<String, String> {
    file_tools
        .read_file(path)
        .await
        .map_err(|error| format!("Failed to read file {path}: {error}"))
}

pub async fn construct_prompt(
    file_tools: &FileTools,
    config: &ModelConfig,
    task: &str,
    context: &str,
    default_system: &str,
    roles: &[String],
    workspace: &str,
) -> Result<(String, Option<(u8, Vec<RecalledMemory>)>), String> {
    let mut prompt = String::new();
    for (label, path) in [
        ("Workflow", config.workflow_file.as_deref()),
        ("Rules", config.rule_file.as_deref()),
        ("Skill", config.skill_file.as_deref()),
    ] {
        if let Some(path) = path {
            prompt.push_str(&format!("{label}:\n{}\n\n", read(file_tools, path).await?));
        }
    }
    if let Ok(memory) = read(file_tools, ".agent/project_memory.md").await {
        if !memory.is_empty() {
            prompt.push_str(&format!(
                "Project Memory (Past Tasks and Context):\n{memory}\n\n"
            ));
        }
    }
    let recalled = append_recalled_memories(&mut prompt, task, workspace).await;
    let system_prompt = match config.prompt_file.as_deref() {
        Some(path) => read(file_tools, path).await?,
        None => default_system.to_string(),
    };
    prompt.push_str(&format!("{system_prompt}\n\n"));
    prompt.push_str("IMPORTANT: If you need clarification from the user, output `[[ASK_USER]]` followed by your question on a new line. Stops speaking. Wait for the user's response.\n");
    let fallback = "Developer".to_string();
    prompt.push_str(&format!(
        "IMPORTANT: If you need to ask another agent ({}), output `[[ASK_AGENT:Role]]` followed by your question. e.g. `[[ASK_AGENT:{}]]. How do I implement X?`\n\n",
        roles.join(", "), roles.first().unwrap_or(&fallback)
    ));
    prompt.push_str(context);
    Ok((prompt, recalled))
}

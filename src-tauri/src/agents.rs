use crate::file_tools::FileTools;
use crate::llm_client::{LLMClient, ModelConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub planner: ModelConfig,
    pub developer: ModelConfig,
    pub reviewer: ModelConfig,
    pub work_dir: String,
}

pub struct AgentSystem {
    pub client: LLMClient,
    pub file_tools: FileTools,
    pub config: AgentConfig,
}

impl AgentSystem {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            client: LLMClient::new(),
            file_tools: FileTools::new(config.work_dir.clone()),
            config,
        }
    }

    pub async fn run_task(&self, task: &str) -> Result<String, String> {
        self.execute_phases(task).await
    }

    async fn execute_phases(&self, task: &str) -> Result<String, String> {
        // 1. Planner
        let planner_context = format!("Task: {}", task);
        let planner_prompt = self.construct_prompt(&self.config.planner, &planner_context, "System: You are an expert software architect.\nUser: You are a task planner. Break down the following task.").await?;

        let plan = self
            .client
            .chat_completion(&self.config.planner, &planner_prompt)
            .await?;

        // 2. Developer
        let developer_context = format!("Plan: {}", plan);
        let developer_prompt = self.construct_prompt(&self.config.developer, &developer_context, "System: You are a senior software developer.\nUser: Based on this plan, implement the solution.").await?;

        let developer_result = self
            .client
            .chat_completion(&self.config.developer, &developer_prompt)
            .await?;

        // 3. Reviewer
        let reviewer_context = format!(
            "Task: {}\nPlan: {}\nImplementation: {}",
            task, plan, developer_result
        );
        let reviewer_prompt = self.construct_prompt(&self.config.reviewer, &reviewer_context, "System: You are a QA engineer and code reviewer.\nUser: Review the following implementation. Provide a code review and any necessary corrections.").await?;

        let reviewer_result = self
            .client
            .chat_completion(&self.config.reviewer, &reviewer_prompt)
            .await?;

        Ok(format!(
            "## Developer Output\n{}\n\n## Reviewer Output\n{}",
            developer_result, reviewer_result
        ))
    }

    async fn get_file_content(&self, path: &Option<String>) -> Result<String, String> {
        match path {
            Some(p) => self
                .file_tools
                .read_file(p)
                .map_err(|e| format!("Failed to read file {}: {}", p, e)),
            None => Ok(String::new()),
        }
    }

    async fn construct_prompt(
        &self,
        config: &ModelConfig,
        context: &str,
        default_system: &str,
    ) -> Result<String, String> {
        let mut full_prompt = String::new();

        // 1. Workflows
        if let Some(workflow_path) = &config.workflow_file {
            let workflow = self.get_file_content(&Some(workflow_path.clone())).await?;
            full_prompt.push_str(&format!("Workflow:\n{}\n\n", workflow));
        }

        // 2. Rules
        if let Some(rule_path) = &config.rule_file {
            let rule = self.get_file_content(&Some(rule_path.clone())).await?;
            full_prompt.push_str(&format!("Rules:\n{}\n\n", rule));
        }

        // 3. System Prompt (File or Default)
        let system_prompt = if let Some(prompt_path) = &config.prompt_file {
            self.get_file_content(&Some(prompt_path.clone())).await?
        } else {
            default_system.to_string()
        };
        full_prompt.push_str(&format!("{}\n\n", system_prompt));

        // 4. Context
        full_prompt.push_str(context);

        Ok(full_prompt)
    }
}

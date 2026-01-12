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
        // 1. Planner Phase
        let planner_prompt = format!(
            "System: You are an expert software architect.\nUser: You are a task planner. Break down the following task: {}", 
            task
        );

        let plan = self
            .client
            .chat_completion(&self.config.planner, &planner_prompt)
            .await?;

        // 2. Developer Phase
        let developer_prompt = format!(
            "System: You are a senior software developer.\nUser: Based on this plan, implement the solution: {}", 
            plan
        );

        let developer_result = self
            .client
            .chat_completion(&self.config.developer, &developer_prompt)
            .await?;

        // 3. Reviewer Phase
        let reviewer_prompt = format!(
            "System: You are a QA engineer and code reviewer.\nUser: Review the following implementation for the task '{}':\n\nPlan:\n{}\n\nImplementation:\n{}\n\nProvide a code review and any necessary corrections.", 
            task, plan, developer_result
        );

        let reviewer_result = self
            .client
            .chat_completion(&self.config.reviewer, &reviewer_prompt)
            .await?;

        let final_output = format!(
            "## Developer Output\n{}\n\n## Reviewer Output\n{}",
            developer_result, reviewer_result
        );

        Ok(final_output)
    }
}

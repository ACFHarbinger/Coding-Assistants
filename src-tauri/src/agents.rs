use crate::file_tools::FileTools;
use crate::llm_client::{LLMClient, ModelConfig};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
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
        let mut client = LLMClient::new();
        if let Some(key) = &config.planner.api_key {
            client.set_openai_key(key.clone());
        }

        Self {
            client,
            file_tools: FileTools::new(config.work_dir.clone()),
            config,
        }
    }

    pub async fn run_task(&self, task: &str) -> Result<String, String> {
        // 1. Planner Phase
        let planner_prompt = format!(
            "You are a task planner. Break down the following task: {}",
            task
        );
        let messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are an expert software architect.")
                .build()
                .map_err(|e| e.to_string())?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(planner_prompt)
                .build()
                .map_err(|e| e.to_string())?
                .into(),
        ];

        let plan = self
            .client
            .chat_completion(&self.config.planner, messages)
            .await?;

        // 2. Developer Phase (Simplified for now)
        let developer_prompt = format!("Based on this plan, implement the solution: {}", plan);
        let messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a senior software developer.")
                .build()
                .map_err(|e| e.to_string())?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(developer_prompt)
                .build()
                .map_err(|e| e.to_string())?
                .into(),
        ];

        let result = self
            .client
            .chat_completion(&self.config.developer, messages)
            .await?;
        Ok(result)
    }
}

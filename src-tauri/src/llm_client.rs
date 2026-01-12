use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub prompt_file: Option<String>,
    pub rule_file: Option<String>,
    pub workflow_file: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "opencode".to_string(),
            model: "big-pickle".to_string(),
            prompt_file: None,
            rule_file: None,
            workflow_file: None,
        }
    }
}

pub struct LLMClient;

impl LLMClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn chat_completion(
        &self,
        config: &ModelConfig,
        prompt: &str,
    ) -> Result<String, String> {
        let model_str = format!("{}/{}", config.provider, config.model);

        let output = Command::new("opencode")
            .arg("run")
            .arg(prompt)
            .arg("-m")
            .arg(&model_str)
            .output()
            .map_err(|e| format!("Failed to execute opencode: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("OpenCode error: {}", error))
        }
    }
}

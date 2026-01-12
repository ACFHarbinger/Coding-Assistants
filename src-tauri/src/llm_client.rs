use crate::agents::AgentEvent;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tauri::{Emitter, Window};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

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
        work_dir: Option<&str>,
        window: &Window,
        source: &str,
    ) -> Result<String, String> {
        let model_str = format!("{}/{}", config.provider, config.model);

        let mut command = Command::new("opencode");
        command
            .arg("run")
            .arg(prompt)
            .arg("-m")
            .arg(&model_str)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = work_dir {
            command.current_dir(dir);
        }

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn opencode: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to open stderr")?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut full_output = String::new();

        while reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?
            > 0
        {
            let _ = window.emit(
                "agent-event",
                AgentEvent {
                    source: source.to_string(),
                    event_type: "stream".to_string(),
                    content: line.clone(),
                },
            );
            full_output.push_str(&line);
            line.clear();
        }

        let status = child.wait().await.map_err(|e| e.to_string())?;

        if status.success() {
            Ok(full_output)
        } else {
            // Read stderr if failed
            let mut err_reader = BufReader::new(stderr);
            let mut err_output = String::new();
            err_reader
                .read_to_string(&mut err_output)
                .await
                .map_err(|e| e.to_string())?;
            Err(format!("OpenCode error: {}", err_output))
        }
    }
}

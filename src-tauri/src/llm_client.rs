use crate::agents::AgentEvent;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Window};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use tokio::time::Duration;

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

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.start_kill();
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
        mcp_config_path: Option<&str>,
        token: Option<Arc<AtomicBool>>,
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
            if let Some(mcp_file) = mcp_config_path {
                let full_mcp_path = Path::new(dir).join(mcp_file);
                eprintln!("Setting MCP_CONFIG_FILE to {:?}", full_mcp_path);
                command.env("MCP_CONFIG_FILE", full_mcp_path);
            }
        }

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn opencode: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to open stderr")?;

        // Ensure child is killed on drop/cancellation
        let mut child_guard = KillOnDrop(child);

        let window_clone = window.clone();
        let source_clone = source.to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                let _ = window_clone.emit(
                    "agent-event",
                    AgentEvent {
                        source: source_clone.clone(),
                        event_type: "log".to_string(),
                        content: line.clone(),
                    },
                );
                line.clear();
            }
        });

        // Cancellation signal setup
        let (cancel_tx, mut cancel_rx) = oneshot::channel();
        if let Some(token) = token {
            tokio::spawn(async move {
                loop {
                    if token.load(Ordering::SeqCst) {
                        let _ = cancel_tx.send(());
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            });
        }

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut full_output = String::new();

        loop {
            tokio::select! {
                result = reader.read_line(&mut line) => {
                     match result {
                         Ok(0) => break, // EOF
                         Ok(_) => {
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
                         Err(e) => return Err(e.to_string()),
                     }
                }
                _ = &mut cancel_rx => {
                     return Err("Task cancelled".to_string());
                }
            }
        }

        let status = child_guard.0.wait().await.map_err(|e| e.to_string())?;

        if status.success() {
            Ok(full_output)
        } else {
            Err(format!(
                "Command failed with status: {}. Check logs for details.",
                status
            ))
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>, String> {
        let output = Command::new("opencode")
            .arg("models")
            .output()
            .await
            .map_err(|e| format!("Failed to execute opencode models: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "opencode models failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let content = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        Ok(content.lines().map(|s| s.to_string()).collect())
    }
}

use crate::file_tools::FileTools;
use crate::llm_client::{LLMClient, ModelConfig};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Window};
use tokio::sync::mpsc;

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub source: String,     // Planner, Developer, Reviewer
    pub event_type: String, // "thought" (input) or "response" (output)
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleConfig {
    pub name: String,
    pub config: ModelConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub roles: Vec<RoleConfig>,
    pub work_dir: String,
    pub mcp_config: String,
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

    pub async fn run_task(
        &self,
        task: &str,
        window: &Window,
        token: Arc<AtomicBool>,
        mut input_rx: mpsc::Receiver<String>,
    ) -> Result<String, String> {
        self.execute_phases(task, window, token, &mut input_rx)
            .await
    }

    async fn execute_phases(
        &self,
        task: &str,
        window: &Window,
        token: Arc<AtomicBool>,
        input_rx: &mut mpsc::Receiver<String>,
    ) -> Result<String, String> {
        // Write MCP Config
        if !self.config.mcp_config.is_empty() {
            if let Err(e) = self
                .file_tools
                .write_file("mcp.json", &self.config.mcp_config)
            {
                eprintln!("Failed to write mcp.json: {}", e);
            }
        }

        let mut previous_outputs = format!("Task: {}\n", task);
        let mut final_result = String::new();

        for role_config in &self.config.roles {
            if token.load(Ordering::SeqCst) {
                return Err("Task cancelled".into());
            }

            let role_name = &role_config.name;
            let default_system = format!(
                "System: You are an expert {}.\nUser: Contribue to the task based on previous work.",
                role_name
            );

            let prompt = self
                .construct_prompt(&role_config.config, &previous_outputs, &default_system)
                .await?;

            let _ = window.emit(
                "agent-event",
                AgentEvent {
                    source: role_name.clone(),
                    event_type: "thought".into(),
                    content: prompt.clone(),
                },
            );

            let output = self
                .interactive_completion(
                    &role_config.config,
                    &prompt,
                    window,
                    role_name,
                    token.clone(),
                    input_rx,
                )
                .await?;

            let _ = window.emit(
                "agent-event",
                AgentEvent {
                    source: role_name.clone(),
                    event_type: "response".into(),
                    content: output.clone(),
                },
            );

            // Save Role Report
            let filename = format!("{}.md", role_name.to_lowercase().replace(" ", "_"));
            if let Err(e) = self.file_tools.write_file(&filename, &output) {
                eprintln!("Failed to write {}: {}", filename, e);
            }

            previous_outputs.push_str(&format!("\nOutput from {}:\n{}\n", role_name, output));
            final_result.push_str(&format!("## {} Output\n{}\n\n", role_name, output));
        }

        Ok(final_result)
    }

    async fn interactive_completion(
        &self,
        config: &ModelConfig,
        initial_prompt: &str,
        window: &Window,
        source: &str,
        token: Arc<AtomicBool>,
        input_rx: &mut mpsc::Receiver<String>,
    ) -> Result<String, String> {
        let mut history = initial_prompt.to_string();

        loop {
            // Call LLM
            let response = self
                .client
                .chat_completion(
                    config,
                    &history,
                    Some(&self.config.work_dir),
                    window,
                    source,
                    Some("mcp.json"),
                    Some(token.clone()),
                )
                .await?;

            // Check for [[ASK_USER]]
            if let Some(pos) = response.find("[[ASK_USER]]") {
                let question = response[pos + "[[ASK_USER]]".len()..].trim().to_string();
                let question_text = if question.is_empty() {
                    "Agent requesting input...".to_string()
                } else {
                    question
                };

                // Emit event to frontend to show prompt
                window
                    .emit(
                        "agent-event",
                        AgentEvent {
                            source: source.to_string(),
                            event_type: "question".into(),
                            content: question_text.clone(),
                        },
                    )
                    .map_err(|e| e.to_string())?;

                // Wait for input
                let user_input = match input_rx.recv().await {
                    Some(input) => input,
                    None => return Err("User input channel closed".into()),
                };

                // Emit acknowledgement
                window
                    .emit(
                        "agent-event",
                        AgentEvent {
                            source: "User".into(),
                            event_type: "input".into(),
                            content: user_input.clone(),
                        },
                    )
                    .map_err(|e| e.to_string())?;

                history.push_str("\n\nAgent: ");
                history.push_str(&response);
                history.push_str("\n\nUser: ");
                history.push_str(&user_input);

                // Loop again
            } else if let Some(pos) = response.find("[[ASK_AGENT:") {
                let rest = &response[pos + "[[ASK_AGENT:".len()..];
                if let Some(end_bracket) = rest.find("]]") {
                    let target_role_name = &rest[..end_bracket];
                    let question = rest[end_bracket + 2..].trim(); // +2 for ]]
                    let question = if question.is_empty() {
                        "Can you help me with this?"
                    } else {
                        question
                    };

                    let target_role = self
                        .config
                        .roles
                        .iter()
                        .find(|r| r.name.to_lowercase() == target_role_name.to_lowercase());

                    let target_config = match target_role {
                        Some(r) => &r.config,
                        None => {
                            let roles_list: Vec<String> =
                                self.config.roles.iter().map(|r| r.name.clone()).collect();
                            history.push_str(&format!(
                                "\n\nSystem: Unknown agent role. Available roles: {}.",
                                roles_list.join(", ")
                            ));
                            continue;
                        }
                    };

                    // Authorization Step
                    let auth_payload = serde_json::json!({
                        "role": target_role_name,
                        "question": question
                    })
                    .to_string();

                    window
                        .emit(
                            "agent-event",
                            AgentEvent {
                                source: "System".into(),
                                event_type: "authorization".into(),
                                content: auth_payload,
                            },
                        )
                        .map_err(|e| e.to_string())?;

                    // Wait for authorization
                    let auth_response = match input_rx.recv().await {
                        Some(input) => input,
                        None => return Err("User input channel closed".into()),
                    };

                    if auth_response != "APPROVED" {
                        window
                            .emit(
                                "agent-event",
                                AgentEvent {
                                    source: "System".into(),
                                    event_type: "thought".into(),
                                    content: format!(
                                        "Authorization DENIED for asking {}",
                                        target_role_name
                                    ),
                                },
                            )
                            .map_err(|e| e.to_string())?;

                        history.push_str(&format!(
                            "\n\nSystem: User DENIED the request to ask {}.",
                            target_role_name
                        ));
                        continue;
                    }

                    window
                        .emit(
                            "agent-event",
                            AgentEvent {
                                source: source.to_string(),
                                event_type: "thought".into(),
                                content: format!("Asking {}: {}", target_role_name, question),
                            },
                        )
                        .map_err(|e| e.to_string())?;

                    let target_context = format!(
                        "Context from {}:\n{}\n\nQuestion: {}",
                        source, history, question
                    );
                    let target_system = format!(
                        "System: You are expert {}.\nUser: Answer the question from {}.",
                        target_role_name, source
                    );

                    let target_prompt = format!("{}\n\n{}", target_system, target_context);

                    // Call target agent (non-interactive to avoid infinite loops for now)
                    let answer = self
                        .client
                        .chat_completion(
                            target_config,
                            &target_prompt,
                            Some(&self.config.work_dir),
                            window,
                            target_role_name,
                            Some("mcp.json"),
                            Some(token.clone()),
                        )
                        .await?;

                    history.push_str("\n\nAgent: ");
                    history.push_str(&response);
                    history.push_str(&format!("\n\nAgent {}: ", target_role_name));
                    history.push_str(&answer);
                } else {
                    history.push_str("\n\nSystem: Malformed ASK_AGENT command.");
                }
            } else {
                return Ok(response);
            }
        }
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
        full_prompt.push_str("IMPORTANT: If you need clarification from the user, output `[[ASK_USER]]` followed by your question on a new line. Stops speaking. Wait for the user's response.\n");

        let roles_list: Vec<String> = self.config.roles.iter().map(|r| r.name.clone()).collect();
        full_prompt.push_str(&format!(
            "IMPORTANT: If you need to ask another agent ({}) a question, output `[[ASK_AGENT:Role]]` followed by your question. e.g. `[[ASK_AGENT:{}]]. How do I implement X?`\n\n",
            roles_list.join(", "),
            roles_list.first().unwrap_or(&"Developer".to_string())
        ));

        // 4. Context
        full_prompt.push_str(context);

        Ok(full_prompt)
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub planner: ModelConfig,
    pub developer: ModelConfig,
    pub reviewer: ModelConfig,
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
        // 1. Planner
        if token.load(Ordering::SeqCst) {
            return Err("Task cancelled".into());
        }

        // Write MCP Config
        if !self.config.mcp_config.is_empty() {
            if let Err(e) = self
                .file_tools
                .write_file("mcp.json", &self.config.mcp_config)
            {
                eprintln!("Failed to write mcp.json: {}", e);
            }
        }

        let planner_context = format!("Task: {}", task);
        let planner_prompt = self.construct_prompt(&self.config.planner, &planner_context, "System: You are an expert software architect.\nUser: You are a task planner. Break down the following task.").await?;

        let _ = window.emit(
            "agent-event",
            AgentEvent {
                source: "Planner".into(),
                event_type: "thought".into(),
                content: planner_prompt.clone(),
            },
        );

        let plan = self
            .interactive_completion(
                &self.config.planner,
                &planner_prompt,
                window,
                "Planner",
                token.clone(),
                input_rx,
            )
            .await?;

        let _ = window.emit(
            "agent-event",
            AgentEvent {
                source: "Planner".into(),
                event_type: "response".into(),
                content: plan.clone(),
            },
        );

        // Save Planner Report
        if let Err(e) = self.file_tools.write_file("plan.md", &plan) {
            eprintln!("Failed to write plan.md: {}", e);
        }

        // 2. Developer
        if token.load(Ordering::SeqCst) {
            return Err("Task cancelled".into());
        }
        let developer_context = format!("Plan: {}", plan);
        let developer_prompt = self.construct_prompt(&self.config.developer, &developer_context, "System: You are a senior software developer.\nUser: Based on this plan, implement the solution.").await?;

        let _ = window.emit(
            "agent-event",
            AgentEvent {
                source: "Developer".into(),
                event_type: "thought".into(),
                content: developer_prompt.clone(),
            },
        );

        let developer_result = self
            .interactive_completion(
                &self.config.developer,
                &developer_prompt,
                window,
                "Developer",
                token.clone(),
                input_rx,
            )
            .await?;

        let _ = window.emit(
            "agent-event",
            AgentEvent {
                source: "Developer".into(),
                event_type: "response".into(),
                content: developer_result.clone(),
            },
        );

        // Save Developer Report
        if let Err(e) = self
            .file_tools
            .write_file("implementation.md", &developer_result)
        {
            eprintln!("Failed to write implementation.md: {}", e);
        }

        // 3. Reviewer
        if token.load(Ordering::SeqCst) {
            return Err("Task cancelled".into());
        }
        let reviewer_context = format!(
            "Task: {}\nPlan: {}\nImplementation: {}",
            task, plan, developer_result
        );
        let reviewer_prompt = self.construct_prompt(&self.config.reviewer, &reviewer_context, "System: You are a QA engineer and code reviewer.\nUser: Review the following implementation. Provide a code review and any necessary corrections.").await?;

        let _ = window.emit(
            "agent-event",
            AgentEvent {
                source: "Reviewer".into(),
                event_type: "thought".into(),
                content: reviewer_prompt.clone(),
            },
        );

        let reviewer_result = self
            .interactive_completion(
                &self.config.reviewer,
                &reviewer_prompt,
                window,
                "Reviewer",
                token.clone(),
                input_rx,
            )
            .await?;

        let _ = window.emit(
            "agent-event",
            AgentEvent {
                source: "Reviewer".into(),
                event_type: "response".into(),
                content: reviewer_result.clone(),
            },
        );

        // Save Reviewer Report
        if let Err(e) = self.file_tools.write_file("review.md", &reviewer_result) {
            eprintln!("Failed to write review.md: {}", e);
        }

        Ok(format!(
            "## Developer Output\n{}\n\n## Reviewer Output\n{}",
            developer_result, reviewer_result
        ))
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
                    let target_role = &rest[..end_bracket];
                    let question = rest[end_bracket + 2..].trim(); // +2 for ]]
                    let question = if question.is_empty() {
                        "Can you help me with this?"
                    } else {
                        question
                    };

                    let target_config = match target_role.to_lowercase().as_str() {
                        "planner" => &self.config.planner,
                        "developer" => &self.config.developer,
                        "reviewer" => &self.config.reviewer,
                        _ => {
                            history.push_str("\n\nSystem: Unknown agent role. Available roles: Planner, Developer, Reviewer.");
                            continue;
                        }
                    };

                    // Authorization Step
                    let auth_payload = serde_json::json!({
                        "role": target_role,
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
                                        target_role
                                    ),
                                },
                            )
                            .map_err(|e| e.to_string())?;

                        history.push_str(&format!(
                            "\n\nSystem: User DENIED the request to ask {}.",
                            target_role
                        ));
                        continue;
                    }

                    window
                        .emit(
                            "agent-event",
                            AgentEvent {
                                source: source.to_string(),
                                event_type: "thought".into(),
                                content: format!("Asking {}: {}", target_role, question),
                            },
                        )
                        .map_err(|e| e.to_string())?;

                    let target_context = format!(
                        "Context from {}:\n{}\n\nQuestion: {}",
                        source, history, question
                    );
                    let target_system = format!(
                        "System: You are expert {}.\nUser: Answer the question from {}.",
                        target_role, source
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
                            target_role,
                            Some("mcp.json"),
                            Some(token.clone()),
                        )
                        .await?;

                    history.push_str("\n\nAgent: ");
                    history.push_str(&response);
                    history.push_str(&format!("\n\nAgent {}: ", target_role));
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
        full_prompt.push_str("IMPORTANT: If you need to ask another agent (Planner, Developer, Reviewer) a question, output `[[ASK_AGENT:Role]]` followed by your question. e.g. `[[ASK_AGENT:Developer]] How do I implement X?`\n\n");

        // 4. Context
        full_prompt.push_str(context);

        Ok(full_prompt)
    }
}

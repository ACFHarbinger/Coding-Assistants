use super::memory_recall::MemoryRecallEvent;
use super::periodic_consolidation::maybe_consolidate;
use super::prompt_builder::construct_prompt;
use crate::client::llm::{LLMClient, ModelConfig};
use crate::core::file_tools::FileTools;
use hub::HubStore;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
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
    #[serde(default)]
    pub auto_consolidate_memories: bool,
    #[serde(default = "default_consolidation_threshold")]
    pub auto_consolidation_min_clusters: usize,
    #[serde(default = "default_consolidation_cooldown_minutes")]
    pub auto_consolidation_cooldown_minutes: u64,
}

fn default_consolidation_threshold() -> usize {
    2
}
fn default_consolidation_cooldown_minutes() -> u64 {
    60
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
        app: &tauri::AppHandle,
        token: Arc<AtomicBool>,
        mut input_rx: mpsc::Receiver<String>,
    ) -> Result<String, String> {
        self.execute_phases(task, app, token, &mut input_rx).await
    }

    async fn execute_phases(
        &self,
        task: &str,
        app: &tauri::AppHandle,
        token: Arc<AtomicBool>,
        input_rx: &mut mpsc::Receiver<String>,
    ) -> Result<String, String> {
        // Keep task-scoped MCP config in the same CA_HOME-aware Hub directory
        // as the rest of this application's state.  Writing through HOME here
        // leaks an isolated/profiled task into the user's real configuration.
        let mut mcp_abs_path = None;
        if !self.config.mcp_config.is_empty() {
            let config_dir = hub::default_hub_home();
            let mcp_config_file = config_dir.join("mcp.json");

            if let Err(e) = tokio::fs::create_dir_all(&config_dir).await {
                eprintln!("Failed to create config directory {:?}: {}", config_dir, e);
            } else if let Err(e) = tokio::fs::write(&mcp_config_file, &self.config.mcp_config).await
            {
                eprintln!("Failed to write mcp.json to {:?}: {}", mcp_config_file, e);
            } else {
                mcp_abs_path = Some(mcp_config_file.to_string_lossy().to_string());
            }
        }

        let mut previous_outputs = format!("Task: {}\n", task);
        let mut final_result = String::new();

        let mut file_vector = Vec::<String>::new();
        let total_roles = self.config.roles.len();
        for (idx, role_config) in self.config.roles.iter().enumerate() {
            if token.load(Ordering::SeqCst) {
                return Err("Task cancelled".into());
            }

            let role_name = &role_config.name;
            let budget_store = HubStore::open(default_hub_dir()).ok();
            let budget_reservation = if let Some(store) = &budget_store {
                if store
                    .get_budget(role_name)
                    .map_err(|e| e.to_string())?
                    .is_some()
                {
                    Some(
                        store
                            .try_consume_budget(role_name, 1.0)
                            .map_err(|e| e.to_string())?,
                    )
                } else {
                    None
                }
            } else {
                None
            };
            let default_system = format!(
                "You are an expert {}. Work with your team to complete the task. \n\
                 Review the previous outputs and contribute your expertise. \n\
                 Focus on quality and follow best practices for the technology stack.",
                role_name
            );

            let role_names = self
                .config
                .roles
                .iter()
                .map(|role| role.name.clone())
                .collect::<Vec<_>>();
            let (prompt, recalled_memories) = construct_prompt(
                &self.file_tools,
                &role_config.config,
                task,
                &previous_outputs,
                &default_system,
                &role_names,
                &self.config.work_dir,
            )
            .await?;

            if let Some(recalled_memories) = recalled_memories {
                let _ = app.emit(
                    "agent-memory-recall",
                    MemoryRecallEvent {
                        role: role_name.clone(),
                        workspace: self.config.work_dir.clone(),
                        limit: recalled_memories.0,
                        memories: recalled_memories.1,
                    },
                );
            }

            let _ = app.emit(
                "agent-event",
                AgentEvent {
                    source: role_name.clone(),
                    event_type: "thought".into(),
                    content: prompt.clone(),
                },
            );

            let completion = self
                .interactive_completion(
                    &role_config.config,
                    &prompt,
                    app,
                    role_name,
                    token.clone(),
                    input_rx,
                    mcp_abs_path.as_deref(),
                )
                .await;

            if let Err(error) = &completion {
                if token.load(Ordering::SeqCst) {
                    if let Ok(store) = HubStore::open(default_hub_dir()) {
                        let _ = store.record_shutdown(role_name, None, task, error, None);
                    }
                }
                return Err(error.clone());
            }
            let completion = completion.expect("completion checked above");

            if let (Some(store), Some(status)) = (&budget_store, budget_reservation) {
                if status.paused {
                    let completed = if completion.is_empty() {
                        "Provider call completed without output."
                    } else {
                        "Provider call completed and its output was captured in the task transcript."
                    };
                    let _ = store.pause_for_budget(
                        role_name,
                        None,
                        task,
                        completed,
                        "Remaining workflow roles and final synthesis.",
                        None,
                    );
                    return Err(format!(
                        "agent {role_name} reached its budget ({}/{} units); handoff written",
                        status.spent_units, status.limit_units
                    ));
                }
            }
            let output = completion;

            // Persist local, provider-neutral observability counters. Exact
            // token/cache values can be supplied later by provider adapters.
            if let Ok(store) = HubStore::open(default_hub_dir()) {
                let _ = store.record_agent_metrics(
                    role_name,
                    output.lines().count() as i64,
                    output.split_whitespace().count() as i64,
                    0,
                    output.chars().count() as i64,
                );
            }

            // Save Role Report
            let filename = format!("{}.md", role_name.to_lowercase().replace(" ", "_"));
            if let Err(e) = self.file_tools.write_file(&filename, &output).await {
                eprintln!("Failed to write {}: {}", filename, e);
            }
            file_vector.push(filename);

            previous_outputs.push_str(&format!("\nOutput from {}:\n{}\n", role_name, output));
            final_result.push_str(&format!("## {} Output\n{}\n\n", role_name, output));
            if idx == total_roles - 1 {
                let mut all_contents = String::new();
                for file_path in &file_vector {
                    if let Ok(content) = self.file_tools.read_file(file_path).await {
                        all_contents.push_str(&format!("\n--- {} ---\n{}\n", file_path, content));
                    }
                }

                let summary_prompt = format!(
                    "You are a project manager. Summarize the progress made in this session based on the following outputs. \
                     Focus on key decisions, implementations, and next steps. \
                     Save this as a concise project memory for future reference.\n\n\
                     Outputs:\n{}",
                    all_contents
                );

                let summary = self
                    .client
                    .chat_completion(
                        &role_config.config,
                        &summary_prompt,
                        Some(&self.config.work_dir),
                        app,
                        "System",
                        mcp_abs_path.as_deref(),
                        Some(token.clone()),
                    )
                    .await?;

                if let Err(e) = self
                    .file_tools
                    .write_file(".agent/project_memory.md", &summary)
                    .await
                {
                    eprintln!("Failed to write project memory: {}", e);
                }
                maybe_consolidate(
                    app,
                    role_config.config.clone(),
                    self.config.work_dir.clone(),
                    self.config.auto_consolidate_memories,
                    self.config.auto_consolidation_min_clusters,
                    self.config.auto_consolidation_cooldown_minutes,
                )
                .await;
            }
        }
        Ok(final_result)
    }

    // TODO(RD2): this argument list should collapse once request state moves
    // into a dedicated struct as part of the actor-model daemon migration.
    #[allow(clippy::too_many_arguments)]
    async fn interactive_completion(
        &self,
        config: &ModelConfig,
        initial_prompt: &str,
        app: &tauri::AppHandle,
        source: &str,
        token: Arc<AtomicBool>,
        input_rx: &mut mpsc::Receiver<String>,
        mcp_config_path: Option<&str>,
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
                    app,
                    source,
                    mcp_config_path,
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
                let _ = app.emit(
                    "agent-event",
                    AgentEvent {
                        source: source.to_string(),
                        event_type: "question".into(),
                        content: question_text.clone(),
                    },
                );

                // Wait for input
                let user_input = match input_rx.recv().await {
                    Some(input) => input,
                    None => return Err("User input channel closed".into()),
                };

                // Emit acknowledgement
                let _ = app.emit(
                    "agent-event",
                    AgentEvent {
                        source: "User".into(),
                        event_type: "input".into(),
                        content: user_input.clone(),
                    },
                );

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

                    let _ = app.emit(
                        "agent-event",
                        AgentEvent {
                            source: "System".into(),
                            event_type: "authorization".into(),
                            content: auth_payload,
                        },
                    );

                    // Wait for authorization
                    let auth_response = match input_rx.recv().await {
                        Some(input) => input,
                        None => return Err("User input channel closed".into()),
                    };

                    if auth_response != "APPROVED" {
                        let _ = app.emit(
                            "agent-event",
                            AgentEvent {
                                source: "System".into(),
                                event_type: "thought".into(),
                                content: format!(
                                    "Authorization DENIED for asking {}",
                                    target_role_name
                                ),
                            },
                        );

                        history.push_str(&format!(
                            "\n\nSystem: User DENIED the request to ask {}.",
                            target_role_name
                        ));
                        continue;
                    }

                    let _ = app.emit(
                        "agent-event",
                        AgentEvent {
                            source: source.to_string(),
                            event_type: "thought".into(),
                            content: format!("Asking {}: {}", target_role_name, question),
                        },
                    );

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
                            app,
                            target_role_name,
                            mcp_config_path,
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
}

fn default_hub_dir() -> std::path::PathBuf {
    hub::default_hub_home()
}

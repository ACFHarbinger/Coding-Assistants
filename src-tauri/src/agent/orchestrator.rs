use super::budget::BudgetContext;
use super::memory_recall::MemoryRecallEvent;
use super::periodic_consolidation::maybe_consolidate;
use super::prompt_builder::construct_prompt;
use super::suborch;
use super::task_state::task_mcp_file;
use crate::client::llm::{LLMClient, ModelConfig};
use crate::core::file_tools::FileTools;
use hub::bus::InProcessBus;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
        task_id: &str,
        task: &str,
        app: &tauri::AppHandle,
        bus: &InProcessBus,
        token: Arc<AtomicBool>,
        mut input_rx: mpsc::Receiver<String>,
    ) -> Result<String, String> {
        self.execute_phases(task_id, task, app, bus, token, &mut input_rx)
            .await
    }

    async fn execute_phases(
        &self,
        task_id: &str,
        task: &str,
        app: &tauri::AppHandle,
        bus: &InProcessBus,
        token: Arc<AtomicBool>,
        input_rx: &mut mpsc::Receiver<String>,
    ) -> Result<String, String> {
        // Task-scoped MCP config (P2 / #331): each task writes its own
        // `<hub_home>/mcp-tasks/<task_id>/mcp.json` so concurrent tasks with
        // different configs cannot clobber one shared `mcp.json`. The caller
        // removes the task dir afterwards (best effort). Keep the write in
        // the CA_HOME-aware Hub directory: writing through HOME would leak an
        // isolated/profiled task into the user's real configuration.
        let mut mcp_abs_path = None;
        if !self.config.mcp_config.is_empty() {
            let mcp_config_file = task_mcp_file(&hub::default_hub_home(), task_id);

            if let Err(e) = tokio::fs::create_dir_all(mcp_config_file.parent().unwrap()).await {
                eprintln!(
                    "Failed to create config directory {:?}: {}",
                    mcp_config_file, e
                );
            } else if let Err(e) = tokio::fs::write(&mcp_config_file, &self.config.mcp_config).await
            {
                eprintln!("Failed to write mcp.json to {:?}: {}", mcp_config_file, e);
            } else {
                mcp_abs_path = Some(mcp_config_file.to_string_lossy().to_string());
            }
        }

        let mut previous_outputs = format!("Task: {}\n", task);
        let mut final_result = String::new();
        let budget = BudgetContext::open(task);

        let mut file_vector = Vec::<String>::new();
        let total_roles = self.config.roles.len();
        for (idx, role_config) in self.config.roles.iter().enumerate() {
            if token.load(Ordering::SeqCst) {
                budget.shutdown(&role_config.name, "Task cancelled");
                return Err("Task cancelled".into());
            }

            let role_name = &role_config.name;
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
                bus.emit(
                    hub::bus::TOPIC_AGENT_MEMORY_RECALL,
                    MemoryRecallEvent {
                        role: role_name.clone(),
                        workspace: self.config.work_dir.clone(),
                        limit: recalled_memories.0,
                        memories: recalled_memories.1,
                    },
                );
            }

            emit_agent(bus, role_name.clone(), "thought", prompt.clone());

            let completion = self
                .interactive_completion(
                    &role_config.config,
                    &prompt,
                    bus,
                    role_name,
                    token.clone(),
                    input_rx,
                    mcp_abs_path.as_deref(),
                    &budget,
                )
                .await;

            if let Err(error) = &completion {
                budget.shutdown_if_cancelled(role_name, &token, error);
                return Err(error.clone());
            }
            let output = completion.expect("completion checked above");

            // Persist local, provider-neutral observability counters. Exact
            // token/cache values can be supplied later by provider adapters.
            if let Ok(store) = hub::HubStore::open(hub::default_hub_home()) {
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
            if idx == total_roles - 1 && !budget.is_paused(role_name) {
                budget.deny_unless_allowed(role_name)?;
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
                        bus,
                        "System",
                        mcp_abs_path.as_deref(),
                        Some(token.clone()),
                    )
                    .await;
                if let Err(error) = &summary {
                    budget.shutdown_if_cancelled(role_name, &token, error);
                    return Err(error.clone());
                }
                let summary = summary.expect("summary checked above");

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

            let completed = if output.is_empty() {
                "Provider call completed without output."
            } else {
                "Provider call completed and its output was captured in the task transcript."
            };
            budget.handoff_if_paused(role_name, completed)?;
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
        bus: &InProcessBus,
        source: &str,
        token: Arc<AtomicBool>,
        input_rx: &mut mpsc::Receiver<String>,
        mcp_config_path: Option<&str>,
        budget: &BudgetContext,
    ) -> Result<String, String> {
        let mut history = initial_prompt.to_string();
        let mut suborch_fanout: u8 = 0;

        loop {
            budget.deny_unless_allowed(source)?;
            let response = self
                .client
                .chat_completion(
                    config,
                    &history,
                    Some(&self.config.work_dir),
                    bus,
                    source,
                    mcp_config_path,
                    Some(token.clone()),
                )
                .await?;
            if budget.is_paused(source) {
                return Ok(response);
            }

            // Check for [[ASK_USER]]
            if let Some(pos) = response.find("[[ASK_USER]]") {
                let question = response[pos + "[[ASK_USER]]".len()..].trim().to_string();
                let question_text = if question.is_empty() {
                    "Agent requesting input...".to_string()
                } else {
                    question
                };

                emit_agent(bus, source, "question", question_text.clone());

                // Wait for input
                let user_input = match input_rx.recv().await {
                    Some(input) => input,
                    None => return Err("User input channel closed".into()),
                };

                emit_agent(bus, "User", "input", user_input.clone());

                history.push_str("\n\nAgent: ");
                history.push_str(&response);
                history.push_str("\n\nUser: ");
                history.push_str(&user_input);

                // Loop again
            } else if let Some(invocation) = suborch::parse(&response) {
                let note = suborch::invoke(
                    &invocation,
                    source,
                    &response,
                    &self.config.roles,
                    &self.client,
                    &self.config.work_dir,
                    bus,
                    token.clone(),
                    mcp_config_path,
                    budget,
                    &mut suborch_fanout,
                )
                .await?;
                history.push_str(&note);
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

                    emit_agent(bus, "System", "authorization", auth_payload);

                    // Wait for authorization
                    let auth_response = match input_rx.recv().await {
                        Some(input) => input,
                        None => return Err("User input channel closed".into()),
                    };

                    if auth_response != "APPROVED" {
                        emit_agent(
                            bus,
                            "System",
                            "thought",
                            format!("Authorization DENIED for asking {}", target_role_name),
                        );

                        history.push_str(&format!(
                            "\n\nSystem: User DENIED the request to ask {}.",
                            target_role_name
                        ));
                        continue;
                    }

                    emit_agent(
                        bus,
                        source,
                        "thought",
                        format!("Asking {}: {}", target_role_name, question),
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

                    if let Err(stop) = budget.deny_unless_allowed(target_role_name) {
                        history.push_str(&format!("\n\nSystem: {stop}"));
                        continue;
                    }
                    let answer = self
                        .client
                        .chat_completion(
                            target_config,
                            &target_prompt,
                            Some(&self.config.work_dir),
                            bus,
                            target_role_name,
                            mcp_config_path,
                            Some(token.clone()),
                        )
                        .await?;
                    budget.write_exhaustion_handoff(
                        target_role_name,
                        "Answered a peer ASK_AGENT turn.",
                    );

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

fn emit_agent(
    bus: &InProcessBus,
    source: impl Into<String>,
    event_type: impl Into<String>,
    content: impl Into<String>,
) {
    bus.emit(
        hub::bus::TOPIC_AGENT_EVENT,
        AgentEvent {
            source: source.into(),
            event_type: event_type.into(),
            content: content.into(),
        },
    );
}

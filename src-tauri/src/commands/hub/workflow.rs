//! Workflow, budget, and shutdown commands.
use super::store::open_store;
use hub::{BudgetPauseOutcome, BudgetStatus, TaskRecord, TaskStatus, WorkflowStep};
#[derive(serde::Deserialize)]
pub struct CreateTaskArgs {
    pub title: String,
    pub workspace: Option<String>,
    pub steps: Vec<WorkflowStep>,
    pub max_parallel: Option<u32>,
    pub require_human_approval: Option<bool>,
}

#[tauri::command]
pub fn hub_create_task(args: CreateTaskArgs) -> Result<TaskRecord, String> {
    open_store()?
        .create_task_with_parallel(
            &args.title,
            args.workspace.as_deref(),
            &args.steps,
            args.max_parallel.unwrap_or(4),
            args.require_human_approval.unwrap_or(true),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_tasks(status: Option<String>) -> Result<Vec<TaskRecord>, String> {
    let status = status
        .as_deref()
        .map(TaskStatus::parse)
        .transpose()
        .map_err(|e| e.to_string())?;
    open_store()?.list_tasks(status).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_get_task(id: String) -> Result<TaskRecord, String> {
    open_store()?
        .get_task(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("task not found: {id}"))
}

#[tauri::command]
pub fn hub_advance_task(
    id: String,
    from: Option<String>,
    note: Option<String>,
) -> Result<TaskRecord, String> {
    open_store()?
        .advance_task(&id, from.as_deref(), note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_cancel_task(id: String) -> Result<TaskRecord, String> {
    open_store()?.cancel_task(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_complete_parallel_member(
    id: String,
    agent: String,
    note: Option<String>,
) -> Result<TaskRecord, String> {
    open_store()?
        .complete_parallel_member(&id, &agent, note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_retry_task(
    id: String,
    from: Option<String>,
    note: Option<String>,
) -> Result<TaskRecord, String> {
    open_store()?
        .retry_task(&id, from.as_deref(), note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_set_agent_budget(agent: String, limit: f64) -> Result<BudgetStatus, String> {
    open_store()?
        .set_agent_budget(&agent, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_get_budget(agent: String) -> Result<Option<BudgetStatus>, String> {
    open_store()?.get_budget(&agent).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_agent_metrics() -> Result<Vec<hub::AgentMetrics>, String> {
    open_store()?
        .list_agent_metrics()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_record_agent_metrics(
    agent: String,
    lines_written: i64,
    tokens_used: i64,
    tokens_cached: i64,
    output_chars: i64,
) -> Result<hub::AgentMetrics, String> {
    open_store()?
        .record_agent_metrics(
            &agent,
            lines_written,
            tokens_used,
            tokens_cached,
            output_chars,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_record_budget_usage(agent: String, amount: f64) -> Result<BudgetStatus, String> {
    open_store()?
        .record_budget_usage(&agent, amount)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_consume_budget(agent: String, amount: f64) -> Result<BudgetStatus, String> {
    open_store()?
        .try_consume_budget(&agent, amount)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_resume_agent(agent: String) -> Result<BudgetStatus, String> {
    open_store()?
        .resume_agent(&agent)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct PauseForBudgetArgs {
    pub agent: String,
    pub task: Option<String>,
    pub objective: String,
    pub completed: String,
    pub missing: String,
    pub delegate_to: Option<String>,
}

#[tauri::command]
pub fn hub_pause_for_budget(args: PauseForBudgetArgs) -> Result<BudgetPauseOutcome, String> {
    open_store()?
        .pause_for_budget(
            &args.agent,
            args.task.as_deref(),
            &args.objective,
            &args.completed,
            &args.missing,
            args.delegate_to.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct RecordShutdownArgs {
    pub agent: String,
    pub task: Option<String>,
    pub objective: String,
    pub reason: String,
    pub delegate_to: Option<String>,
}

#[tauri::command]
pub fn hub_record_shutdown(args: RecordShutdownArgs) -> Result<hub::ShutdownOutcome, String> {
    open_store()?
        .record_shutdown(
            &args.agent,
            args.task.as_deref(),
            &args.objective,
            &args.reason,
            args.delegate_to.as_deref(),
        )
        .map_err(|e| e.to_string())
}

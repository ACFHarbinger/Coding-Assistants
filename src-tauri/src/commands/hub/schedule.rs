//! Tauri commands for Scheduled Goals (`roadmaps/schedule.md` S7, #350).

use crate::commands::commands::store::open_store;
use hub::{
    BudgetPolicy, GateCombinator, GateCriterion, ScheduleTarget, ScheduledTask, TickSummary,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ScheduleCreateArgs {
    pub name: String,
    pub instruction: String,
    pub target: ScheduleTarget,
    pub workspace: String,
    pub fire_at: String,
    #[serde(default)]
    pub repeat_interval_secs: Option<i64>,
    #[serde(default = "default_response_timeout_secs")]
    pub response_timeout_secs: i64,
    #[serde(default)]
    pub gate_criteria: Vec<GateCriterion>,
    #[serde(default)]
    pub gate_combinator: GateCombinator,
    #[serde(default)]
    pub budget: BudgetPolicy,
    pub max_attempts: u32,
    #[serde(default)]
    pub deadline: Option<String>,
    /// Audit-only reason/priority object (P17 / #361).
    #[serde(default)]
    pub handoff_metadata: Option<serde_json::Value>,
    /// Opt-in prior-context filter. Omit for the unfiltered prompt.
    #[serde(default)]
    pub context_filter: Option<String>,
}

fn default_response_timeout_secs() -> i64 {
    3600
}

#[tauri::command]
pub fn hub_schedule_create(
    args: ScheduleCreateArgs,
    caller_agent_id: Option<String>,
) -> Result<ScheduledTask, String> {
    let store = open_store()?;
    let created_by = caller_agent_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| "human".to_string());
    let task = store
        .create_scheduled_task(
            &args.name,
            &args.instruction,
            args.target,
            &args.workspace,
            &args.fire_at,
            args.repeat_interval_secs,
            args.response_timeout_secs,
            args.gate_criteria,
            args.gate_combinator,
            args.budget,
            args.max_attempts,
            args.deadline.as_deref(),
            &created_by,
        )
        .map_err(|e| e.to_string())?;
    if args.handoff_metadata.is_none()
        && args
            .context_filter
            .as_deref()
            .is_none_or(|spec| spec.trim().is_empty())
    {
        return Ok(task);
    }
    store
        .set_schedule_handoff(
            &task.id,
            args.handoff_metadata,
            args.context_filter.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_schedule_list() -> Result<Vec<ScheduledTask>, String> {
    open_store()?
        .list_scheduled_tasks()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_schedule_get(id: String) -> Result<Option<ScheduledTask>, String> {
    open_store()?
        .get_scheduled_task(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_schedule_cancel(id: String) -> Result<ScheduledTask, String> {
    open_store()?
        .cancel_scheduled_task(&id)
        .map_err(|e| e.to_string())
}

/// Manual "run now" and the S6 in-app poller's shared entry. Goes through
/// the live-quota bridge (pause on exhausted session/weekly %) then the
/// hub tick. Headless `ca schedule tick` does not use this path.
#[tauri::command]
pub async fn hub_schedule_tick() -> Result<TickSummary, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let store = open_store()?;
        super::schedule_poller::tick_in_app(
            &store,
            &chrono::Utc::now().to_rfc3339(),
            super::quotas::probe_agent_quota,
        )
    })
    .await
    .map_err(|e| format!("schedule tick worker panic: {e}"))?
}

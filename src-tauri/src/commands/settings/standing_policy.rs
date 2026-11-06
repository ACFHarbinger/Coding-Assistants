//! Standing wake-gate policy view and per-agent budgets, split from
//! `settings.rs` to stay under the 500-LoC cap.

use hub::EffectiveOrchestrationPolicy;

use super::{open_settings_store, record_settings_audit};

/// Composes Settings' orchestration policy with the Hub's existing
/// `WakePolicy` into one typed view, so Settings is the sole *editor* of
/// standing policy even though the wake-gate bit's storage deliberately
/// stays in `HubStore` — every C10-C13 wake path already reads it there,
/// so moving it would mean touching every one of those call sites instead
/// of composing at the IPC layer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StandingPolicySnapshot {
    /// Mirrors `hub::WakePolicy::default_requires_human_gate`.
    pub confirm_wakes: bool,
    /// Mirrors `hub::WakePolicy::allow_auto_wake`. When false, any wake
    /// attempting to bypass the human gate is rejected outright rather
    /// than silently falling back to requiring approval.
    pub allow_auto_wake: bool,
    pub orchestration: EffectiveOrchestrationPolicy,
}

#[tauri::command]
pub fn settings_get_standing_policy(
    workspace: Option<String>,
) -> Result<StandingPolicySnapshot, String> {
    let orchestration = open_settings_store()
        .effective(workspace.as_deref())
        .orchestration;
    let wake_policy = super::super::store::open_store()?
        .get_wake_policy()
        .map_err(|e| e.to_string())?;
    Ok(StandingPolicySnapshot {
        confirm_wakes: wake_policy.default_requires_human_gate,
        allow_auto_wake: wake_policy.allow_auto_wake,
        orchestration,
    })
}

/// Global only: the wake human-gate is not a per-workspace concept in
/// today's `WakePolicy`.
#[tauri::command]
pub fn settings_set_confirm_wakes(value: bool) -> Result<StandingPolicySnapshot, String> {
    let hub_store = super::super::store::open_store()?;
    let mut policy = hub_store.get_wake_policy().map_err(|e| e.to_string())?;
    policy.default_requires_human_gate = value;
    hub_store
        .set_wake_policy(&policy)
        .map_err(|e| e.to_string())?;
    record_settings_audit("orchestration.confirm_wakes", "global", "update")?;
    let orchestration = open_settings_store().effective(None).orchestration;
    Ok(StandingPolicySnapshot {
        confirm_wakes: value,
        allow_auto_wake: policy.allow_auto_wake,
        orchestration,
    })
}

/// Global only, same as `settings_set_confirm_wakes`.
#[tauri::command]
pub fn settings_set_allow_auto_wake(value: bool) -> Result<StandingPolicySnapshot, String> {
    let hub_store = super::super::store::open_store()?;
    let mut policy = hub_store.get_wake_policy().map_err(|e| e.to_string())?;
    policy.allow_auto_wake = value;
    hub_store
        .set_wake_policy(&policy)
        .map_err(|e| e.to_string())?;
    record_settings_audit("orchestration.allow_auto_wake", "global", "update")?;
    let orchestration = open_settings_store().effective(None).orchestration;
    Ok(StandingPolicySnapshot {
        confirm_wakes: policy.default_requires_human_gate,
        allow_auto_wake: value,
        orchestration,
    })
}

/// Per-agent budgets, exposed through Settings' typed command surface.
/// Storage stays in `HubStore`'s existing `agent_budgets` table — every C6
/// budget flow already reads/writes it — rather than duplicating it in
/// `settings.toml`.
#[tauri::command]
pub fn settings_list_agent_budgets() -> Result<Vec<hub::BudgetStatus>, String> {
    super::super::store::open_store()?
        .list_agent_budgets()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn settings_set_agent_budget(
    agent_id: String,
    limit_units: f64,
) -> Result<hub::BudgetStatus, String> {
    let status = super::super::store::open_store()?
        .set_agent_budget(&agent_id, limit_units)
        .map_err(|e| e.to_string())?;
    record_settings_audit("orchestration.agent_budget", &agent_id, "update")?;
    Ok(status)
}

//! Runtime budget helpers for in-process LLM calls (`platform.md` P10).
//!
//! Opens HubStore per decision so the orchestrator future stays `Send`
//! (rusqlite connections cannot be held across `await`).

use hub::{BudgetStatus, HubStore, ProviderCallGate, DEFAULT_PROVIDER_CALL_UNITS};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct BudgetContext {
    task: String,
    home: Option<PathBuf>,
}

impl BudgetContext {
    pub fn open(task: &str) -> Self {
        Self {
            task: task.to_string(),
            home: None,
        }
    }

    fn store(&self) -> Option<HubStore> {
        HubStore::open(self.home.clone().unwrap_or_else(hub::default_hub_home)).ok()
    }

    pub fn gate(&self, agent_id: &str) -> Result<ProviderCallGate, String> {
        let Some(store) = self.store() else {
            return Ok(ProviderCallGate::Unmetered);
        };
        store
            .gate_provider_call(
                agent_id,
                DEFAULT_PROVIDER_CALL_UNITS,
                None,
                &self.task,
                "Provider call completed and its output was captured.",
                "Remaining workflow roles and further provider calls.",
                None,
            )
            .map_err(|error| error.to_string())
    }

    pub fn deny_unless_allowed(&self, agent_id: &str) -> Result<(), String> {
        match self.gate(agent_id)? {
            ProviderCallGate::Unmetered | ProviderCallGate::Reserved { .. } => Ok(()),
            ProviderCallGate::Stopped { status, handoff } => {
                Err(stop_message(agent_id, &status, handoff.is_some()))
            }
        }
    }

    pub fn is_paused(&self, agent_id: &str) -> bool {
        let Some(store) = self.store() else {
            return false;
        };
        store
            .get_budget(agent_id)
            .ok()
            .flatten()
            .is_some_and(|status| status.paused)
    }

    pub fn write_exhaustion_handoff(&self, agent_id: &str, completed: &str) {
        let Some(store) = self.store() else {
            return;
        };
        let Ok(Some(status)) = store.get_budget(agent_id) else {
            return;
        };
        if !status.paused {
            return;
        }
        let _ = store.pause_for_budget(
            agent_id,
            None,
            &self.task,
            completed,
            "Remaining workflow roles and further provider calls.",
            None,
        );
    }

    /// After persisting a successful turn, write the C6 handoff if this call
    /// consumed the last unit, and stop remaining roles.
    pub fn handoff_if_paused(&self, agent_id: &str, completed: &str) -> Result<(), String> {
        if !self.is_paused(agent_id) {
            return Ok(());
        }
        self.write_exhaustion_handoff(agent_id, completed);
        let status = self
            .store()
            .and_then(|store| store.get_budget(agent_id).ok().flatten())
            .ok_or_else(|| format!("agent {agent_id} is budget-paused"))?;
        Err(stop_message(agent_id, &status, true))
    }

    pub fn shutdown(&self, agent_id: &str, reason: &str) {
        let Some(store) = self.store() else {
            return;
        };
        let _ = store.record_shutdown(agent_id, None, &self.task, reason, None);
    }

    pub fn shutdown_if_cancelled(&self, agent_id: &str, token: &Arc<AtomicBool>, error: &str) {
        if token.load(Ordering::SeqCst) {
            self.shutdown(agent_id, error);
        }
    }
}

fn stop_message(agent_id: &str, status: &BudgetStatus, wrote_handoff: bool) -> String {
    if wrote_handoff {
        format!(
            "agent {agent_id} reached its budget ({}/{} units); handoff written",
            status.spent_units, status.limit_units
        )
    } else {
        format!(
            "agent {agent_id} is budget-paused ({}/{} units); resume_agent required",
            status.spent_units, status.limit_units
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn deny_unless_allowed_stops_when_paused() {
        let dir = tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        store.set_agent_budget("planner", 1.0).unwrap();
        store.try_consume_budget("planner", 1.0).unwrap();
        drop(store);
        let budget = BudgetContext {
            task: "p10".into(),
            home: Some(dir.path().to_path_buf()),
        };
        let err = budget.deny_unless_allowed("planner").unwrap_err();
        assert!(
            err.contains("budget-paused") || err.contains("reached its budget"),
            "{err}"
        );
    }
}

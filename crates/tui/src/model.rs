//! Shared Hub read model for the Ratatui TUI client (T2 / #136).
//!
//! Provides a unified, read-only snapshot of Hub data (work sessions, team roster,
//! channel messages, tasks, settings audit stream, effective settings) without depending
//! on Tauri IPC.

use hub::{
    AgentRecord, AuditEvent, EffectiveSettings, HubStore, MessageRecord, PendingGateApproval,
    SettingsStore, TaskRecord, WakeRecord, WorkSessionRecord,
};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct HubReadModel {
    pub work_sessions: Vec<WorkSessionRecord>,
    pub team_members: Vec<AgentRecord>,
    pub all_agents: Vec<AgentRecord>,
    pub channel_messages: Vec<MessageRecord>,
    pub inbox_messages: Vec<MessageRecord>,
    pub tasks: Vec<TaskRecord>,
    pub pending_gates: Vec<PendingGateApproval>,
    pub pending_wakes: Vec<WakeRecord>,
    pub audit_events: Vec<AuditEvent>,
    pub effective_settings: EffectiveSettings,
}

impl HubReadModel {
    pub fn empty(effective_settings: EffectiveSettings) -> Self {
        Self {
            work_sessions: vec![],
            team_members: vec![],
            all_agents: vec![],
            channel_messages: vec![],
            inbox_messages: vec![],
            tasks: vec![],
            pending_gates: vec![],
            pending_wakes: vec![],
            audit_events: vec![],
            effective_settings,
        }
    }

    pub fn load(
        home_dir: &Path,
        workspace: Option<&Path>,
        active_session: Option<&str>,
    ) -> Result<Self, anyhow::Error> {
        let hub_store = HubStore::open(home_dir)?;
        let settings_store = SettingsStore::open(home_dir);

        let ws_str = workspace.map(|p| p.display().to_string());
        let effective_settings = settings_store.effective(ws_str.as_deref());

        let work_sessions = hub_store.list_work_sessions()?;
        let all_agents = hub_store.list_agents()?;
        let team_members = all_agents
            .iter()
            .filter(|agent| agent.team_member)
            .cloned()
            .collect();

        let channel_id = active_session
            .map(|s| format!("session:{s}"))
            .unwrap_or_else(|| "general".to_string());

        let channel_messages = hub_store.list_channel_messages(&channel_id, 50)?;
        let inbox_messages = hub_store
            .list_messages(Some("human"), None)
            .unwrap_or_default();

        let tasks = hub_store.list_tasks(None)?;
        let pending_gates = hub_store
            .list_pending_gate_approvals(Some("pending"))
            .unwrap_or_default();
        let pending_wakes = hub_store.list_wakes(None, true).unwrap_or_default();
        let audit_events = hub_store.list_settings_audit_events()?;

        Ok(Self {
            work_sessions,
            team_members,
            all_agents,
            channel_messages,
            inbox_messages,
            tasks,
            pending_gates,
            pending_wakes,
            audit_events,
            effective_settings,
        })
    }
}

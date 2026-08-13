//! Shared Hub read model for the Ratatui TUI client (T2 / #136).
//!
//! Provides a unified, read-only snapshot of Hub data (work sessions, team roster,
//! channel messages, tasks, settings audit stream, effective settings) without depending
//! on Tauri IPC.

use hub::{
    AgentRecord, AuditEvent, EffectiveSettings, HubStore, MessageRecord, SettingsStore, TaskRecord,
    WorkSessionRecord,
};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct HubReadModel {
    pub work_sessions: Vec<WorkSessionRecord>,
    pub team_members: Vec<AgentRecord>,
    pub channel_messages: Vec<MessageRecord>,
    pub tasks: Vec<TaskRecord>,
    pub audit_events: Vec<AuditEvent>,
    pub effective_settings: EffectiveSettings,
}

impl HubReadModel {
    pub fn load(
        home_dir: &Path,
        workspace: Option<&Path>,
        active_session: Option<&str>,
    ) -> Result<Self, anyhow::Error> {
        let hub_store = HubStore::open(home_dir)?;
        let settings_store = SettingsStore::open(home_dir);

        let ws_str = workspace.map(|p| p.display().to_string());
        let effective_settings = settings_store.effective(ws_str.as_deref());

        let work_sessions = hub_store.list_work_sessions().unwrap_or_default();
        let team_members = hub_store
            .list_agents()
            .unwrap_or_default()
            .into_iter()
            .filter(|agent| agent.team_member)
            .collect();

        let channel_id = active_session
            .map(|s| format!("session:{s}"))
            .unwrap_or_else(|| "general".to_string());

        let channel_messages = hub_store
            .list_channel_messages(&channel_id, 50)
            .unwrap_or_default();

        let tasks = hub_store.list_tasks(None).unwrap_or_default();
        let audit_events = hub_store.list_settings_audit_events().unwrap_or_default();

        Ok(Self {
            work_sessions,
            team_members,
            channel_messages,
            tasks,
            audit_events,
            effective_settings,
        })
    }
}

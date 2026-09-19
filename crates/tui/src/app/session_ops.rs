//! Work session switching and creation operations for the Ratatui TUI client (T4 / #138).
//!
//! Provides desktop-parity Create/Load work session flows with membership configuration
//! and workspace scoping via HubStore.

use hub::{AgentRecord, HubStore, WorkSessionRecord};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct SessionSwitcherState {
    pub is_open: bool,
    pub selected_index: usize,
}

impl SessionSwitcherState {
    pub fn open(&mut self) {
        self.is_open = true;
        self.selected_index = 0;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn select_next(&mut self, total_sessions: usize) {
        if total_sessions > 0 {
            self.selected_index = (self.selected_index + 1) % total_sessions;
        }
    }

    pub fn select_prev(&mut self, total_sessions: usize) {
        if total_sessions > 0 {
            if self.selected_index == 0 {
                self.selected_index = total_sessions - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct CreateSessionState {
    pub is_open: bool,
    pub name_input: String,
    pub selected_members: BTreeMap<String, bool>,
    pub member_cursor: usize,
    pub focused_field: usize, // 0: Name input, 1: Member list, 2: Create button
    pub error_message: Option<String>,
}

impl CreateSessionState {
    pub fn open(&mut self, roster: &[AgentRecord]) {
        self.is_open = true;
        self.name_input.clear();
        self.error_message = None;
        self.focused_field = 0;
        self.member_cursor = 0;
        self.selected_members.clear();
        for agent in roster {
            if agent.id != "system" {
                self.selected_members.insert(agent.id.clone(), true);
            }
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.error_message = None;
    }

    pub fn toggle_current_member(&mut self, member_keys: &[String]) {
        if let Some(key) = member_keys.get(self.member_cursor) {
            let current = *self.selected_members.get(key).unwrap_or(&true);
            self.selected_members.insert(key.clone(), !current);
        }
    }

    pub fn select_next_member(&mut self, total: usize) {
        if total > 0 {
            self.member_cursor = (self.member_cursor + 1) % total;
        }
    }

    pub fn select_prev_member(&mut self, total: usize) {
        if total > 0 {
            if self.member_cursor == 0 {
                self.member_cursor = total - 1;
            } else {
                self.member_cursor -= 1;
            }
        }
    }

    pub fn create_session(
        &mut self,
        store: &HubStore,
        workspace_id: Option<&str>,
    ) -> Result<WorkSessionRecord, String> {
        let name = self.name_input.trim();
        if name.is_empty() {
            let err = String::from("Work session name cannot be empty.");
            self.error_message = Some(err.clone());
            return Err(err);
        }

        let members: Vec<String> = self
            .selected_members
            .iter()
            .filter(|(_, selected)| **selected)
            .map(|(id, _)| id.clone())
            .collect();

        match store.create_work_session_with_members_in_workspace(
            name,
            Some(&members),
            workspace_id,
        ) {
            Ok(session) => {
                self.is_open = false;
                self.error_message = None;
                self.name_input.clear();
                Ok(session)
            }
            Err(e) => {
                let err = e.to_string();
                self.error_message = Some(err.clone());
                Err(err)
            }
        }
    }
}

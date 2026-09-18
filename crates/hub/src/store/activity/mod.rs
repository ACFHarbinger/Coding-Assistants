use super::*;
use std::collections::HashSet;

mod sessions;
mod tasks;
mod types;
mod workspaces;

pub use types::*;

impl HubStore {
    /// Read-only activity query over tasks, work sessions, audit events,
    /// and harness captures (Roadmap D4 / #324).
    pub fn get_activity_view(
        &self,
        filter: &ActivityFilter,
    ) -> Result<Vec<ActivityItem>, HubError> {
        let audit_events = self.list_audit_events(false)?;
        let mut items = Vec::new();
        let target_agent = filter
            .agent
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let agent_matches = |agents: &[String]| -> bool {
            match target_agent {
                None => true,
                Some(needle) => agents.iter().any(|a| a.eq_ignore_ascii_case(needle)),
            }
        };

        let filter_kind = filter.kind.as_deref().map(str::trim).unwrap_or("all");
        let include_tasks = filter_kind == "all" || filter_kind == "task";
        let include_sessions = filter_kind == "all" || filter_kind == "work_session";

        let mut matched_audit_ids = HashSet::new();

        if include_tasks {
            self.collect_task_activities(
                filter,
                &audit_events,
                &agent_matches,
                &mut matched_audit_ids,
                &mut items,
            )?;
        }

        if include_sessions {
            self.collect_session_activities(
                filter,
                &audit_events,
                &agent_matches,
                &mut matched_audit_ids,
                &mut items,
            )?;
        }

        if filter_kind == "all" || filter_kind == "workspace" {
            self.collect_workspace_activities(
                filter,
                &audit_events,
                &agent_matches,
                &matched_audit_ids,
                &mut items,
            );
        }

        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        if let Some(limit) = filter.limit {
            if items.len() > limit {
                items.truncate(limit);
            }
        }

        Ok(items)
    }
}

use super::*;
use std::collections::HashSet;

impl HubStore {
    pub(super) fn collect_task_activities(
        &self,
        filter: &ActivityFilter,
        audit_events: &[AuditEvent],
        agent_matches: &dyn Fn(&[String]) -> bool,
        matched_audit_ids: &mut HashSet<String>,
        items: &mut Vec<ActivityItem>,
    ) -> Result<(), HubError> {
        let tasks = self.list_tasks(None)?;
        for task in tasks {
            if let Some(ref since) = filter.since {
                if task.updated_at.as_str() < since.as_str()
                    && task.created_at.as_str() < since.as_str()
                {
                    continue;
                }
            }
            if let Some(ref until) = filter.until {
                if task.created_at.as_str() > until.as_str() {
                    continue;
                }
            }
            if let Some(ref ws) = filter.workspace_path {
                if let Some(ref task_ws) = task.workspace_path {
                    if !task_ws.contains(ws) && !ws.contains(task_ws) {
                        continue;
                    }
                }
            }

            let mut agents_set = HashSet::new();
            for step in &task.steps {
                if !step.agent.trim().is_empty() {
                    agents_set.insert(step.agent.clone());
                }
            }
            for a in &task.open_agents {
                agents_set.insert(a.clone());
            }
            for a in &task.pending_agents {
                agents_set.insert(a.clone());
            }

            let mut msg_stmt = self
                .conn
                .prepare("SELECT from_agent, to_agent, body FROM messages WHERE task_id = ?1")?;
            let mut msg_count = 0usize;
            let mut extracted_commands = Vec::new();

            let msg_rows = msg_stmt.query_map(params![task.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            for row_res in msg_rows {
                let (from_a, to_a, body) = row_res?;
                msg_count += 1;
                if !from_a.trim().is_empty() && from_a != "human" {
                    agents_set.insert(from_a);
                }
                if !to_a.trim().is_empty() && to_a != "team" && !to_a.starts_with("session:") {
                    agents_set.insert(to_a);
                }
                extract_commands_from_text(&body, &mut extracted_commands, &task.created_at);
            }

            let mut wake_stmt = self
                .conn
                .prepare("SELECT target_agent FROM wake_requests WHERE reason LIKE ?1")?;
            let wake_pattern = format!("%task {}%", task.id);
            let wake_rows =
                wake_stmt.query_map(params![wake_pattern], |r| r.get::<_, String>(0))?;
            for w in wake_rows {
                agents_set.insert(w?);
            }

            let agents: Vec<String> = {
                let mut list: Vec<_> = agents_set.into_iter().collect();
                list.sort();
                list
            };

            if !agent_matches(&agents) {
                continue;
            }

            let mut files = Vec::new();
            let mut commands = extracted_commands;

            for audit in audit_events {
                let meta = parse_process_meta(&audit.process_json);
                let explicit_match = meta.task_id.as_deref() == Some(&task.id);
                let ws_match = match (&task.workspace_path, &audit.root_path) {
                    (Some(tw), ar) => tw == ar || ar.starts_with(tw),
                    _ => false,
                };
                let is_active = task.status == "pending" || task.status == "running";
                let time_match = audit.observed_at >= task.created_at
                    && (is_active || audit.observed_at <= task.updated_at);

                if explicit_match || (ws_match && time_match) {
                    matched_audit_ids.insert(audit.id.clone());
                    files.push(ActivityFileTouch {
                        path: audit.path.clone(),
                        operation: audit.operation.clone(),
                        observed_at: audit.observed_at.clone(),
                        content_hash: audit.content_hash.clone(),
                        status: audit.status.clone(),
                    });

                    let cmdline = meta.cmdline.as_ref().map(parse_cmdline).unwrap_or_default();
                    if !cmdline.is_empty() || meta.exe.is_some() {
                        let raw = if !cmdline.is_empty() {
                            cmdline.join(" ")
                        } else {
                            meta.exe.clone().unwrap_or_default()
                        };
                        commands.push(ActivityCommandRun {
                            cmdline,
                            raw,
                            exe: meta.exe,
                            observed_at: audit.observed_at.clone(),
                            attribution: meta.agent.or(meta.attribution),
                            source: "audit".into(),
                        });
                    }
                }
            }

            items.push(ActivityItem {
                id: task.id,
                kind: "task".into(),
                title: task.title,
                status: Some(task.status),
                workspace_path: task.workspace_path,
                started_at: task.created_at,
                updated_at: task.updated_at,
                agents,
                commands,
                files,
                message_count: msg_count,
                capture_count: 0,
            });
        }
        Ok(())
    }
}

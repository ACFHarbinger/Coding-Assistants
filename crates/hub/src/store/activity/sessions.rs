use super::*;
use std::collections::HashSet;

impl HubStore {
    pub(super) fn collect_session_activities(
        &self,
        filter: &ActivityFilter,
        audit_events: &[AuditEvent],
        agent_matches: &dyn Fn(&[String]) -> bool,
        matched_audit_ids: &mut HashSet<String>,
        items: &mut Vec<ActivityItem>,
    ) -> Result<(), HubError> {
        let sessions = self.list_work_sessions()?;
        for session in sessions {
            if let Some(ref since) = filter.since {
                if session.created_at.as_str() < since.as_str() {
                    continue;
                }
            }
            if let Some(ref until) = filter.until {
                if session.created_at.as_str() > until.as_str() {
                    continue;
                }
            }

            let mut agents_set: HashSet<String> = session.member_ids.into_iter().collect();

            let mut cap_stmt = self.conn.prepare(
                "SELECT id, harness, agent_id, body, created_at FROM harness_captures WHERE session_id = ?1",
            )?;
            let mut capture_count = 0usize;
            let mut session_commands = Vec::new();
            let mut latest_activity = session.created_at.clone();

            let cap_rows = cap_stmt.query_map(params![session.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            for r in cap_rows {
                let (_id, _harness, agent_id, body, created_at) = r?;
                capture_count += 1;
                if !agent_id.trim().is_empty() {
                    agents_set.insert(agent_id);
                }
                if created_at > latest_activity {
                    latest_activity = created_at.clone();
                }
                extract_commands_from_text(&body, &mut session_commands, &created_at);
            }

            let mut msg_stmt = self.conn.prepare(
                "SELECT from_agent, to_agent, body, created_at FROM messages WHERE to_agent = ?1 OR subject LIKE ?2",
            )?;
            let to_session = format!("session:{}", session.id);
            let subject_prefix = format!("channel:session:{}%", session.id);
            let mut message_count = 0usize;

            let msg_rows = msg_stmt.query_map(params![to_session, subject_prefix], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;

            for r in msg_rows {
                let (from_a, _to_a, body, created_at) = r?;
                message_count += 1;
                if !from_a.trim().is_empty() && from_a != "human" {
                    agents_set.insert(from_a);
                }
                if created_at > latest_activity {
                    latest_activity = created_at.clone();
                }
                extract_commands_from_text(&body, &mut session_commands, &created_at);
            }

            let agents: Vec<String> = {
                let mut list: Vec<_> = agents_set.into_iter().collect();
                list.sort();
                list
            };

            if !agent_matches(&agents) {
                continue;
            }

            let session_ws_path: Option<String> = if let Some(ref ws_id) = session.workspace_id {
                self.conn
                    .query_row(
                        "SELECT path FROM workspaces WHERE id = ?1",
                        params![ws_id],
                        |row| row.get(0),
                    )
                    .optional()?
            } else {
                None
            };

            if let Some(ref ws) = filter.workspace_path {
                if let Some(ref s_ws) = session_ws_path {
                    if !s_ws.contains(ws) && !ws.contains(s_ws) {
                        continue;
                    }
                }
            }

            let mut files = Vec::new();
            for audit in audit_events {
                let meta = parse_process_meta(&audit.process_json);
                let explicit_match = meta.session_id.as_deref() == Some(&session.id);
                let ws_match = match (&session_ws_path, &audit.root_path) {
                    (Some(sw), ar) => sw == ar || ar.starts_with(sw),
                    _ => false,
                };
                let time_match = audit.observed_at >= session.created_at;

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
                        session_commands.push(ActivityCommandRun {
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
                id: session.id,
                kind: "work_session".into(),
                title: session.name,
                status: Some("active".into()),
                workspace_path: session_ws_path,
                started_at: session.created_at,
                updated_at: latest_activity,
                agents,
                commands: session_commands,
                files,
                message_count,
                capture_count,
            });
        }
        Ok(())
    }
}

use super::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

impl HubStore {
    pub(super) fn collect_workspace_activities(
        &self,
        filter: &ActivityFilter,
        audit_events: &[AuditEvent],
        agent_matches: &dyn Fn(&[String]) -> bool,
        matched_audit_ids: &HashSet<String>,
        items: &mut Vec<ActivityItem>,
    ) {
        let mut ws_groups: HashMap<String, Vec<&AuditEvent>> = HashMap::new();
        for audit in audit_events {
            if !matched_audit_ids.contains(&audit.id) && audit.root_path != "settings" {
                ws_groups
                    .entry(audit.root_path.clone())
                    .or_default()
                    .push(audit);
            }
        }

        for (ws_root, events) in ws_groups {
            if let Some(ref ws) = filter.workspace_path {
                if !ws_root.contains(ws) && !ws.contains(&ws_root) {
                    continue;
                }
            }

            let mut files = Vec::new();
            let mut commands = Vec::new();
            let mut agents_set = HashSet::new();
            let mut first_time = Utc::now().to_rfc3339();
            let mut last_time = String::new();

            for audit in events {
                if audit.observed_at < first_time {
                    first_time = audit.observed_at.clone();
                }
                if audit.observed_at > last_time {
                    last_time = audit.observed_at.clone();
                }
                let meta = parse_process_meta(&audit.process_json);
                if let Some(ref a) = meta.agent {
                    agents_set.insert(a.clone());
                }
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

            let agents: Vec<String> = {
                let mut list: Vec<_> = agents_set.into_iter().collect();
                list.sort();
                list
            };

            if !agent_matches(&agents) {
                continue;
            }

            let name = Path::new(&ws_root)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| ws_root.clone());

            items.push(ActivityItem {
                id: format!("workspace:{}", ws_root),
                kind: "workspace".into(),
                title: format!("Direct Workspace Activity ({name})"),
                status: Some("observed".into()),
                workspace_path: Some(ws_root),
                started_at: first_time,
                updated_at: last_time,
                agents,
                commands,
                files,
                message_count: 0,
                capture_count: 0,
            });
        }
    }
}

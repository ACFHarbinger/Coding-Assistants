use super::*;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Event types tracked in D1 task timeline & collaboration view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventType {
    TaskCreated,
    WorkflowStep,
    Transition,
    Message,
    Wake,
    Handoff,
    Tool,
    File,
    ProviderCall,
    Gate,
}

/// Operational status of a timeline event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventStatus {
    Completed,
    InProgress,
    Paused,
    Failed,
    Pending,
}

/// A discrete chronological event in a task's lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTimelineEvent {
    pub id: String,
    pub task_id: String,
    pub timestamp: String,
    pub event_type: TimelineEventType,
    pub actor: String,
    #[serde(default)]
    pub target: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub detail: Option<String>,
    pub status: TimelineEventStatus,
    #[serde(default)]
    pub step_index: Option<i64>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Aggregated 2D timeline and collaboration view for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTimelineView {
    pub task: TaskRecord,
    pub events: Vec<TaskTimelineEvent>,
    pub involved_agents: Vec<String>,
    pub duration_seconds: Option<f64>,
    pub status: String,
    pub failure_reason: Option<String>,
    pub is_paused: bool,
}

/// Filter criteria for listing timeline events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TimelineFilter {
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
}

fn parse_ts_seconds(ts: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp())
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.and_utc().timestamp())
        })
}

impl HubStore {
    /// Retrieve the complete 2D chronological event timeline for a specific task (Roadmap D1 / #87).
    pub fn get_task_timeline(&self, task_id: &str) -> Result<TaskTimelineView, HubError> {
        let task = self
            .get_task(task_id)?
            .ok_or_else(|| HubError::NotFound(format!("task not found: {task_id}")))?;

        let mut events: Vec<TaskTimelineEvent> = Vec::new();
        let mut involved = HashSet::new();
        let mut failure_reason = None;

        // 1. Initial task creation event
        let creator = task
            .steps
            .first()
            .map(|s| s.agent.clone())
            .unwrap_or_else(|| "orchestrator".to_string());
        involved.insert(creator.clone());

        events.push(TaskTimelineEvent {
            id: format!("evt-create-{}", task.id),
            task_id: task.id.clone(),
            timestamp: task.created_at.clone(),
            event_type: TimelineEventType::TaskCreated,
            actor: creator,
            target: None,
            summary: format!("Task created: {}", task.title),
            detail: task.workspace_path.clone(),
            status: TimelineEventStatus::Completed,
            step_index: Some(0),
            metadata: Some(serde_json::json!({
                "max_parallel": task.max_parallel,
                "require_human_approval": task.require_human_approval,
            })),
        });

        // 2. Workflow steps
        for (idx, step) in task.steps.iter().enumerate() {
            let step_idx = idx as i64;
            involved.insert(step.agent.clone());

            let step_status = if step_idx < task.step_index {
                TimelineEventStatus::Completed
            } else if step_idx == task.step_index {
                match task.status.as_str() {
                    "failed" => TimelineEventStatus::Failed,
                    "paused" => TimelineEventStatus::Paused,
                    "completed" => TimelineEventStatus::Completed,
                    _ => TimelineEventStatus::InProgress,
                }
            } else {
                TimelineEventStatus::Pending
            };

            let step_time = if step_idx == 0 {
                task.created_at.clone()
            } else {
                task.updated_at.clone()
            };

            let detail = format!(
                "Instruction: {}{}{}",
                step.instruction,
                step.parallel_group
                    .as_deref()
                    .map(|g| format!(" [parallel group: {g}]"))
                    .unwrap_or_default(),
                step.peer
                    .as_deref()
                    .map(|p| format!(" [peer: {p}]"))
                    .unwrap_or_default()
            );

            events.push(TaskTimelineEvent {
                id: format!("evt-step-{}-{}", task.id, idx),
                task_id: task.id.clone(),
                timestamp: step_time,
                event_type: TimelineEventType::WorkflowStep,
                actor: step.agent.clone(),
                target: None,
                summary: format!(
                    "Step {}: {}",
                    idx + 1,
                    step.role.as_deref().unwrap_or(&step.agent)
                ),
                detail: Some(detail),
                status: step_status,
                step_index: Some(step_idx),
                metadata: Some(serde_json::json!({
                    "role": step.role,
                    "max_retries": step.max_retries,
                    "branch": step.branch,
                    "handoff_metadata": step.handoff_metadata,
                })),
            });
        }

        // 3. Task messages & handoffs
        let mut msg_stmt = self.conn.prepare(
            "SELECT id, from_agent, to_agent, body, created_at, kind, status, subject \
             FROM messages WHERE task_id = ?1 ORDER BY created_at ASC",
        )?;

        let msg_rows = msg_stmt.query_map(params![task.id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })?;

        for r in msg_rows {
            let (m_id, from_a, to_a, body, created_at, kind, status_str, subject) = r?;
            if !from_a.is_empty() && from_a != "human" {
                involved.insert(from_a.clone());
            }
            if !to_a.is_empty() && to_a != "team" && !to_a.starts_with("session:") {
                involved.insert(to_a.clone());
            }

            let (evt_type, status) = if kind == "handoff" {
                (
                    TimelineEventType::Handoff,
                    if status_str == "failed" {
                        TimelineEventStatus::Failed
                    } else {
                        TimelineEventStatus::Completed
                    },
                )
            } else if to_a == "human" || kind == "ask_user" {
                (
                    TimelineEventType::Gate,
                    if status_str == "delivered" {
                        TimelineEventStatus::Completed
                    } else {
                        TimelineEventStatus::Paused
                    },
                )
            } else {
                (TimelineEventType::Message, TimelineEventStatus::Completed)
            };

            let first_line = body.lines().next().unwrap_or("").trim();
            let summary = if evt_type == TimelineEventType::Handoff {
                format!("Handoff: {from_a} -> {to_a}")
            } else if evt_type == TimelineEventType::Gate {
                format!("Human Gate: input requested by {from_a}")
            } else {
                format!("{from_a} -> {to_a}: {first_line}")
            };

            events.push(TaskTimelineEvent {
                id: format!("evt-msg-{m_id}"),
                task_id: task.id.clone(),
                timestamp: created_at,
                event_type: evt_type,
                actor: from_a,
                target: Some(to_a),
                summary,
                detail: Some(body),
                status,
                step_index: None,
                metadata: Some(serde_json::json!({
                    "kind": kind,
                    "status": status_str,
                    "subject": subject
                })),
            });
        }

        // 4. Correlated wake requests
        let mut wake_stmt = self.conn.prepare(
            "SELECT id, target_agent, reason, status, requires_human_gate, created_at \
             FROM wake_requests WHERE reason LIKE ?1 OR message_id IN (SELECT id FROM messages WHERE task_id = ?2) \
             ORDER BY created_at ASC",
        )?;
        let wake_pattern = format!("%task {}%", task.id);
        let wake_rows = wake_stmt.query_map(params![wake_pattern, task.id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?;

        for r in wake_rows {
            let (w_id, target, reason, status_str, req_gate, created_at) = r?;
            involved.insert(target.clone());

            let status = match status_str.as_str() {
                "completed" | "resolved" => TimelineEventStatus::Completed,
                "failed" | "rejected" => TimelineEventStatus::Failed,
                "pending" => TimelineEventStatus::Paused,
                _ => TimelineEventStatus::InProgress,
            };

            let reason_text = reason.unwrap_or_else(|| "task execution".to_string());

            events.push(TaskTimelineEvent {
                id: format!("evt-wake-{w_id}"),
                task_id: task.id.clone(),
                timestamp: created_at,
                event_type: TimelineEventType::Wake,
                actor: "hub".to_string(),
                target: Some(target.clone()),
                summary: format!("Wake requested for {target}"),
                detail: Some(format!(
                    "Reason: {reason_text}{}",
                    if req_gate == 1 {
                        " [Requires human gate]"
                    } else {
                        ""
                    }
                )),
                status,
                step_index: None,
                metadata: Some(serde_json::json!({
                    "reason": reason_text,
                    "status": status_str,
                    "requires_human_gate": req_gate == 1,
                })),
            });
        }

        // 5. Audit events matching task
        let audit_events = self.list_audit_events(false)?;
        for audit in &audit_events {
            let meta = parse_process_meta(&audit.process_json);
            let matches_task =
                meta.task_id.as_deref() == Some(&task.id) || audit.process_json.contains(&task.id);

            if !matches_task {
                continue;
            }

            let actor = meta
                .agent
                .clone()
                .or_else(|| meta.attribution.clone())
                .unwrap_or_else(|| "system".to_string());
            involved.insert(actor.clone());

            let (evt_type, status, summary) = if audit.operation.starts_with("os_tool.") {
                let cmd = meta
                    .cmdline
                    .as_ref()
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_else(|| audit.path.clone());
                let st = if audit.status == "denied" || audit.status == "quarantined" {
                    TimelineEventStatus::Failed
                } else if audit.status == "pending" {
                    TimelineEventStatus::Paused
                } else {
                    TimelineEventStatus::Completed
                };
                (
                    TimelineEventType::Tool,
                    st,
                    format!("Tool execution: {cmd}"),
                )
            } else if audit.operation.starts_with("file.") {
                (
                    TimelineEventType::File,
                    TimelineEventStatus::Completed,
                    format!("File {}: {}", audit.operation, audit.path),
                )
            } else if audit.operation.starts_with("provider.") {
                (
                    TimelineEventType::ProviderCall,
                    TimelineEventStatus::Completed,
                    format!("Model inference: {}", audit.path),
                )
            } else if audit.operation.starts_with("guardrail.")
                || audit.operation.starts_with("gate.")
            {
                let st = if audit.status == "denied" {
                    TimelineEventStatus::Failed
                } else {
                    TimelineEventStatus::Completed
                };
                (
                    TimelineEventType::Gate,
                    st,
                    format!("Gate: {}", audit.operation),
                )
            } else {
                let st = if audit.operation.contains("paused") {
                    TimelineEventStatus::Paused
                } else if audit.operation.contains("failed") {
                    TimelineEventStatus::Failed
                } else {
                    TimelineEventStatus::Completed
                };
                (
                    TimelineEventType::Transition,
                    st,
                    format!("Transition: {}", audit.operation),
                )
            };

            if status == TimelineEventStatus::Failed && failure_reason.is_none() {
                failure_reason = Some(summary.clone());
            }

            events.push(TaskTimelineEvent {
                id: format!("evt-audit-{}", audit.id),
                task_id: task.id.clone(),
                timestamp: audit.observed_at.clone(),
                event_type: evt_type,
                actor,
                target: None,
                summary,
                detail: Some(format!(
                    "Operation: {} | Path: {}",
                    audit.operation, audit.path
                )),
                status,
                step_index: None,
                metadata: Some(serde_json::json!({
                    "operation": audit.operation,
                    "status": audit.status,
                    "event_hash": audit.event_hash,
                })),
            });
        }

        // 6. Chronological ordering
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // 7. Calculate total duration
        let duration_seconds = {
            let start = parse_ts_seconds(&task.created_at);
            let end = parse_ts_seconds(&task.updated_at);
            match (start, end) {
                (Some(s), Some(e)) if e >= s => Some((e - s) as f64),
                _ => None,
            }
        };

        let is_paused = task.status == "paused"
            || events
                .iter()
                .any(|e| e.status == TimelineEventStatus::Paused);

        if task.status == "failed" && failure_reason.is_none() {
            failure_reason = Some("Task terminated in failed status".to_string());
        }

        let mut involved_list: Vec<String> = involved.into_iter().collect();
        involved_list.sort();

        Ok(TaskTimelineView {
            task,
            events,
            involved_agents: involved_list,
            duration_seconds,
            status: if is_paused {
                "paused".to_string()
            } else {
                "active".to_string()
            },
            failure_reason,
            is_paused,
        })
    }
}

//! Owner review digest (Team Scorecard T4 / #344).
//!
//! Private, advisory, owner-only. Groups pending human decisions, queue age,
//! review volume, and unresolved findings by work session (U24). Remote HITL
//! (Telegram/TCP) resolves the same wake rows on the same `human` clock —
//! tagged `via` — so nothing is double-counted. No approve-all, no deny path.

use super::helpers::{is_open_security_event, is_security_class_op};
use crate::store::{HubError, HubStore, SliceStamp};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Fixed review-volume window in days. Internal bound, not a setting.
pub const DIGEST_WINDOW_DAYS: u32 = 7;

/// Which writer produced a pending human decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DigestDecisionKind {
    Wake,
    GateApproval,
    OsTool,
}

impl DigestDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wake => "wake",
            Self::GateApproval => "gate_approval",
            Self::OsTool => "os_tool",
        }
    }
}

/// Human decisions resolved inside the digest window, split by channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DigestResolutionCounts {
    pub desktop: u32,
    pub telegram: u32,
    pub tcp: u32,
    pub total: u32,
}

impl DigestResolutionCounts {
    pub fn record(&mut self, via: &str) {
        match via.trim() {
            "telegram" => self.telegram += 1,
            "tcp" => self.tcp += 1,
            _ => self.desktop += 1,
        }
        self.total += 1;
    }
}

/// One outstanding decision awaiting the owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigestPendingDecision {
    /// Row id in the source table (`wake_requests`, `pending_gate_approvals`)
    /// or the `audit_events` id for `os_tool.proposed`.
    pub id: String,
    pub kind: DigestDecisionKind,
    pub subject: String,
    /// The agent the decision is about: the sending agent (gate approval),
    /// the acting agent (`os_tool`), or the wake target.
    pub agent: Option<String>,
    pub created_at: String,
    /// `None` when the timestamp is unparseable — age unknown, never zero.
    pub age_seconds: Option<i64>,
    pub task_id: Option<String>,
    /// Optional label; present only when the source row carries it.
    pub github_issue: Option<String>,
}

/// All digest facts attached to one work session (or the ungrouped bucket).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigestSessionGroup {
    /// `None` for decisions that carry no work-session stamp.
    pub work_session_id: Option<String>,
    pub work_session_name: Option<String>,
    pub workspace_id: Option<String>,
    pub pending: Vec<DigestPendingDecision>,
    pub unresolved_findings: u32,
    pub resolved_window: DigestResolutionCounts,
}

/// Totals across every group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigestSummary {
    pub pending_count: u32,
    pub oldest_pending_at: Option<String>,
    pub unresolved_findings: u32,
    pub resolved_window: DigestResolutionCounts,
}

/// The owner-only digest attached to `ScorecardView` (T4 / #344).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerDigest {
    pub groups: Vec<DigestSessionGroup>,
    pub summary: DigestSummary,
    pub window_days: u32,
}

#[derive(Default)]
struct GroupAccumulator {
    name: Option<String>,
    workspace_id: Option<String>,
    pending: Vec<DigestPendingDecision>,
    findings: u32,
    resolved: DigestResolutionCounts,
    oldest_pending_at: Option<String>,
}

impl GroupAccumulator {
    fn push(&mut self, decision: DigestPendingDecision) {
        if self
            .oldest_pending_at
            .as_ref()
            .is_none_or(|oldest| &decision.created_at < oldest)
        {
            self.oldest_pending_at = Some(decision.created_at.clone());
        }
        self.pending.push(decision);
    }
}

type SessionIndex = BTreeMap<String, (String, Option<String>)>;

/// Entry helper: creates the group on first sight, filling the session name
/// and workspace link when the stamp names a known work session.
fn group_mut<'a>(
    groups: &'a mut BTreeMap<Option<String>, GroupAccumulator>,
    sessions: &SessionIndex,
    key: Option<String>,
) -> &'a mut GroupAccumulator {
    groups.entry(key.clone()).or_insert_with(|| {
        let (name, workspace) = key
            .as_ref()
            .and_then(|id| sessions.get(id))
            .map(|(name, workspace)| (Some(name.clone()), workspace.clone()))
            .unwrap_or((None, None));
        GroupAccumulator {
            name,
            workspace_id: workspace,
            ..GroupAccumulator::default()
        }
    })
}

fn age_seconds(created_at: &str, now: &DateTime<Utc>) -> Option<i64> {
    DateTime::parse_from_rfc3339(created_at)
        .ok()
        .map(|t| (*now - t.with_timezone(&Utc)).num_seconds())
}

fn in_window(timestamp: &str, cutoff: &DateTime<Utc>) -> bool {
    match DateTime::parse_from_rfc3339(timestamp) {
        Ok(t) => t.with_timezone(&Utc) >= *cutoff,
        Err(_) => false,
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
}

/// Subject for an `os_tool.proposed` row: the executable plus its arguments,
/// bounded so the digest never dumps a full command line.
fn os_tool_subject(process: &Value) -> String {
    let mut subject = string_field(process, "exe").unwrap_or_else(|| "os_tool".to_string());
    if let Some(cmdline) = process.get("cmdline").and_then(Value::as_array) {
        for arg in cmdline.iter().filter_map(Value::as_str).skip(1) {
            subject.push(' ');
            subject.push_str(arg);
        }
    }
    subject.truncate(80);
    subject
}

impl HubStore {
    /// Builds the owner-only digest. Advisory by construction: it reads
    /// existing rows and writes nothing.
    pub fn get_owner_digest(
        &self,
        now: DateTime<Utc>,
        window_days: u32,
    ) -> Result<OwnerDigest, HubError> {
        let cutoff = now - chrono::Duration::days(i64::from(window_days));

        // 1. Work sessions: names and workspace links for group headers.
        let mut sessions: SessionIndex = BTreeMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT id, name, workspace_id FROM work_sessions")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            for r in rows {
                let (id, name, workspace) = r?;
                sessions.insert(id, (name, workspace));
            }
        }

        let mut groups: BTreeMap<Option<String>, GroupAccumulator> = BTreeMap::new();

        // 2. Pending gate approvals addressed to the human.
        for approval in self.list_pending_gate_approvals(Some("pending"))? {
            if !approval.to_agents.iter().any(|to| to == "human") {
                continue;
            }
            let created_at = approval.created_at.clone();
            let decision = DigestPendingDecision {
                id: approval.id,
                kind: DigestDecisionKind::GateApproval,
                subject: approval.subject,
                agent: Some(approval.from_agent),
                created_at: created_at.clone(),
                age_seconds: age_seconds(&created_at, &now),
                task_id: approval.task_id,
                github_issue: None,
            };
            group_mut(&mut groups, &sessions, approval.session_id).push(decision);
        }

        // 3. Pending human-gated wakes.
        for wake in self.list_wakes(None, true)? {
            if !wake.requires_human_gate {
                continue;
            }
            let created_at = wake.created_at.clone();
            let decision = DigestPendingDecision {
                id: wake.id,
                kind: DigestDecisionKind::Wake,
                subject: wake.reason.unwrap_or_else(|| "wake".to_string()),
                agent: Some(wake.target_agent),
                created_at: created_at.clone(),
                age_seconds: age_seconds(&created_at, &now),
                task_id: None,
                github_issue: None,
            };
            group_mut(&mut groups, &sessions, None).push(decision);
        }

        // 4. Audit rows: pending `os_tool.proposed` (decisions), resolutions
        //    in the window (review volume, via-tagged), unresolved findings.
        let mut summary = DigestSummary {
            pending_count: 0,
            oldest_pending_at: None,
            unresolved_findings: 0,
            resolved_window: DigestResolutionCounts::default(),
        };

        {
            let mut stmt = self.conn.prepare(
                "SELECT id, operation, observed_at, process_json, status FROM audit_events ORDER BY rowid ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;
            for r in rows {
                let (id, operation, observed_at, process_raw, status) = r?;
                let process: Value = serde_json::from_str(&process_raw).unwrap_or(Value::Null);
                let stamp = SliceStamp::from_json_str(&process_raw);

                if operation == "os_tool.proposed" && status == "pending" {
                    let created_at = observed_at.clone();
                    let decision = DigestPendingDecision {
                        id,
                        kind: DigestDecisionKind::OsTool,
                        subject: os_tool_subject(&process),
                        agent: stamp.agent.clone(),
                        created_at: created_at.clone(),
                        age_seconds: age_seconds(&created_at, &now),
                        task_id: stamp.task_id.clone(),
                        github_issue: string_field(&process, "github_issue"),
                    };
                    group_mut(&mut groups, &sessions, stamp.work_session_id.clone())
                        .push(decision);
                    continue;
                }

                if operation == "wake.resolved" {
                    if in_window(&observed_at, &cutoff) {
                        let via =
                            string_field(&process, "via").unwrap_or_else(|| "desktop".into());
                        summary.resolved_window.record(&via);
                        group_mut(&mut groups, &sessions, None)
                            .resolved
                            .record(&via);
                    }
                    continue;
                }

                if operation == "os_tool.ran" && in_window(&observed_at, &cutoff) {
                    summary.resolved_window.record("desktop");
                    group_mut(&mut groups, &sessions, stamp.work_session_id.clone())
                        .resolved
                        .record("desktop");
                    continue;
                }

                if is_security_class_op(&operation) && is_open_security_event(&status, &operation)
                {
                    group_mut(&mut groups, &sessions, stamp.work_session_id.clone()).findings += 1;
                }
            }
        }

        // 5. Gate approvals resolved inside the window (desktop-only path).
        for approval in self.list_pending_gate_approvals(None)? {
            if approval.status == "pending" {
                continue;
            }
            if approval
                .resolved_at
                .as_deref()
                .is_some_and(|at| in_window(at, &cutoff))
            {
                summary.resolved_window.record("desktop");
                group_mut(&mut groups, &sessions, approval.session_id)
                    .resolved
                    .record("desktop");
            }
        }

        // 6. Assemble: totals, oldest-first pending order, group order.
        let mut session_groups: Vec<DigestSessionGroup> = Vec::new();
        for (key, mut acc) in groups {
            acc.pending.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            if let Some(oldest) = &acc.oldest_pending_at {
                match &summary.oldest_pending_at {
                    Some(current) if current < oldest => {}
                    _ => summary.oldest_pending_at = Some(oldest.clone()),
                }
            }
            summary.pending_count += acc.pending.len() as u32;
            summary.unresolved_findings += acc.findings;
            session_groups.push(DigestSessionGroup {
                work_session_id: key,
                work_session_name: acc.name,
                workspace_id: acc.workspace_id,
                pending: acc.pending,
                unresolved_findings: acc.findings,
                resolved_window: acc.resolved,
            });
        }

        // Groups carrying the oldest pending decision first; groups without
        // pending decisions after, tied by session id (ungrouped last).
        session_groups.sort_by(|a, b| {
            let a_key = a.pending.first().map(|d| d.created_at.clone());
            let b_key = b.pending.first().map(|d| d.created_at.clone());
            match (a_key, b_key) {
                (Some(ta), Some(tb)) => ta.cmp(&tb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.work_session_id.cmp(&b.work_session_id),
            }
        });

        Ok(OwnerDigest {
            groups: session_groups,
            summary,
            window_days,
        })
    }
}

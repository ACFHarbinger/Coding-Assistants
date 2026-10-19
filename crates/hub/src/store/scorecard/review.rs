//! Reviewer-side oversight signature for Team Scorecard T7 (#347).
//!
//! Records `review.finding` / `review.finding_corrected` on the existing
//! hash-chained audit stream (no new columns — the finding's severity,
//! origin, task link, and reviewer all live in `process_json`), then
//! projects a descriptive signature per reviewer. Same discipline as the
//! owner's own signal: independently provenanced figures, never a single
//! reviewer-quality score, no latency-only alerts.

use super::types::ReviewerSignatureRow;
use super::types::ScorecardFigure;
use crate::store::{AuditEvent, HubError, HubStore, SliceStamp};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const REVIEW_FINDING_OP: &str = "review.finding";
pub const REVIEW_FINDING_CORRECTED_OP: &str = "review.finding_corrected";

/// Origin of a review finding (Codex's T1 classification). Distinguishing
/// "your code was wrong" from "the requirement changed" is the whole point —
/// raw fix counts penalize reviewers of moving targets.
pub const FINDING_ORIGINS: &[&str] = &[
    "implementation_defect",
    "changed_requirement",
    "clarification",
    "reviewer_preference",
];

pub const FINDING_SEVERITIES: &[&str] = &["blocking", "major", "minor", "note"];

#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub reviewer: String,
    pub task_id: Option<String>,
    pub severity: String,
    pub origin: String,
    pub note: Option<String>,
}

impl HubStore {
    /// Record one classified review finding on the audit chain. The reviewer
    /// is explicit identity (a `SliceStamp` with `registered` attribution),
    /// so the signature never guesses who reviewed what.
    pub fn record_review_finding(&self, finding: &ReviewFinding) -> Result<AuditEvent, HubError> {
        let reviewer = finding.reviewer.trim();
        if reviewer.is_empty() {
            return Err(HubError::Invalid("reviewer is required".into()));
        }
        if !FINDING_SEVERITIES.contains(&finding.severity.as_str()) {
            return Err(HubError::Invalid(format!(
                "unknown severity {:?} (expected one of {FINDING_SEVERITIES:?})",
                finding.severity
            )));
        }
        if !FINDING_ORIGINS.contains(&finding.origin.as_str()) {
            return Err(HubError::Invalid(format!(
                "unknown origin {:?} (expected one of {FINDING_ORIGINS:?})",
                finding.origin
            )));
        }
        let stamp =
            SliceStamp::registered(finding.task_id.clone(), None, Some(reviewer.to_string()));
        let mut process = json!({
            "kind": "review.finding",
            "reviewer": reviewer,
            "severity": finding.severity,
            "origin": finding.origin,
        });
        if let Some(note) = finding
            .note
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            process
                .as_object_mut()
                .expect("object")
                .insert("note".into(), json!(note));
        }
        let root = self.data_dir.clone();
        self.record_stamped_audit(&root, &root, REVIEW_FINDING_OP, process, &stamp)
    }

    /// The owner corrects a finding's classification. Correction, not
    /// deletion: the original row stays on the chain and this row points at
    /// it — the reversal signal is "corrected ÷ recorded", and the audit
    /// trail keeps both halves.
    pub fn record_review_finding_corrected(
        &self,
        finding_id: &str,
        corrected_by: &str,
        reason: &str,
    ) -> Result<AuditEvent, HubError> {
        let finding = self.get_audit_event(finding_id)?;
        if finding.operation != REVIEW_FINDING_OP {
            return Err(HubError::Invalid(format!(
                "audit event {finding_id} is not a review finding"
            )));
        }
        let corrected_by = corrected_by.trim();
        if corrected_by.is_empty() || reason.trim().is_empty() {
            return Err(HubError::Invalid(
                "corrected_by and reason are required".into(),
            ));
        }
        let meta = SliceStamp::from_json_str(&finding.process_json);
        let process = json!({
            "kind": "review.finding_corrected",
            "finding_id": finding_id,
            "original_reviewer": meta.agent,
            "corrected_by": corrected_by,
            "reason": reason.trim(),
        });
        let root = self.data_dir.clone();
        self.record_audit_event(
            &root,
            &root,
            REVIEW_FINDING_CORRECTED_OP,
            &process.to_string(),
            None,
        )
    }

    /// Descriptive signature per reviewer agent. Rows exist for every
    /// standing reviewer (T2 `oversight_role`) plus any agent that recorded
    /// findings without the role — omission would hide the data.
    pub(crate) fn reviewer_signature(&self) -> Result<Vec<ReviewerSignatureRow>, HubError> {
        let mut reviewers: Vec<(String, String)> = Vec::new();
        {
            let has_oversight = self
                .conn
                .prepare("SELECT oversight_role FROM agents LIMIT 1")
                .is_ok();
            if has_oversight {
                let mut stmt = self.conn.prepare(
                    "SELECT id, display_name FROM agents WHERE oversight_role = 'reviewer' ORDER BY id",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                reviewers.extend(rows.flatten());
            }
        }

        let mut recorded: HashMap<String, u64> = HashMap::new();
        let mut tasks: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        let mut finding_owner: HashMap<String, String> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, process_json FROM audit_events WHERE operation = ?1 ORDER BY rowid ASC",
            )?;
            let rows = stmt.query_map(params![REVIEW_FINDING_OP], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for (id, raw) in rows.flatten() {
                let Ok(value) = serde_json::from_str::<Value>(&raw) else {
                    continue;
                };
                let reviewer = value
                    .get("reviewer")
                    .or_else(|| value.get("agent"))
                    .and_then(Value::as_str)
                    .unwrap_or("unattributed")
                    .to_string();
                *recorded.entry(reviewer.clone()).or_default() += 1;
                finding_owner.insert(id, reviewer.clone());
                if let Some(task) = value
                    .get("task_id")
                    .and_then(Value::as_str)
                    .filter(|t| !t.is_empty())
                {
                    tasks
                        .entry(reviewer.clone())
                        .or_default()
                        .insert(task.to_string());
                }
            }
        }

        let mut corrected: HashMap<String, u64> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT process_json FROM audit_events WHERE operation = ?1 ORDER BY rowid ASC",
            )?;
            let rows = stmt.query_map(params![REVIEW_FINDING_CORRECTED_OP], |row| {
                row.get::<_, String>(0)
            })?;
            for raw in rows.flatten() {
                let Ok(value) = serde_json::from_str::<Value>(&raw) else {
                    continue;
                };
                let finding_id = value
                    .get("finding_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if let Some(owner) = finding_owner.get(finding_id) {
                    *corrected.entry(owner.clone()).or_default() += 1;
                }
            }
        }

        for reviewer in recorded.keys() {
            if !reviewers.iter().any(|(id, _)| id == reviewer) {
                reviewers.push((reviewer.clone(), reviewer.clone()));
            }
        }

        let zero_evidence = self.zero_evidence_approvals()?;
        let count_figure = |value: u64, unit: &str| {
            if value > 0 {
                ScorecardFigure::measured(value as f64, unit)
            } else {
                ScorecardFigure::unknown(unit)
            }
        };

        let mut out = Vec::new();
        for (id, name) in reviewers {
            let rec = recorded.get(&id).copied().unwrap_or(0);
            let corr = corrected.get(&id).copied().unwrap_or(0);
            let task_set = tasks.get(&id).cloned().unwrap_or_default();
            let fix_find_rate = if !task_set.is_empty() {
                ScorecardFigure::measured(rec as f64 / task_set.len() as f64, "findings/task")
            } else {
                ScorecardFigure::unknown("findings/task")
            };
            let reversal_rate = if rec > 0 {
                ScorecardFigure::measured(corr as f64 / rec as f64, "corrected/recorded")
            } else {
                ScorecardFigure::unknown("corrected/recorded")
            };
            out.push(ReviewerSignatureRow {
                reviewer_id: id.clone(),
                display_name: name,
                findings_recorded: count_figure(rec, "findings"),
                findings_corrected_by_owner: count_figure(corr, "findings"),
                fix_find_rate,
                owner_override_reversal_rate: reversal_rate,
                zero_evidence_approvals: count_figure(
                    zero_evidence.get(&id).copied().unwrap_or(0),
                    "approvals",
                ),
                // Reviewer decision timestamps do not exist in the Hub; see
                // the type's docs. Unknown, never a guessed proxy.
                review_dwell: ScorecardFigure::unknown("seconds"),
            });
        }
        Ok(out)
    }

    /// Resolved gate approvals (either verdict) whose linked task carries no
    /// stamped evidence rows at all. Counted per slice claimant (`from_agent`)
    /// — the approval is about their slice.
    pub(crate) fn zero_evidence_approvals(&self) -> Result<HashMap<String, u64>, HubError> {
        let mut counts: HashMap<String, u64> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT from_agent, task_id FROM pending_gate_approvals \
             WHERE status IN ('approved','rejected') AND task_id IS NOT NULL AND task_id != ''",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for (from_agent, task_id) in rows.flatten() {
            let pattern = format!("%\"task_id\":\"{task_id}\"%");
            let has_evidence: Option<i64> = self
                .conn
                .query_row(
                    "SELECT 1 FROM audit_events WHERE process_json LIKE ?1 LIMIT 1",
                    params![pattern],
                    |row| row.get(0),
                )
                .optional()?;
            if has_evidence.is_none() {
                *counts.entry(from_agent).or_default() += 1;
            }
        }
        Ok(counts)
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod review_tests;

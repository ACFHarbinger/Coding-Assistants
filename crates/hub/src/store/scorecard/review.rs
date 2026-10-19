//! Reviewer-side oversight signature for Team Scorecard T7 (#347).
//!
//! Records `review.finding` / `review.finding_corrected` on the existing
//! hash-chained audit stream (no new columns — the finding's severity,
//! origin, task link, and reviewer all live in `process_json`), then
//! projects a descriptive signature per reviewer. Same discipline as the
//! owner's own signal: independently provenanced figures, never a single
//! reviewer-quality score, no latency-only alerts.
//!
//! Review 2026-10-19 (Codex) corrections baked in:
//! - findings attribute only to enrolled roster agents, and identical
//!   re-submissions dedupe to the existing row (idempotent recording);
//! - corrections require actual owner authority, and the reversal rate
//!   counts distinct owner-corrected findings — never above 100%;
//! - zero-evidence approvals stay `unknown`: gate decisions record no
//!   reviewer identity, a tagged send is not proof of evidence inspection,
//!   and JSON substring matching misses equivalent formatting;
//! - the per-task figure is scoped to tasks with a documented finding — a
//!   rate across all reviewed tasks needs a review record the Hub does not
//!   yet have.

use super::types::ReviewerSignatureRow;
use super::types::ScorecardFigure;
use crate::store::{AuditEvent, HubError, HubStore, SliceStamp};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub const REVIEW_FINDING_OP: &str = "review.finding";
pub const REVIEW_FINDING_CORRECTED_OP: &str = "review.finding_corrected";

/// The only actor allowed to correct a finding's classification.
pub const REVIEW_CORRECTION_OWNER: &str = "human";

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

/// Content key for idempotent recording — an identical re-submission is a
/// retry, not a new finding.
fn same_finding(value: &Value, finding: &ReviewFinding) -> bool {
    let str_field = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let opt_eq = |key: &str, expected: &Option<String>| match expected {
        Some(expected) => str_field(key) == Some(expected.as_str()),
        None => str_field(key).is_none(),
    };
    str_field("reviewer") == Some(finding.reviewer.trim())
        && str_field("severity") == Some(finding.severity.as_str())
        && str_field("origin") == Some(finding.origin.as_str())
        && opt_eq("task_id", &finding.task_id)
        && opt_eq(
            "note",
            &finding.note.as_ref().map(|note| note.trim().to_string()),
        )
}

impl HubStore {
    /// Record one classified review finding on the audit chain. The reviewer
    /// must be an enrolled roster agent — attribution is a roster fact, not
    /// a free-text claim — and an identical existing finding is returned
    /// unchanged so double-submission cannot inflate the counts.
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
        let enrolled: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM agents WHERE id = ?1",
                rusqlite::params![reviewer],
                |row| row.get(0),
            )
            .ok();
        if enrolled.is_none() {
            return Err(HubError::Invalid(format!(
                "reviewer {reviewer:?} is not an enrolled roster agent"
            )));
        }

        // Idempotent dedupe: identical content returns the existing row.
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, process_json FROM audit_events WHERE operation = ?1 ORDER BY rowid DESC",
            )?;
            let rows = stmt.query_map(rusqlite::params![REVIEW_FINDING_OP], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for (id, raw) in rows.flatten() {
                if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                    if same_finding(&value, finding) {
                        return self.get_audit_event(&id);
                    }
                }
            }
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

    /// The owner corrects a finding's classification. `actor` must be the
    /// owner — corrections are owner overrides, and counting them as such
    /// requires actual owner authority, not any nonempty string. Correction,
    /// not deletion: the original row stays on the chain and this row points
    /// at it. Repeated corrections of one finding count once in the
    /// signature (distinct finding ids).
    pub fn record_review_finding_corrected(
        &self,
        actor: &str,
        finding_id: &str,
        corrected_by: &str,
        reason: &str,
    ) -> Result<AuditEvent, HubError> {
        if actor.trim() != REVIEW_CORRECTION_OWNER {
            return Err(HubError::Invalid(
                "only the owner can correct a review finding".into(),
            ));
        }
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
    /// standing reviewer (T2 `oversight_role`) plus any enrolled agent that
    /// recorded findings without the role — omission would hide the data.
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
        let mut tasks: HashMap<String, HashSet<String>> = HashMap::new();
        // finding id -> original reviewer (corrections resolve through this).
        let mut finding_owner: HashMap<String, String> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, process_json FROM audit_events WHERE operation = ?1 ORDER BY rowid ASC",
            )?;
            let rows = stmt.query_map(rusqlite::params![REVIEW_FINDING_OP], |row| {
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

        // Distinct owner-corrected findings per original reviewer: repeated
        // corrections of one finding never push the rate past 100%.
        let mut corrected: HashMap<String, HashSet<String>> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT process_json FROM audit_events WHERE operation = ?1 ORDER BY rowid ASC",
            )?;
            let rows = stmt.query_map(rusqlite::params![REVIEW_FINDING_CORRECTED_OP], |row| {
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
                    corrected
                        .entry(owner.clone())
                        .or_default()
                        .insert(finding_id.to_string());
                }
            }
        }

        for reviewer in recorded.keys() {
            if !reviewers.iter().any(|(id, _)| id == reviewer) {
                reviewers.push((reviewer.clone(), reviewer.clone()));
            }
        }

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
            let corr = corrected.get(&id).map(|set| set.len() as u64).unwrap_or(0);
            let task_set = tasks.get(&id).cloned().unwrap_or_default();
            let findings_per_task = if !task_set.is_empty() {
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
                findings_per_task,
                owner_override_reversal_rate: reversal_rate,
                // Gate decisions record no reviewer identity, a tagged send
                // is not proof evidence was inspected, and substring JSON
                // matching misses equivalent formatting — so this stays
                // honestly unknown until real decision actors exist.
                zero_evidence_approvals: ScorecardFigure::unknown("approvals"),
                // Reviewer decision timestamps do not exist in the Hub; see
                // the type's docs. Unknown, never a guessed proxy.
                review_dwell: ScorecardFigure::unknown("seconds"),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod review_tests;

//! Reviewer-side signature regressions for Team Scorecard T7 (#347),
//! revised per the 2026-10-19 review: owner authority on corrections,
//! distinct-finding counting, idempotent finding recording, roster
//! attribution, and honestly-unknown zero-evidence approvals.

use super::*;
use crate::store::{FigureProvenance, HubStore};
use rusqlite::params;
use tempfile::tempdir;

#[test]
fn findings_per_task_excludes_findings_without_a_task() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .record_review_finding(&finding(
            "chat",
            Some("task-a"),
            "major",
            "implementation_defect",
        ))
        .unwrap();
    store
        .record_review_finding(&finding("chat", None, "minor", "clarification"))
        .unwrap();
    let rows = store.reviewer_signature().unwrap();
    let row = rows.iter().find(|r| r.reviewer_id == "chat").unwrap();
    assert_eq!(row.findings_recorded.value, Some(2.0));
    assert_eq!(row.findings_per_task.value, Some(1.0));
}

fn finding(reviewer: &str, task: Option<&str>, severity: &str, origin: &str) -> ReviewFinding {
    ReviewFinding {
        reviewer: reviewer.into(),
        task_id: task.map(str::to_string),
        severity: severity.into(),
        origin: origin.into(),
        note: Some("checked against the slice's stamped rows".into()),
    }
}

/// Resolved gate approvals exist in the Hub but must NOT feed the
/// reviewer signature: no recorded reviewer identity, and a send is not
/// proof evidence was inspected.
fn resolved_approval(store: &HubStore, id: &str, from_agent: &str, task_id: &str) {
    store
        .conn
        .execute(
            "INSERT INTO pending_gate_approvals \
             (id, subject, from_agent, to_agents_json, is_task, is_wake, body, \
              workspace_path, task_id, session_id, reason, status, created_at, resolved_at) \
             VALUES (?1, 's', ?2, '[\"human\"]', 1, 0, 'b', NULL, ?3, NULL, 'r', 'approved', \
                     '2026-10-19T10:00:00Z', '2026-10-19T10:05:00Z')",
            params![id, from_agent, task_id],
        )
        .unwrap();
}

#[test]
fn finding_rejects_bad_severity_origin_blank_and_unknown_reviewer() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    assert!(store
        .record_review_finding(&finding(
            "chat",
            Some("t"),
            "fatal",
            "implementation_defect"
        ))
        .is_err());
    assert!(store
        .record_review_finding(&finding("chat", Some("t"), "major", "taste"))
        .is_err());
    assert!(store
        .record_review_finding(&finding("  ", Some("t"), "major", "clarification"))
        .is_err());
    // Attribution is a roster fact: an unenrolled name is rejected.
    assert!(store
        .record_review_finding(&finding(
            "not-an-agent",
            Some("t"),
            "major",
            "clarification"
        ))
        .is_err());
    assert!(store
        .record_review_finding(&finding("chat", None, "note", "reviewer_preference"))
        .is_ok());
}

#[test]
fn finding_roundtrips_through_the_audit_chain() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let event = store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "major",
            "implementation_defect",
        ))
        .unwrap();
    assert_eq!(event.operation, REVIEW_FINDING_OP);
    let json: serde_json::Value = serde_json::from_str(&event.process_json).unwrap();
    assert_eq!(json["reviewer"], "chat");
    assert_eq!(json["severity"], "major");
    assert_eq!(json["origin"], "implementation_defect");
    assert_eq!(json["task_id"], "task-1");
    assert_eq!(json["agent"], "chat");
    assert_eq!(json["attribution"], "registered");
    store.verify_audit_chain().unwrap();
}

#[test]
fn identical_finding_resubmission_dedupes_to_one_row() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let first = store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "major",
            "implementation_defect",
        ))
        .unwrap();
    let second = store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "major",
            "implementation_defect",
        ))
        .unwrap();
    assert_eq!(first.id, second.id, "retry returns the existing row");

    // A genuinely different finding (different origin) still records.
    let other = store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "major",
            "changed_requirement",
        ))
        .unwrap();
    assert_ne!(first.id, other.id);

    let rows = store.reviewer_signature().unwrap();
    let chat = rows.iter().find(|r| r.reviewer_id == "chat").unwrap();
    assert_eq!(chat.findings_recorded.value, Some(2.0));
}

#[test]
fn correction_requires_owner_actor() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let recorded = store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "major",
            "implementation_defect",
        ))
        .unwrap();

    // Not a finding row: rejected.
    assert!(store
        .record_review_finding_corrected("human", "no-such-id", "human", "wrong call")
        .is_err());
    // Non-owner actor: rejected even with a plausible corrected_by.
    assert!(store
        .record_review_finding_corrected("chat", &recorded.id, "human", "self-correction")
        .is_err());
    // Owner actor: accepted.
    assert!(store
        .record_review_finding_corrected("human", &recorded.id, "human", "requirement moved")
        .is_ok());
}

#[test]
fn repeated_corrections_of_one_finding_count_once() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let one = store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "major",
            "implementation_defect",
        ))
        .unwrap();
    store
        .record_review_finding(&finding("chat", Some("task-1"), "minor", "clarification"))
        .unwrap();

    store
        .record_review_finding_corrected("human", &one.id, "human", "first reason")
        .unwrap();
    store
        .record_review_finding_corrected("human", &one.id, "human", "second reason")
        .unwrap();

    let rows = store.reviewer_signature().unwrap();
    let chat = rows.iter().find(|r| r.reviewer_id == "chat").unwrap();
    // Two recorded findings, ONE distinct corrected finding.
    assert_eq!(chat.findings_recorded.value, Some(2.0));
    assert_eq!(chat.findings_corrected_by_owner.value, Some(1.0));
    assert_eq!(chat.owner_override_reversal_rate.value, Some(0.5));
}

#[test]
fn correction_links_back_to_the_original_reviewer() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let recorded = store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "major",
            "implementation_defect",
        ))
        .unwrap();
    let corrected = store
        .record_review_finding_corrected(
            "human",
            &recorded.id,
            "human",
            "requirement moved mid-slice",
        )
        .unwrap();
    assert_eq!(corrected.operation, REVIEW_FINDING_CORRECTED_OP);
    let json: serde_json::Value = serde_json::from_str(&corrected.process_json).unwrap();
    assert_eq!(json["finding_id"], recorded.id);
    assert_eq!(json["original_reviewer"], "chat");
    assert_eq!(json["corrected_by"], "human");
    store.verify_audit_chain().unwrap();
}

#[test]
fn signature_composes_figures_and_keeps_zero_evidence_unknown() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.upsert_agent("chat", "Chat").unwrap();
    store
        .set_oversight_role("human", "chat", Some("reviewer"))
        .unwrap();

    // 3 findings across 2 distinct tasks → findings/documented-task 1.5.
    store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "major",
            "implementation_defect",
        ))
        .unwrap();
    store
        .record_review_finding(&finding(
            "chat",
            Some("task-1"),
            "minor",
            "changed_requirement",
        ))
        .unwrap();
    store
        .record_review_finding(&finding(
            "chat",
            Some("task-2"),
            "blocking",
            "implementation_defect",
        ))
        .unwrap();
    // A finding by a non-reviewer still surfaces (as its own row).
    store
        .record_review_finding(&finding("grok", Some("task-1"), "note", "clarification"))
        .unwrap();

    // Resolved approvals exist — including one for the exact task a finding
    // references — and the zero-evidence figure must STILL be unknown:
    // there is no recorded reviewer identity on these decisions.
    resolved_approval(&store, "ga-1", "chat", "task-1");
    resolved_approval(&store, "ga-2", "chat", "task-none");

    let rows = store.reviewer_signature().unwrap();
    let chat = rows.iter().find(|r| r.reviewer_id == "chat").unwrap();
    assert_eq!(chat.findings_recorded.value, Some(3.0));
    assert_eq!(chat.findings_per_task.value, Some(1.5));
    assert_eq!(chat.findings_per_task.unit, "findings/task");
    assert_eq!(
        chat.zero_evidence_approvals.provenance,
        FigureProvenance::Unknown,
        "no decision actors recorded — approvals never masquerade as evidence"
    );

    let grok = rows.iter().find(|r| r.reviewer_id == "grok").unwrap();
    assert_eq!(grok.findings_recorded.value, Some(1.0));

    // A reviewer without findings: everything unknown, never zero.
    store.upsert_agent("codex", "Codex").unwrap();
    store
        .set_oversight_role("human", "codex", Some("reviewer"))
        .unwrap();
    let rows = store.reviewer_signature().unwrap();
    let codex = rows.iter().find(|r| r.reviewer_id == "codex").unwrap();
    assert_eq!(
        codex.findings_recorded.provenance,
        FigureProvenance::Unknown
    );
    assert_eq!(
        codex.findings_per_task.provenance,
        FigureProvenance::Unknown
    );
    assert_eq!(
        codex.owner_override_reversal_rate.provenance,
        FigureProvenance::Unknown
    );
    assert_eq!(
        codex.zero_evidence_approvals.provenance,
        FigureProvenance::Unknown
    );
}

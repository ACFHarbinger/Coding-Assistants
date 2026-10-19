//! Reviewer-side signature regressions for Team Scorecard T7 (#347).

use super::*;
use crate::store::{FigureProvenance, HubStore};
use rusqlite::params;
use tempfile::tempdir;

fn finding(reviewer: &str, task: Option<&str>, severity: &str, origin: &str) -> ReviewFinding {
    ReviewFinding {
        reviewer: reviewer.into(),
        task_id: task.map(str::to_string),
        severity: severity.into(),
        origin: origin.into(),
        note: Some("checked against the slice's stamped rows".into()),
    }
}

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
fn finding_rejects_unknown_severity_origin_and_blank_reviewer() {
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

    // Not a finding row: rejected.
    let other = store
        .record_review_finding(&finding("chat", Some("task-1"), "minor", "clarification"))
        .unwrap();
    let _ = other;
    assert!(store
        .record_review_finding_corrected("no-such-id", "human", "wrong call")
        .is_err());

    let corrected = store
        .record_review_finding_corrected(&recorded.id, "human", "requirement moved mid-slice")
        .unwrap();
    assert_eq!(corrected.operation, REVIEW_FINDING_CORRECTED_OP);
    let json: serde_json::Value = serde_json::from_str(&corrected.process_json).unwrap();
    assert_eq!(json["finding_id"], recorded.id);
    assert_eq!(json["original_reviewer"], "chat");
    assert_eq!(json["corrected_by"], "human");

    let rows = store.reviewer_signature().unwrap();
    let chat = rows.iter().find(|r| r.reviewer_id == "chat").unwrap();
    assert_eq!(
        chat.findings_recorded.provenance,
        FigureProvenance::Measured
    );
    assert_eq!(
        chat.findings_corrected_by_owner.provenance,
        FigureProvenance::Measured
    );
    assert_eq!(chat.owner_override_reversal_rate.value, Some(0.5));
}

#[test]
fn signature_composes_fix_rate_and_zero_evidence_approvals() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.upsert_agent("chat", "Chat").unwrap();
    store
        .set_oversight_role("human", "chat", Some("reviewer"))
        .unwrap();

    // 3 findings across 2 distinct tasks → fix-find rate 1.5.
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

    // task-evid has stamped evidence; task-none does not.
    store
        .record_tagged_send_audit("chat", "evidence", None, Some("task-evid"), None, 1)
        .unwrap();
    resolved_approval(&store, "ga-1", "chat", "task-evid");
    resolved_approval(&store, "ga-2", "chat", "task-none");

    let rows = store.reviewer_signature().unwrap();
    let chat = rows.iter().find(|r| r.reviewer_id == "chat").unwrap();
    assert_eq!(chat.findings_recorded.value, Some(3.0));
    assert_eq!(chat.fix_find_rate.value, Some(1.5));
    assert_eq!(chat.fix_find_rate.unit, "findings/task");
    assert_eq!(chat.zero_evidence_approvals.value, Some(1.0));
    assert_eq!(chat.review_dwell.provenance, FigureProvenance::Unknown);

    let grok = rows.iter().find(|r| r.reviewer_id == "grok").unwrap();
    assert_eq!(grok.findings_recorded.value, Some(1.0));

    // The reviewer without any findings yet: everything unknown, never zero.
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
    assert_eq!(codex.fix_find_rate.provenance, FigureProvenance::Unknown);
    assert_eq!(
        codex.owner_override_reversal_rate.provenance,
        FigureProvenance::Unknown
    );
}

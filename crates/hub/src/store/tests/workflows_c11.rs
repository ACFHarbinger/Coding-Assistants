//! C11 task/wake tag delivery tests, split from `workflows.rs` for the 500-LoC cap.

use super::super::*;
use tempfile::tempdir;

#[test]
fn c11_task_tag_rejects_absent_recipient_without_side_effects() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("grok", true).unwrap();
    // "outsider" has never been seen, so it starts out absent.
    let outcomes = store
        .send_tagged_message(
            "human",
            &["grok".to_string(), "outsider".to_string()],
            true,
            false,
            "ship the release",
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let grok = outcomes.iter().find(|o| o.to_agent == "grok").unwrap();
    assert!(grok.accepted);
    assert!(!grok.enrolled);
    assert!(grok.message_id.is_some());

    let outsider = outcomes.iter().find(|o| o.to_agent == "outsider").unwrap();
    assert!(!outsider.accepted);
    assert!(!outsider.enrolled);
    assert!(outsider.message_id.is_none());
    assert_eq!(outsider.policy_decision, "task_refused_not_present");
    assert!(outsider
        .reason
        .as_deref()
        .unwrap()
        .contains("not a current"));

    // No membership mutation and no message actually delivered to "outsider".
    assert!(!store.is_team_member("outsider").unwrap());
    assert!(store
        .list_messages(Some("outsider"), None)
        .unwrap()
        .is_empty());

    // Durable per-recipient audit trail survives independent of the caller.
    let replayed = store
        .list_tagged_send_outcomes(&outcomes[0].subject)
        .unwrap();
    assert_eq!(replayed.len(), 2);
}

#[test]
fn c11_wake_tag_enrolls_and_requests_wake_for_a_new_identity() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    assert!(!store.is_team_member("newbie").unwrap());

    let outcomes = store
        .send_tagged_message(
            "human",
            &["newbie".to_string()],
            false,
            true,
            "join the session and pick up C12",
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let outcome = &outcomes[0];
    assert!(outcome.accepted);
    assert!(outcome.enrolled);
    assert!(outcome.wake_requested);
    assert_eq!(outcome.policy_decision, "wake_enrolled");
    assert!(store.is_team_member("newbie").unwrap());
    let pending = store.list_wakes(Some("newbie"), true).unwrap();
    assert_eq!(pending.len(), 1);
}

#[test]
fn c11_wake_refuses_a_new_identity_when_auto_enrollment_is_disabled() {
    let dir = tempdir().unwrap();
    let mut settings = crate::SettingsStore::open(dir.path());
    settings.set_auto_enrollment_allowed(false).unwrap();
    settings.save().unwrap();

    let store = HubStore::open(dir.path()).unwrap();
    assert!(!store.is_team_member("newbie").unwrap());

    let outcomes = store
        .send_tagged_message(
            "human",
            &["newbie".to_string()],
            false,
            true,
            "join the session and pick up C12",
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let outcome = &outcomes[0];
    assert!(!outcome.accepted);
    assert!(!outcome.enrolled);
    assert!(!outcome.wake_requested);
    assert_eq!(
        outcome.policy_decision,
        "wake_refused_auto_enrollment_disabled"
    );
    // No membership mutation and no message/wake left behind.
    assert!(!store.is_team_member("newbie").unwrap());
    assert!(store.list_wakes(Some("newbie"), true).unwrap().is_empty());
}

#[test]
fn c11_wake_still_adds_an_existing_team_member_to_a_session_when_auto_enrollment_is_disabled() {
    let dir = tempdir().unwrap();
    let mut settings = crate::SettingsStore::open(dir.path());
    settings.set_auto_enrollment_allowed(false).unwrap();
    settings.save().unwrap();

    let store = HubStore::open(dir.path()).unwrap();
    // Create the session before enrolling "grok" on the team, so the
    // session's team-member auto-seed doesn't already include it — this
    // test wants "grok" present on the team but absent from the session.
    let session = store
        .create_work_session("existing member session add")
        .unwrap();
    store.set_team_member("grok", true).unwrap();
    assert!(!store.is_session_member(&session.id, "grok").unwrap());

    // "grok" is already a team member — disabling *new-identity*
    // auto-enrollment must not block adding an already-known member to a
    // session; that's a distinct, always-allowed concern.
    let outcomes = store
        .send_tagged_message(
            "human",
            &["grok".to_string()],
            false,
            true,
            "join this session",
            None,
            None,
            None,
            Some(&session.id),
        )
        .unwrap();

    let outcome = &outcomes[0];
    assert!(outcome.accepted);
    assert!(outcome.enrolled);
    assert!(store.is_session_member(&session.id, "grok").unwrap());
}

#[test]
fn c11_task_and_wake_together_apply_both_rules_per_recipient() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("grok", true).unwrap();
    let session = store.create_work_session("Cloud sync design").unwrap();

    let outcomes = store
        .send_tagged_message(
            "human",
            &["grok".to_string(), "fresh".to_string()],
            true,
            true,
            "session kickoff",
            None,
            None,
            None,
            Some(&session.id),
        )
        .unwrap();

    // "grok" is already a team+session member: task passes, wake is a no-op enroll.
    let grok = outcomes.iter().find(|o| o.to_agent == "grok").unwrap();
    assert!(grok.accepted);
    assert!(!grok.enrolled);
    assert_eq!(grok.policy_decision, "accepted");

    // "fresh" is present in neither team nor session, so the task check
    // fails first — task always wins over wake for the same recipient.
    let fresh = outcomes.iter().find(|o| o.to_agent == "fresh").unwrap();
    assert!(!fresh.accepted);
    assert!(!fresh.enrolled);
    assert_eq!(fresh.policy_decision, "task_refused_not_present");
    assert!(!store.is_team_member("fresh").unwrap());
}

#[test]
fn c11_wake_enrolls_a_team_member_into_the_session() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("claude", true).unwrap();
    store.set_team_member("grok", true).unwrap();
    let session = store.create_work_session("session enroll").unwrap();
    store.upsert_agent("later", "later").unwrap();
    store.set_team_member("later", true).unwrap();
    assert!(!store.is_session_member(&session.id, "later").unwrap());

    let outcomes = store
        .send_tagged_message(
            "human",
            &["later".to_string()],
            false,
            true,
            "join this session",
            None,
            None,
            None,
            Some(&session.id),
        )
        .unwrap();
    assert!(outcomes[0].accepted);
    assert!(outcomes[0].enrolled);
    assert_eq!(outcomes[0].policy_decision, "wake_enrolled");
    assert!(store.is_session_member(&session.id, "later").unwrap());
}

#[test]
fn c11_task_refuses_a_team_member_missing_from_the_session() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("claude", true).unwrap();
    let session = store.create_work_session("task presence").unwrap();
    store.upsert_agent("later", "later").unwrap();
    store.set_team_member("later", true).unwrap();

    let outcomes = store
        .send_tagged_message(
            "human",
            &["later".to_string()],
            true,
            false,
            "do not spawn",
            None,
            None,
            None,
            Some(&session.id),
        )
        .unwrap();
    assert!(!outcomes[0].accepted);
    assert_eq!(outcomes[0].policy_decision, "task_refused_not_present");
    assert!(!store.is_session_member(&session.id, "later").unwrap());
    assert!(store.list_messages(Some("later"), None).unwrap().is_empty());
}

#[test]
fn c11_unknown_session_fails_before_any_delivery() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.set_team_member("grok", true).unwrap();
    let error = store
        .send_tagged_message(
            "human",
            &["grok".to_string()],
            false,
            true,
            "missing session",
            None,
            None,
            None,
            Some("no-such-session"),
        )
        .unwrap_err();
    assert!(error.to_string().contains("no-such-session"));
    assert!(store.list_messages(Some("grok"), None).unwrap().is_empty());
    assert!(store
        .send_session_message(
            "human",
            "no-such-session",
            &["grok".to_string()],
            "also missing",
            None,
            None,
            None,
        )
        .is_err());
}

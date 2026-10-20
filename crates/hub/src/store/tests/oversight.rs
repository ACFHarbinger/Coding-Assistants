use super::super::*;
use chrono::Duration;
use tempfile::tempdir;

fn scope_issue(issue: &str) -> OversightExceptionScope {
    OversightExceptionScope {
        task_id: None,
        issue: Some(issue.into()),
        branch: None,
    }
}

#[test]
fn sync_lock_rejects_role_changes_before_any_state_mutation() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_oversight_role("human", "grok", Some("trial"))
        .unwrap();
    crate::sync::acquire_persisted(dir.path(), "up").unwrap();
    assert!(store
        .set_oversight_role("human", "claude", Some("lead"))
        .is_err());
    assert!(store
        .set_oversight_exception_authority("human", true)
        .is_err());
    assert!(store
        .graduate_trial("human", "grok", "implementer", "verified")
        .is_err());
    assert_eq!(store.oversight_exception_authority().unwrap(), "owner");
    let agents = store.list_agents().unwrap();
    assert_eq!(
        agents
            .iter()
            .find(|a| a.id == "grok")
            .unwrap()
            .oversight_role
            .as_deref(),
        Some("trial")
    );
    assert_eq!(
        agents
            .iter()
            .find(|a| a.id == "claude")
            .unwrap()
            .oversight_role,
        None
    );
    crate::sync::release(dir.path()).unwrap();
}

fn grant(store: &HubStore, agent: &str, to_role: &str, by: &str) -> Result<AuditEvent, HubError> {
    store.grant_oversight_exception(
        by,
        &OversightExceptionRequest {
            agent_id: agent.into(),
            to_role: to_role.into(),
            scope: scope_issue("342"),
            expires_at: None,
            reason: "cover the review gap".into(),
        },
    )
}

#[test]
fn oversight_role_is_a_separate_column_from_the_display_badge() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_agent_role("grok", Some("Lead Orchestrator"))
        .unwrap();
    store
        .set_oversight_role("human", "grok", Some("implementer"))
        .unwrap();
    let agent = store
        .list_agents()
        .unwrap()
        .into_iter()
        .find(|agent| agent.id == "grok")
        .unwrap();
    assert_eq!(agent.role.as_deref(), Some("Lead Orchestrator"));
    assert_eq!(agent.oversight_role.as_deref(), Some("implementer"));
    assert_eq!(
        agent.effective_oversight_role.as_deref(),
        Some("implementer")
    );

    let err = store.set_oversight_role("claude", "grok", Some("lead"));
    assert!(err.unwrap_err().to_string().contains("only the owner"));
    assert!(store
        .set_oversight_role("human", "grok", Some("captain"))
        .unwrap_err()
        .to_string()
        .contains("oversight role"));
}

#[test]
fn exception_changes_effective_role_without_rewriting_the_badge_or_granting_rights() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.upsert_agent("codex", "Codex").unwrap();
    store.set_agent_role("codex", Some("review lead")).unwrap();
    store
        .set_oversight_role("human", "codex", Some("reviewer"))
        .unwrap();
    let event = grant(&store, "codex", "implementer", "human").unwrap();
    assert_eq!(event.operation, "oversight.exception");
    assert_eq!(event.status, "approved");
    let body: serde_json::Value = serde_json::from_str(&event.process_json).unwrap();
    assert_eq!(body["grants_p5"], false);
    assert_eq!(body["grants_vault"], false);
    assert_eq!(body["grants_network"], false);
    assert_eq!(body["authorizes_self_review"], false);
    assert_eq!(body["from"], "reviewer");
    assert_eq!(body["to"], "implementer");

    let agent = store
        .list_agents()
        .unwrap()
        .into_iter()
        .find(|agent| agent.id == "codex")
        .unwrap();
    assert_eq!(agent.role.as_deref(), Some("review lead"));
    assert_eq!(agent.oversight_role.as_deref(), Some("reviewer"));
    assert_eq!(
        agent.effective_oversight_role.as_deref(),
        Some("implementer")
    );
    store.verify_audit_chain().unwrap();
}

#[test]
fn trial_cannot_hold_or_grant_exceptions_and_graduates_by_audit() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store.upsert_agent("deepseek", "DeepSeek").unwrap();
    store
        .set_oversight_role("human", "deepseek", Some("trial"))
        .unwrap();
    let blocked = grant(&store, "deepseek", "implementer", "human").unwrap_err();
    assert!(blocked.to_string().contains("trial"));
    let direct = store
        .set_oversight_role("human", "deepseek", Some("implementer"))
        .unwrap_err();
    assert!(direct.to_string().contains("trial_graduated"));

    let lead_attempt = store.graduate_trial("claude", "deepseek", "implementer", "trial ended");
    assert!(lead_attempt
        .unwrap_err()
        .to_string()
        .contains("only the owner"));

    let graduated = store
        .graduate_trial("human", "deepseek", "implementer", "trial ended")
        .unwrap();
    assert_eq!(graduated.oversight_role.as_deref(), Some("implementer"));
    let events = store.list_audit_events(false).unwrap();
    assert!(events.iter().any(|event| {
        event.operation == "oversight.trial_graduated"
            && event.process_json.contains("\"granted_by\":\"human\"")
            && event.process_json.contains("trial ended")
    }));
}

#[test]
fn a_lead_cannot_enable_delegated_authority_and_can_grant_only_after_the_owner_does() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_oversight_role("human", "claude", Some("lead"))
        .unwrap();
    store
        .set_oversight_role("human", "grok", Some("implementer"))
        .unwrap();
    assert_eq!(store.oversight_exception_authority().unwrap(), "owner");
    assert!(store
        .set_oversight_exception_authority("claude", true)
        .unwrap_err()
        .to_string()
        .contains("only the owner"));
    assert!(grant(&store, "grok", "reviewer", "claude")
        .unwrap_err()
        .to_string()
        .contains("only the owner"));

    store
        .set_oversight_exception_authority("human", true)
        .unwrap();
    assert_eq!(
        store.oversight_exception_authority().unwrap(),
        "delegated_lead"
    );
    let event = grant(&store, "grok", "reviewer", "claude").unwrap();
    assert!(event.process_json.contains("\"granted_by\":\"claude\""));
    assert!(store
        .set_oversight_exception_authority("claude", true)
        .is_err());
    let authority = store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .find(|event| event.operation == "oversight.authority")
        .unwrap();
    assert!(authority.process_json.contains("\"from\":\"owner\""));
    assert!(authority.process_json.contains("\"to\":\"delegated_lead\""));
}

#[test]
fn revoke_deadline_and_task_completion_end_the_latest_exception() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_oversight_role("human", "grok", Some("implementer"))
        .unwrap();
    let first = grant(&store, "grok", "observer", "human").unwrap();
    let first_id = serde_json::from_str::<serde_json::Value>(&first.process_json).unwrap()
        ["grant_id"]
        .as_str()
        .unwrap()
        .to_string();
    let second = grant(&store, "grok", "reviewer", "human").unwrap();
    assert_eq!(
        store.effective_oversight_role("grok").unwrap().as_deref(),
        Some("reviewer")
    );
    store
        .revoke_oversight_exception("human", &first_id, "superseded")
        .unwrap();
    assert_eq!(
        store.effective_oversight_role("grok").unwrap().as_deref(),
        Some("reviewer")
    );
    let second_id = serde_json::from_str::<serde_json::Value>(&second.process_json).unwrap()
        ["grant_id"]
        .as_str()
        .unwrap()
        .to_string();
    store
        .revoke_oversight_exception("human", &second_id, "owner pulled it back")
        .unwrap();
    assert_eq!(
        store.effective_oversight_role("grok").unwrap().as_deref(),
        Some("implementer")
    );

    let expires = (Utc::now() + Duration::hours(1)).to_rfc3339();
    let timed = store
        .grant_oversight_exception(
            "human",
            &OversightExceptionRequest {
                agent_id: "grok".into(),
                to_role: "lead".into(),
                scope: scope_issue("342"),
                expires_at: Some(expires),
                reason: "short cover".into(),
            },
        )
        .unwrap();
    assert_eq!(
        store.effective_oversight_role("grok").unwrap().as_deref(),
        Some("lead")
    );
    let written = store
        .sweep_oversight_exceptions_at(Utc::now() + Duration::hours(2))
        .unwrap();
    assert_eq!(written, 1);
    assert_eq!(
        store.effective_oversight_role("grok").unwrap().as_deref(),
        Some("implementer")
    );
    assert!(timed
        .process_json
        .contains("\"authorizes_self_review\":false"));

    let task = store
        .create_task(
            "finish slice",
            None,
            &[WorkflowStep {
                agent: "grok".into(),
                role: None,
                instruction: "implement".into(),
                max_retries: 0,
                parallel_group: None,
                peer: None,
                branch: None,
            }],
        )
        .unwrap();
    store
        .grant_oversight_exception(
            "human",
            &OversightExceptionRequest {
                agent_id: "grok".into(),
                to_role: "reviewer".into(),
                scope: OversightExceptionScope {
                    task_id: Some(task.id.clone()),
                    issue: None,
                    branch: None,
                },
                expires_at: None,
                reason: "review this task".into(),
            },
        )
        .unwrap();
    assert_eq!(
        store.effective_oversight_role("grok").unwrap().as_deref(),
        Some("reviewer")
    );
    store.advance_task(&task.id, Some("human"), None).unwrap();
    let done = store.advance_task(&task.id, Some("human"), None).unwrap();
    assert_eq!(done.status, "done");
    assert_eq!(
        store.effective_oversight_role("grok").unwrap().as_deref(),
        Some("implementer")
    );
    let expired = store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .filter(|event| {
            event.operation == "oversight.exception" && event.process_json.contains("\"expire\"")
        })
        .count();
    assert!(expired >= 2);
}

use super::super::*;
use tempfile::tempdir;

#[test]
fn c4_task_policy_controls_wake_gate() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: true,
        })
        .unwrap();
    let steps = vec![WorkflowStep {
        agent: "claude".into(),
        role: None,
        instruction: "Run the delegated step.".into(),
        max_retries: 0,
        parallel_group: None,
        peer: None,
        branch: None,
        sub_task_id: None,
        handoff_metadata: None,
        context_filter: None,
    }];
    let task = store
        .create_task_with_parallel("ungated task", None, &steps, 1, false)
        .unwrap();
    store.advance_task(&task.id, Some("human"), None).unwrap();
    let wakes = store.list_wakes(Some("claude"), true).unwrap();
    assert_eq!(wakes.len(), 1);
    assert!(!wakes[0].requires_human_gate);
    let stored = store.get_task(&task.id).unwrap().unwrap();
    assert!(!stored.require_human_approval);
    assert_eq!(stored.status, "running");
}

fn claude_step() -> Vec<WorkflowStep> {
    vec![WorkflowStep {
        agent: "claude".into(),
        role: None,
        instruction: "Run the delegated step.".into(),
        max_retries: 0,
        parallel_group: None,
        peer: None,
        branch: None,
        sub_task_id: None,
        handoff_metadata: None,
        context_filter: None,
    }]
}

#[test]
fn default_gate_parks_task_at_input_required_until_delivered() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task_with_parallel("gated task", None, &claude_step(), 1, true)
        .unwrap();
    let advanced = store.advance_task(&task.id, Some("human"), None).unwrap();
    assert_eq!(advanced.status, "input_required");
    let wakes = store.list_wakes(Some("claude"), true).unwrap();
    assert_eq!(wakes.len(), 1);
    assert!(wakes[0].requires_human_gate);
    store
        .set_wake_status(&wakes[0].id, WakeStatus::Delivered)
        .unwrap();
    let resumed = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(resumed.status, "running");
}

#[test]
fn policy_forced_gate_is_auth_required_and_cancel_fails_the_task() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: true,
            allow_auto_wake: true,
        })
        .unwrap();
    let task = store
        .create_task_with_parallel("policy gate", None, &claude_step(), 1, false)
        .unwrap();
    let advanced = store.advance_task(&task.id, Some("human"), None).unwrap();
    assert_eq!(advanced.status, "auth_required");
    let blocked = store
        .advance_task(&task.id, Some("human"), None)
        .expect_err("gates block advancement");
    assert!(
        blocked
            .to_string()
            .contains("approval gates are still unresolved"),
        "{blocked}"
    );
    let wakes = store.list_wakes(Some("claude"), true).unwrap();
    assert!(wakes[0].requires_human_gate);
    store
        .set_wake_status(&wakes[0].id, WakeStatus::Cancelled)
        .unwrap();
    let failed = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(failed.status, "failed");
}

fn parallel_impl_steps() -> Vec<WorkflowStep> {
    ["dev_a", "dev_b"]
        .into_iter()
        .map(|agent| WorkflowStep {
            agent: agent.into(),
            role: None,
            instruction: format!("implement {agent}"),
            max_retries: 0,
            parallel_group: Some("impl".into()),
            peer: None,
            branch: None,
            sub_task_id: None,
            handoff_metadata: None,
            context_filter: None,
        })
        .collect()
}

#[test]
fn parallel_delivery_stays_gated_until_every_member_wake_is_delivered() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task_with_parallel("parallel gate", None, &parallel_impl_steps(), 2, true)
        .unwrap();
    let advanced = store.advance_task(&task.id, Some("human"), None).unwrap();
    assert_eq!(advanced.status, "input_required");
    let last = advanced.last_message_id.expect("last handoff");
    let wakes = store.list_wakes(None, true).unwrap();
    assert_eq!(wakes.len(), 2);
    let latest = wakes
        .iter()
        .find(|wake| wake.message_id.as_deref() == Some(last.as_str()))
        .expect("wake for the last handoff");
    let earlier = wakes
        .iter()
        .find(|wake| wake.message_id.as_deref() != Some(last.as_str()))
        .expect("wake for the earlier handoff");
    store
        .set_wake_status(&latest.id, WakeStatus::Delivered)
        .unwrap();
    assert_eq!(
        store.get_task(&task.id).unwrap().unwrap().status,
        "input_required"
    );
    store
        .set_wake_status(&earlier.id, WakeStatus::Delivered)
        .unwrap();
    assert_eq!(store.get_task(&task.id).unwrap().unwrap().status, "running");
}

#[test]
fn parallel_cancel_fails_only_after_the_attempt_is_fully_resolved() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task_with_parallel("parallel cancel", None, &parallel_impl_steps(), 2, true)
        .unwrap();
    let advanced = store.advance_task(&task.id, Some("human"), None).unwrap();
    let last = advanced.last_message_id.expect("last handoff");
    let wakes = store.list_wakes(None, true).unwrap();
    let earlier = wakes
        .iter()
        .find(|wake| wake.message_id.as_deref() != Some(last.as_str()))
        .expect("earlier member wake");
    let latest = wakes
        .iter()
        .find(|wake| wake.message_id.as_deref() == Some(last.as_str()))
        .expect("latest member wake");
    store
        .set_wake_status(&earlier.id, WakeStatus::Cancelled)
        .unwrap();
    let waiting = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(waiting.status, "input_required");
    assert!(!waiting.open_agents.is_empty());
    store
        .set_wake_status(&latest.id, WakeStatus::Delivered)
        .unwrap();
    let failed = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(failed.status, "failed");
    assert!(failed.open_agents.is_empty());
}

#[test]
fn queued_refill_uses_the_same_attempt_label_and_unblocks_when_resolved() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task_with_parallel("queued gate", None, &parallel_impl_steps(), 1, true)
        .unwrap();
    let advanced = store.advance_task(&task.id, Some("human"), None).unwrap();
    assert_eq!(advanced.open_agents.len(), 1);
    assert_eq!(advanced.pending_agents.len(), 1);
    let first_id = advanced.last_message_id.expect("first handoff");
    let first_subject = store
        .get_message(&first_id)
        .unwrap()
        .unwrap()
        .subject
        .unwrap();
    assert!(
        first_subject.starts_with("[1/1/a1/"),
        "initial label: {first_subject}"
    );
    let first_wake = store.list_wakes(None, true).unwrap().remove(0);
    store
        .set_wake_status(&first_wake.id, WakeStatus::Delivered)
        .unwrap();
    assert_eq!(store.get_task(&task.id).unwrap().unwrap().status, "running");

    let agent = advanced.open_agents[0].clone();
    let refilled = store
        .complete_parallel_member(&task.id, &agent, None)
        .unwrap();
    assert_eq!(refilled.status, "input_required");
    let second_id = refilled.last_message_id.expect("refilled handoff");
    let second_subject = store
        .get_message(&second_id)
        .unwrap()
        .unwrap()
        .subject
        .unwrap();
    assert!(
        second_subject.starts_with("[1/1/a1/"),
        "queued label: {second_subject}"
    );
    assert_ne!(first_subject, second_subject);
    let second_wake = store
        .list_wakes(None, true)
        .unwrap()
        .into_iter()
        .find(|wake| wake.message_id.as_deref() == Some(second_id.as_str()))
        .expect("queued member wake");
    store
        .set_wake_status(&second_wake.id, WakeStatus::Delivered)
        .unwrap();
    assert_eq!(store.get_task(&task.id).unwrap().unwrap().status, "running");
}

#[test]
fn retry_waiting_state_ignores_the_previous_attempt() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let steps: Vec<_> = parallel_impl_steps()
        .into_iter()
        .map(|mut step| {
            step.max_retries = 1;
            step
        })
        .collect();
    let task = store
        .create_task_with_parallel("retry gate", None, &steps, 2, true)
        .unwrap();
    store.advance_task(&task.id, Some("human"), None).unwrap();
    let retried = store
        .retry_task(&task.id, Some("human"), Some("again"))
        .unwrap();
    assert_eq!(retried.status, "input_required");
    let wakes = store.list_wakes(None, false).unwrap();
    let mut stale = Vec::new();
    let mut current = Vec::new();
    for wake in wakes {
        let subject = store
            .get_message(wake.message_id.as_deref().unwrap())
            .unwrap()
            .unwrap()
            .subject
            .unwrap();
        if subject.contains("/a1/") {
            stale.push(wake);
        } else if subject.contains("/a2/") {
            current.push(wake);
        }
    }
    assert_eq!(stale.len(), 2);
    assert_eq!(current.len(), 2);
    store
        .set_wake_status(&stale[0].id, WakeStatus::Cancelled)
        .unwrap();
    assert_eq!(
        store.get_task(&task.id).unwrap().unwrap().status,
        "input_required"
    );
    for wake in &current {
        store
            .set_wake_status(&wake.id, WakeStatus::Delivered)
            .unwrap();
    }
    assert_eq!(store.get_task(&task.id).unwrap().unwrap().status, "running");
}

#[test]
fn parallel_completion_rejects_until_gates_are_resolved() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task_with_parallel("gated parallel", None, &parallel_impl_steps(), 2, true)
        .unwrap();
    let advanced = store.advance_task(&task.id, Some("human"), None).unwrap();
    assert_eq!(advanced.status, "input_required");
    assert_eq!(advanced.open_agents.len(), 2);

    for agent in &advanced.open_agents {
        let err = store
            .complete_parallel_member(&task.id, agent, None)
            .expect_err("gates block completion");
        assert!(
            err.to_string()
                .contains("approval gates are still unresolved"),
            "{err}"
        );
    }
    let advance_err = store
        .advance_task(&task.id, Some("human"), None)
        .expect_err("gates block advancement");
    assert!(
        advance_err
            .to_string()
            .contains("approval gates are still unresolved"),
        "{advance_err}"
    );
    let still = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(still.status, "input_required");
    assert_eq!(store.list_wakes(None, true).unwrap().len(), 2);

    for wake in store.list_wakes(None, true).unwrap() {
        store
            .set_wake_status(&wake.id, WakeStatus::Delivered)
            .unwrap();
    }
    let mut cur = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(cur.status, "running");
    while !cur.open_agents.is_empty() {
        let agent = cur.open_agents[0].clone();
        cur = store
            .complete_parallel_member(&task.id, &agent, None)
            .unwrap();
    }
    assert!(cur.pending_agents.is_empty());
    assert_eq!(cur.status, "running");
    let done = store.advance_task(&task.id, Some("human"), None).unwrap();
    assert_eq!(done.status, "done");
    assert!(store.list_wakes(None, true).unwrap().is_empty());
}

#[test]
fn list_agent_budgets_returns_every_configured_agent() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    assert!(store.list_agent_budgets().unwrap().is_empty());

    store.set_agent_budget("claude", 10.0).unwrap();
    store.set_agent_budget("grok", 5.0).unwrap();

    let budgets = store.list_agent_budgets().unwrap();
    assert_eq!(budgets.len(), 2);
    let ids: Vec<_> = budgets.iter().map(|b| b.agent_id.as_str()).collect();
    assert!(ids.contains(&"claude"));
    assert!(ids.contains(&"grok"));
}

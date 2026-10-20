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
fn parallel_cancel_of_an_earlier_member_fails_the_task() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let task = store
        .create_task_with_parallel("parallel cancel", None, &parallel_impl_steps(), 2, true)
        .unwrap();
    let advanced = store.advance_task(&task.id, Some("human"), None).unwrap();
    let last = advanced.last_message_id.expect("last handoff");
    let earlier = store
        .list_wakes(None, true)
        .unwrap()
        .into_iter()
        .find(|wake| wake.message_id.as_deref() != Some(last.as_str()))
        .expect("earlier member wake");
    store
        .set_wake_status(&earlier.id, WakeStatus::Cancelled)
        .unwrap();
    let failed = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(failed.status, "failed");
    assert!(failed.open_agents.is_empty());
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

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
    }];
    let task = store
        .create_task_with_parallel("ungated task", None, &steps, 1, false)
        .unwrap();
    store.advance_task(&task.id, Some("human"), None).unwrap();
    let wakes = store.list_wakes(Some("claude"), true).unwrap();
    assert_eq!(wakes.len(), 1);
    assert!(!wakes[0].requires_human_gate);
    assert!(
        !store
            .get_task(&task.id)
            .unwrap()
            .unwrap()
            .require_human_approval
    );
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

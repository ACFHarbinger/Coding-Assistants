use super::*;
use std::path::PathBuf;

#[test]
fn task_injection_queues_without_spawning_a_new_harness() {
    let result = inject_harness(&HarnessInjectRequest {
        harness: "grok".into(),
        workspace: PathBuf::from("/tmp/workspace-for-task-queue"),
        session_id: Some("session-1".into()),
        message_id: Some("message-1".into()),
        body: "review this".into(),
        is_task: true,
        is_wake: false,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(result.status, "queued");
    assert_eq!(result.pid, None);
}

#[test]
fn chat_task_without_a_persisted_thread_is_unavailable_not_spawned() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::HubStore::open(dir.path()).unwrap();
    let result = crate::inject_harness_with_store(
        &store,
        &HarnessInjectRequest {
            harness: "chat".into(),
            workspace: PathBuf::from("/tmp/workspace-for-codex-queue"),
            session_id: Some("session-1".into()),
            message_id: Some("message-1".into()),
            body: "review this".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.status, "unavailable");
    assert_eq!(result.pid, None);
    assert!(!result.detail.contains("spawned"));
}

#[test]
fn claude_wake_spawn_stops_before_the_call_when_the_budget_would_be_exceeded() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::HubStore::open(dir.path()).unwrap();
    // Half a unit: one reserved provider call cannot fit, so the
    // gate pauses the agent and stops before any spawn.
    store.set_agent_budget("claude", 0.5).unwrap();
    let result = crate::inject_harness_with_store(
        &store,
        &HarnessInjectRequest {
            harness: "claude".into(),
            workspace: PathBuf::from("/tmp/workspace-claude-budget"),
            body: "wake up".into(),
            is_task: false,
            is_wake: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.status, "stopped");
    assert_eq!(result.pid, None);
    assert!(result.detail.contains("handoff"), "{}", result.detail);
    let status = store.get_budget("claude").unwrap().unwrap();
    assert!(status.paused);
}

#[test]
fn claude_spawn_gate_allows_an_unmetered_agent() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::HubStore::open(dir.path()).unwrap();
    // No budget row for claude: the gate must allow (None) without
    // reserving anything, and without touching the spawn path.
    assert!(one_shot_spawn_gate(&store, HarnessId::Claude)
        .unwrap()
        .is_none());
}

#[test]
fn codex_wake_spawn_stops_before_the_call_when_the_budget_would_be_exceeded() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::HubStore::open(dir.path()).unwrap();
    store.set_agent_budget("codex", 0.5).unwrap();
    let result = crate::inject_harness_with_store(
        &store,
        &HarnessInjectRequest {
            harness: "chat".into(),
            workspace: PathBuf::from("/tmp/workspace-codex-budget"),
            body: "wake up".into(),
            is_task: false,
            is_wake: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.harness, "chat");
    assert_eq!(result.status, "stopped");
    assert_eq!(result.pid, None);
    assert!(result.detail.contains("Codex"), "{}", result.detail);
    assert!(result.detail.contains("handoff"), "{}", result.detail);
    let status = store.get_budget("codex").unwrap().unwrap();
    assert!(status.paused);
}

#[test]
fn codex_spawn_gate_allows_an_unmetered_agent() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::HubStore::open(dir.path()).unwrap();
    assert!(one_shot_spawn_gate(&store, HarnessId::Chat)
        .unwrap()
        .is_none());
}

#[test]
fn codex_task_delivery_is_not_spawn_gated() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::HubStore::open(dir.path()).unwrap();
    store.set_agent_budget("codex", 0.5).unwrap();
    let result = crate::inject_harness_with_store(
        &store,
        &HarnessInjectRequest {
            harness: "chat".into(),
            workspace: PathBuf::from("/tmp/workspace-codex-task-not-gated"),
            session_id: Some("session-1".into()),
            message_id: Some("message-1".into()),
            body: "review this".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.status, "unavailable");
    assert_eq!(result.pid, None);
    let status = store.get_budget("codex").unwrap().unwrap();
    assert!(!status.paused);
}

#[test]
fn one_shot_wakes_stop_before_spawn_when_that_agents_budget_is_stopped() {
    for harness in [
        "grok", "gemini", "opencode", "deepseek", "vibe", "muse", "cursor", "qwen", "kimi",
        "hermes",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::HubStore::open(dir.path()).unwrap();
        let agent_id = if harness == "vibe" {
            "mistral"
        } else {
            harness
        };
        store.set_agent_budget(agent_id, 0.5).unwrap();
        let request = HarnessInjectRequest {
            harness: harness.into(),
            workspace: PathBuf::from(format!("/tmp/workspace-{harness}-budget")),
            body: "wake up".into(),
            is_task: false,
            is_wake: true,
            ..Default::default()
        };
        let stopped = crate::inject_harness_with_store(&store, &request).unwrap();
        assert_eq!(stopped.harness, harness);
        assert_eq!(stopped.status, "stopped");
        assert_eq!(stopped.pid, None);
        assert!(
            stopped.detail.contains(harness) && stopped.detail.contains("handoff"),
            "{}",
            stopped.detail
        );
        assert!(store.get_budget(agent_id).unwrap().unwrap().paused);

        let paused = crate::inject_harness_with_store(&store, &request).unwrap();
        assert_eq!(paused.status, "stopped");
        assert_eq!(paused.pid, None);
        assert!(
            paused.detail.contains(harness) && paused.detail.contains("budget-paused"),
            "{}",
            paused.detail
        );
    }
}

#[test]
fn grok_task_delivery_is_not_spawn_gated() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::HubStore::open(dir.path()).unwrap();
    store.set_agent_budget("grok", 0.5).unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let result = crate::inject_harness_with_store(
        &store,
        &HarnessInjectRequest {
            harness: "grok".into(),
            workspace,
            session_id: Some("session-1".into()),
            message_id: Some("message-1".into()),
            body: "review this".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.status, "unavailable");
    assert_eq!(result.pid, None);
    let status = store.get_budget("grok").unwrap().unwrap();
    assert!(!status.paused);
    assert_eq!(status.spent_units, 0.0);
}

#[test]
fn vibe_spawn_checks_the_mistral_roster_budget() {
    let dir = tempfile::tempdir().unwrap();
    let store = crate::HubStore::open(dir.path()).unwrap();
    store.set_agent_budget("mistral", 0.5).unwrap();
    let stopped = one_shot_spawn_gate(&store, HarnessId::Vibe).unwrap();
    assert!(stopped.is_some(), "Vibe must not bypass the Mistral budget");
    let stopped = stopped.unwrap();
    assert_eq!(stopped.harness, "vibe");
    assert_eq!(stopped.status, "stopped");
    assert!(store.get_budget("mistral").unwrap().unwrap().paused);
    assert!(store.get_budget("vibe").unwrap().is_none());
}

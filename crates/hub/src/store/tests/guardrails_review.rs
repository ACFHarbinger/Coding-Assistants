//! Codex review fixes for P16 (#360): workspace scoping, output
//! watermarks, and parallel-stage input preflight.
use super::super::*;
use crate::guardrails::{GuardrailMode, GuardrailSpec, GuardrailStage};
use std::fs;
use tempfile::tempdir;

fn open() -> (HubStore, String, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path().join("home")).unwrap();
    let workspace = dir.path().join("ws");
    fs::create_dir_all(&workspace).unwrap();
    (store, workspace.to_string_lossy().to_string(), dir)
}

fn step(agent: &str, instruction: &str) -> WorkflowStep {
    WorkflowStep {
        agent: agent.into(),
        role: None,
        instruction: instruction.into(),
        max_retries: 0,
        parallel_group: None,
        peer: None,
        branch: None,
    }
}

fn grouped(agent: &str, instruction: &str) -> WorkflowStep {
    let mut step = step(agent, instruction);
    step.parallel_group = Some("g".into());
    step
}

fn deny(phrases: &[&str]) -> GuardrailSpec {
    GuardrailSpec::DenyPhrases {
        phrases: phrases.iter().map(|p| p.to_string()).collect(),
        case_insensitive: true,
    }
}

#[test]
fn unscoped_task_inherits_only_hub_wide_rules() {
    let (store, ws, _guard) = open();
    store
        .add_guardrail(
            "ws-only",
            Some(&ws),
            GuardrailStage::Input,
            GuardrailMode::Blocking,
            deny(&["rm -rf"]),
        )
        .unwrap();
    assert_eq!(store.list_guardrails(None).unwrap().len(), 1);
    assert!(store
        .effective_guardrails(GuardrailStage::Input, None)
        .unwrap()
        .is_empty());

    let task = store
        .create_task("unscoped", None, &[step("dev", "please rm -rf /")])
        .unwrap();
    assert_eq!(
        store.advance_task(&task.id, None, None).unwrap().status,
        "running",
        "another workspace's rule must not screen an unscoped task"
    );

    store
        .add_guardrail(
            "hub-wide",
            None,
            GuardrailStage::Input,
            GuardrailMode::Blocking,
            deny(&["force-push"]),
        )
        .unwrap();
    let blocked = store
        .create_task("hub", None, &[step("dev", "force-push the branch")])
        .unwrap();
    let err = store
        .advance_task(&blocked.id, None, None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("hub-wide"), "{err}");
}

#[test]
fn queued_parallel_dispatch_does_not_hide_earlier_member_output() {
    let (store, ws, _guard) = open();
    store
        .add_guardrail(
            "no-secret",
            None,
            GuardrailStage::Output,
            GuardrailMode::Blocking,
            deny(&["secret"]),
        )
        .unwrap();
    let task = store
        .create_task_with_parallel(
            "par",
            Some(&ws),
            &[
                grouped("dev_a", "a"),
                grouped("dev_b", "b"),
                grouped("dev_c", "c"),
            ],
            2,
            true,
        )
        .unwrap();
    let started = store.advance_task(&task.id, None, None).unwrap();
    assert_eq!(started.open_agents, vec!["dev_a", "dev_b"]);
    assert_eq!(started.pending_agents, vec!["dev_c"]);

    store
        .send_message(
            "dev_b",
            "human",
            MessageKind::Message,
            "leaked a secret in the log",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();
    store
        .send_message(
            "dev_a",
            "human",
            MessageKind::Message,
            "clean",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();

    let after_a = store
        .complete_parallel_member(&task.id, "dev_a", None)
        .unwrap();
    assert!(after_a.open_agents.iter().any(|a| a == "dev_c"));
    assert_eq!(
        after_a.last_message_id, started.last_message_id,
        "queued dispatch must not move the stage output watermark"
    );

    let err = store
        .complete_parallel_member(&task.id, "dev_b", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("no-secret"), "{err}");
}

#[test]
fn parallel_activation_preflights_input_guardrails_before_any_dispatch() {
    let (store, ws, _guard) = open();
    store
        .add_guardrail(
            "no-destructive",
            Some(&ws),
            GuardrailStage::Input,
            GuardrailMode::Blocking,
            deny(&["rm -rf"]),
        )
        .unwrap();
    let task = store
        .create_task(
            "par",
            Some(&ws),
            &[
                grouped("dev_a", "safe work"),
                grouped("dev_b", "please rm -rf /tmp"),
            ],
        )
        .unwrap();
    let err = store
        .advance_task(&task.id, None, None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("no-destructive"), "{err}");

    let after = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(after.status, "pending");
    assert!(after.last_message_id.is_none());
    assert!(store.poll_messages("dev_a", false).unwrap().is_empty());
    assert!(store.poll_messages("dev_b", false).unwrap().is_empty());
    assert!(store.list_wakes(None, false).unwrap().is_empty());
}

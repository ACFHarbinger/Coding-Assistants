//! Codex review fixes for P16 (#360): workspace scoping, output
//! watermarks, and parallel-stage input preflight.
use super::super::*;
use crate::guardrails::{GuardrailMode, GuardrailSpec, GuardrailStage};
use std::fs;
use tempfile::tempdir;

fn open() -> (HubStore, String, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path().join("home")).unwrap();
    // These tests exercise guardrail/watermark mechanics, not the #365
    // approval-gate wait state — keep wakes ungated.
    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: true,
        })
        .unwrap();
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
        sub_task_id: None,
        handoff_metadata: None,
        context_filter: None,
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
        .create_task_with_parallel(
            "unscoped",
            None,
            &[step("dev", "please rm -rf /")],
            4,
            false,
        )
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
            false,
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
        after_a.output_watermark_id, started.output_watermark_id,
        "refill must not move the internal screening watermark"
    );
    assert_ne!(
        after_a.last_message_id, started.output_watermark_id,
        "result pointer stays a real handoff, not the watermark"
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

#[test]
fn parallel_output_watermark_is_stamped_before_member_dispatches() {
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
            &[grouped("dev_a", "a"), grouped("dev_b", "b")],
            4,
            false,
        )
        .unwrap();
    let started = store.advance_task(&task.id, None, None).unwrap();
    let watermark = store
        .get_message(started.output_watermark_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(watermark.kind, "system");
    assert_eq!(watermark.from_agent, "system");
    let result = store
        .get_message(started.last_message_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(result.kind, "handoff");
    assert_ne!(
        started.last_message_id, started.output_watermark_id,
        "A2A/result readers must not see the internal watermark"
    );

    let handoff_a = store.poll_messages("dev_a", false).unwrap();
    let handoff_b = store.poll_messages("dev_b", false).unwrap();
    assert_eq!(handoff_a.len(), 1);
    assert_eq!(handoff_b.len(), 1);

    let origin = chrono::DateTime::parse_from_rfc3339(&watermark.created_at)
        .unwrap()
        .with_timezone(&chrono::Utc);
    let t_a = (origin + chrono::Duration::milliseconds(1)).to_rfc3339();
    let t_reply = (origin + chrono::Duration::milliseconds(2)).to_rfc3339();
    let t_b = (origin + chrono::Duration::milliseconds(3)).to_rfc3339();
    store
        .set_message_created_at(&handoff_a[0].id, &t_a)
        .unwrap();
    store
        .set_message_created_at(&handoff_b[0].id, &t_b)
        .unwrap();

    let reply = store
        .send_message(
            "dev_a",
            "human",
            MessageKind::Message,
            "leaked a secret while B was still dispatching",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();
    store.set_message_created_at(&reply.id, &t_reply).unwrap();

    let err = store
        .complete_parallel_member(&task.id, "dev_a", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("no-secret"), "{err}");
}

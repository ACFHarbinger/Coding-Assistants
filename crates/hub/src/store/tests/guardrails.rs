//! P16 (#360): guardrail rules screen dispatches and replies.
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

fn deny(phrases: &[&str]) -> GuardrailSpec {
    GuardrailSpec::DenyPhrases {
        phrases: phrases.iter().map(|p| p.to_string()).collect(),
        case_insensitive: true,
    }
}

fn audit_ops(store: &HubStore) -> Vec<String> {
    store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .map(|event| event.operation)
        .collect()
}

#[test]
fn rules_round_trip_scope_and_toggle() {
    let (store, ws, _guard) = open();
    let global = store
        .add_guardrail(
            "no-force",
            None,
            GuardrailStage::Input,
            GuardrailMode::Blocking,
            deny(&["force-push"]),
        )
        .unwrap();
    let scoped = store
        .add_guardrail(
            "short",
            Some(&ws),
            GuardrailStage::Output,
            GuardrailMode::Parallel,
            GuardrailSpec::MaxChars { max: 10 },
        )
        .unwrap();
    assert!(global.workspace.is_none());
    assert_eq!(scoped.workspace.as_deref(), Some(ws.as_str()));
    assert_eq!(store.list_guardrails(None).unwrap().len(), 2);
    assert_eq!(store.list_guardrails(Some("/elsewhere")).unwrap().len(), 1);
    assert_eq!(
        store
            .effective_guardrails(GuardrailStage::Output, Some(&ws))
            .unwrap()
            .len(),
        1
    );
    assert!(store
        .effective_guardrails(GuardrailStage::Output, Some("/elsewhere"))
        .unwrap()
        .is_empty());

    store.set_guardrail_enabled(&scoped.id, false).unwrap();
    assert!(store
        .effective_guardrails(GuardrailStage::Output, Some(&ws))
        .unwrap()
        .is_empty());
    store.remove_guardrail(&global.id).unwrap();
    assert!(matches!(
        store.remove_guardrail(&global.id),
        Err(HubError::NotFound(_))
    ));

    assert!(store
        .add_guardrail(
            "bad",
            None,
            GuardrailStage::Input,
            GuardrailMode::Blocking,
            GuardrailSpec::MaxChars { max: 0 }
        )
        .is_err());
    assert!(store
        .add_guardrail(
            "rel",
            Some("relative/ws"),
            GuardrailStage::Input,
            GuardrailMode::Blocking,
            deny(&["x"])
        )
        .is_err());
}

#[test]
fn input_guardrail_blocks_task_dispatch_without_sending_or_advancing() {
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
            "risky",
            Some(&ws),
            &[step("dev", "please rm -rf the build dir")],
        )
        .unwrap();
    let err = store
        .advance_task(&task.id, None, None)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("input guardrail 'no-destructive' tripped"),
        "{err}"
    );

    let after = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(after.status, "pending", "no attempt burned");
    assert!(after.last_message_id.is_none());
    assert!(store.poll_messages("dev", false).unwrap().is_empty());
    assert!(store.list_wakes(None, false).unwrap().is_empty());
    let ops = audit_ops(&store);
    assert_eq!(
        ops.iter().filter(|op| *op == "guardrail.tripped").count(),
        1
    );

    // A hub-wide rule elsewhere does not reach a different workspace.
    let other = store
        .create_task(
            "fine",
            Some("/other/ws"),
            &[step("dev", "please rm -rf the build dir")],
        )
        .unwrap();
    assert_eq!(
        store.advance_task(&other.id, None, None).unwrap().status,
        "running"
    );
}

#[test]
fn output_guardrail_blocks_stage_advance_until_the_reply_is_acceptable() {
    let (store, ws, _guard) = open();
    store
        .add_guardrail(
            "must-report",
            None,
            GuardrailStage::Output,
            GuardrailMode::Blocking,
            GuardrailSpec::RequirePhrases {
                phrases: vec!["tests pass".into()],
                case_insensitive: true,
            },
        )
        .unwrap();
    let task = store
        .create_task(
            "flow",
            Some(&ws),
            &[step("dev", "implement"), step("reviewer", "review")],
        )
        .unwrap();
    store.advance_task(&task.id, None, None).unwrap();
    store
        .send_message(
            "dev",
            "human",
            MessageKind::Message,
            "shipped it, didn't run anything",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();
    let err = store
        .advance_task(&task.id, Some("dev"), None)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("output guardrail 'must-report' tripped"),
        "{err}"
    );
    let stuck = store.get_task(&task.id).unwrap().unwrap();
    assert_eq!(stuck.step_index, 0);
    assert!(store.poll_messages("reviewer", false).unwrap().is_empty());

    // The hand-back note is screened too, and an acceptable one passes.
    let err = store
        .advance_task(&task.id, Some("dev"), Some("done"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("must-report"), "{err}");
    store
        .send_message(
            "dev",
            "human",
            MessageKind::Message,
            "Tests pass: 12/12",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();
    // The earlier non-compliant reply is still on the task; it still trips.
    assert!(store.advance_task(&task.id, Some("dev"), None).is_err());
    store
        .set_guardrail_enabled(&store.list_guardrails(None).unwrap()[0].id, false)
        .unwrap();
    assert_eq!(
        store
            .advance_task(&task.id, Some("dev"), None)
            .unwrap()
            .step_index,
        1
    );
}

#[test]
fn output_guardrail_screens_parallel_members_on_completion() {
    let (store, ws, _guard) = open();
    store
        .add_guardrail(
            "no-secrets",
            None,
            GuardrailStage::Output,
            GuardrailMode::Parallel,
            GuardrailSpec::NoSecrets,
        )
        .unwrap();
    let mut a = step("dev_a", "a");
    a.parallel_group = Some("g".into());
    let mut b = step("dev_b", "b");
    b.parallel_group = Some("g".into());
    let task = store.create_task("par", Some(&ws), &[a, b]).unwrap();
    store.advance_task(&task.id, None, None).unwrap();
    store
        .send_message(
            "dev_a",
            "human",
            MessageKind::Message,
            "token is ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();
    let err = store
        .complete_parallel_member(&task.id, "dev_a", None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("credential-shaped"), "{err}");
    assert!(
        !err.contains("abcdefghij"),
        "secret must never be echoed: {err}"
    );
    assert_eq!(
        store
            .complete_parallel_member(&task.id, "dev_b", Some("clean"))
            .unwrap()
            .open_agents,
        vec!["dev_a".to_string()]
    );
}

fn schedule(
    store: &HubStore,
    ws: &str,
    instruction: &str,
    repeat: Option<i64>,
    max_attempts: u32,
) -> ScheduledTask {
    store
        .create_scheduled_task(
            "g",
            instruction,
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            ws,
            "2020-01-01T00:00:00Z",
            repeat,
            3600,
            vec![GateCriterion::Command {
                program: "true".into(),
                args: vec![],
                check: GateCheck::ExitCodeZero,
            }],
            GateCombinator::All,
            BudgetPolicy::default(),
            max_attempts,
            None,
            "owner",
        )
        .unwrap()
}

#[test]
fn schedule_input_trip_fails_the_schedule_without_burning_an_attempt() {
    let (store, ws, _guard) = open();
    store
        .add_guardrail(
            "no-prod",
            None,
            GuardrailStage::Input,
            GuardrailMode::Blocking,
            deny(&["deploy to prod"]),
        )
        .unwrap();
    let task = schedule(&store, &ws, "deploy to prod now", None, 3);
    let summary = store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    assert_eq!(summary.outcomes[0].action, "failed");
    assert!(summary.outcomes[0]
        .detail
        .contains("input guardrail 'no-prod' tripped"));
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Failed);
    assert_eq!(fetched.attempts, 0, "claim never happened");
    assert!(store.poll_messages("cursor", false).unwrap().is_empty());
    assert!(audit_ops(&store).iter().any(|op| op == "guardrail.tripped"));
    assert!(!audit_ops(&store).iter().any(|op| op == "schedule.fired"));
}

#[test]
fn schedule_output_trip_reprompts_without_evaluating_gates_then_fails_on_final_attempt() {
    let (store, ws, _guard) = open();
    store
        .add_guardrail(
            "terse",
            None,
            GuardrailStage::Output,
            GuardrailMode::Blocking,
            GuardrailSpec::MaxChars { max: 8 },
        )
        .unwrap();
    let task = schedule(&store, &ws, "work", Some(1), 2);
    store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    store
        .send_message(
            "cursor",
            "system",
            MessageKind::Message,
            "a very long reply indeed",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();
    let second = store.tick_scheduled_tasks("2020-01-01T00:00:05Z").unwrap();
    assert_eq!(
        second.outcomes[0].action, "dispatched",
        "{:?}",
        second.outcomes[0]
    );
    assert!(second.outcomes[0]
        .detail
        .contains("output guardrail 'terse' tripped"));
    let ops = audit_ops(&store);
    assert!(ops.iter().any(|op| op == "guardrail.tripped"));
    assert!(
        !ops.iter().any(|op| op == "schedule.gate_checked"),
        "gate must not run on a tripped reply"
    );

    store
        .send_message(
            "cursor",
            "system",
            MessageKind::Message,
            "still far too long a reply",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();
    let third = store.tick_scheduled_tasks("2020-01-01T00:00:10Z").unwrap();
    assert_eq!(third.outcomes[0].action, "failed");
    let fetched = store.get_scheduled_task(&task.id).unwrap().unwrap();
    assert_eq!(fetched.attempts, 2);
    assert_eq!(fetched.status, ScheduleStatus::Failed);
}

#[test]
fn schedule_output_passes_a_clean_reply_through_to_the_gate() {
    let (store, ws, _guard) = open();
    store
        .add_guardrail(
            "terse",
            None,
            GuardrailStage::Output,
            GuardrailMode::Blocking,
            GuardrailSpec::MaxChars { max: 8 },
        )
        .unwrap();
    let task = schedule(&store, &ws, "work", Some(1), 2);
    store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    store
        .send_message(
            "cursor",
            "system",
            MessageKind::Message,
            "done",
            None,
            Some(&ws),
            Some(&task.id),
        )
        .unwrap();
    let second = store.tick_scheduled_tasks("2020-01-01T00:00:05Z").unwrap();
    assert_eq!(second.outcomes[0].action, "completed");
    assert!(audit_ops(&store)
        .iter()
        .any(|op| op == "schedule.gate_checked"));
    assert!(!audit_ops(&store).iter().any(|op| op == "guardrail.tripped"));
}

#[test]
fn tripped_rows_count_as_security_class_audit_events() {
    assert!(crate::store::scorecard::helpers::is_security_class_op(
        "guardrail.tripped"
    ));
}

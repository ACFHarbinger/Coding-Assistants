//! P17 / #361: structured handoff metadata and opt-in context filtering.

use super::super::*;
use tempfile::tempdir;

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

fn open() -> (HubStore, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    (store, dir)
}

#[test]
fn unfiltered_handoff_keeps_the_prior_note_and_skips_audit() {
    let (store, _dir) = open();
    let task = store
        .create_task("plain", Some("/tmp/p17"), &[step("grok", "INSTR-PLAIN")])
        .unwrap();
    store
        .advance_task(&task.id, Some("human"), Some("plan ready"))
        .unwrap();
    let inbox = store.poll_messages("grok", true).unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].to_agent, "grok");
    assert_eq!(inbox[0].body, "INSTR-PLAIN\n\n---\nPrior note: plan ready");
    let ops = audit_ops(&store);
    assert!(!ops.iter().any(|op| op.starts_with("handoff.")));
}

#[test]
fn filter_strips_tool_noise_caps_history_and_audits_metadata_without_the_body() {
    let (store, _dir) = open();
    let mut coded = step("claude", "INSTR-KEEP");
    coded.handoff_metadata = Some(serde_json::json!({"reason": "REASON-ONLY", "priority": 2}));
    coded.context_filter = Some("strip_tools,cap:2".into());
    let task = store
        .create_task("filtered", Some("/tmp/p17"), &[coded])
        .unwrap();
    store
        .send_message(
            "system",
            "system",
            MessageKind::System,
            "WATERMARK-NOISE",
            None,
            Some("/tmp/p17"),
            Some(&task.id),
        )
        .unwrap();
    store
        .send_message(
            "grok",
            "claude",
            MessageKind::Message,
            "keep-a\nTool: read src/lib.rs\n{\"type\":\"tool_use\",\"name\":\"bash\"}\nkeep-b",
            None,
            Some("/tmp/p17"),
            Some(&task.id),
        )
        .unwrap();
    store
        .advance_task(&task.id, Some("human"), Some("note-tail"))
        .unwrap();
    let inbox = store.poll_messages("claude", true).unwrap();
    let handoff = inbox
        .iter()
        .find(|message| message.kind == "handoff")
        .unwrap();
    assert_eq!(handoff.to_agent, "claude");
    assert!(handoff
        .body
        .starts_with("INSTR-KEEP\n\n---\nPrior context:\n"));
    assert!(handoff.body.contains("keep-b"));
    assert!(handoff.body.contains("note: note-tail"));
    assert!(!handoff.body.contains("Tool:"));
    assert!(!handoff.body.contains("tool_use"));
    assert!(!handoff.body.contains("WATERMARK-NOISE"));
    assert!(!handoff.body.contains("REASON-ONLY"));
    let events = store.list_audit_events(false).unwrap();
    let meta = events
        .iter()
        .find(|event| event.operation == "handoff.metadata")
        .unwrap();
    assert!(meta.process_json.contains("REASON-ONLY"));
    assert!(!meta.process_json.contains("INSTR-KEEP"));
    assert!(!meta.process_json.contains("keep-b"));
    let filtered = events
        .iter()
        .find(|event| event.operation == "handoff.context_filtered")
        .unwrap();
    assert!(filtered.process_json.contains("strip_tools,cap:2"));
    assert!(!filtered.process_json.contains("keep-b"));
    assert!(!filtered.process_json.contains("WATERMARK-NOISE"));
}

#[test]
fn bad_filter_and_metadata_are_rejected_before_a_task_exists() {
    let (store, _dir) = open();
    let mut bad_filter = step("grok", "do it");
    bad_filter.context_filter = Some("drop_all".into());
    assert!(store
        .create_task("nope", Some("/tmp/p17"), &[bad_filter])
        .is_err());
    let mut bad_meta = step("grok", "do it");
    bad_meta.handoff_metadata = Some(serde_json::json!({"agent": "root"}));
    assert!(store
        .create_task("nope", Some("/tmp/p17"), &[bad_meta])
        .is_err());
    assert!(store.list_tasks(None).unwrap().is_empty());
}

#[test]
fn schedule_filter_is_opt_in_and_metadata_does_not_change_the_target() {
    let dir = tempdir().unwrap();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let store = HubStore::open(dir.path()).unwrap();
    let plain = store
        .create_scheduled_task(
            "plain",
            "INSTR-SCHED",
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            &workspace,
            "2020-01-01T00:00:00Z",
            None,
            3600,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            1,
            None,
            "human",
        )
        .unwrap();
    let raw = format!("{}\n\n---\nalpha\nbeta", plain.instruction);
    assert_eq!(store.schedule_handoff_body(&plain, &raw).unwrap(), raw);

    let filtered = store
        .set_schedule_handoff(
            &plain.id,
            Some(serde_json::json!({"reason": "REASON-SCHED", "priority": "high"})),
            Some("cap:1"),
        )
        .unwrap();
    assert_eq!(filtered.context_filter.as_deref(), Some("cap:1"));
    let seeded = store.schedule_handoff_body(&filtered, &raw).unwrap();
    assert!(seeded.starts_with("INSTR-SCHED\n\n---\nPrior context:\n"));
    assert!(seeded.ends_with("beta"));
    assert!(!seeded.contains("alpha"));
    assert!(!seeded.contains("REASON-SCHED"));

    store.tick_scheduled_tasks("2020-01-01T00:00:01Z").unwrap();
    let sent = store
        .list_messages(None, None)
        .unwrap()
        .into_iter()
        .find(|message| message.kind == "handoff")
        .unwrap();
    assert_eq!(sent.to_agent, "cursor");
    assert_eq!(sent.body, "INSTR-SCHED");
    let meta = store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .find(|event| event.operation == "handoff.metadata")
        .unwrap();
    assert!(meta.process_json.contains("REASON-SCHED"));
    assert!(meta.process_json.contains("cursor"));
    assert!(!meta.process_json.contains("INSTR-SCHED"));
}

#[test]
fn first_line_tool_records_are_stripped_before_message_attribution() {
    let (store, _dir) = open();
    let mut coded = step("claude", "INSTR-KEEP");
    coded.context_filter = Some("strip_tools".into());
    let task = store
        .create_task("first-line-tools", Some("/tmp/p17"), &[coded])
        .unwrap();
    store
        .send_message(
            "grok",
            "claude",
            MessageKind::Message,
            "Tool: read src/lib.rs",
            None,
            Some("/tmp/p17"),
            Some(&task.id),
        )
        .unwrap();
    store
        .send_message(
            "gemini",
            "claude",
            MessageKind::Message,
            r#"{"type":"RUN_COMMAND"}"#,
            None,
            Some("/tmp/p17"),
            Some(&task.id),
        )
        .unwrap();
    store
        .send_message(
            "cursor",
            "claude",
            MessageKind::Message,
            "keep-visible",
            None,
            Some("/tmp/p17"),
            Some(&task.id),
        )
        .unwrap();
    store
        .advance_task(&task.id, Some("human"), Some("note-tail"))
        .unwrap();
    let handoff = store
        .poll_messages("claude", true)
        .unwrap()
        .into_iter()
        .find(|message| message.kind == "handoff")
        .unwrap();
    assert!(handoff.body.contains("keep-visible"));
    assert!(handoff.body.contains("note: note-tail"));
    assert!(handoff
        .body
        .contains("cursor -> claude (message): keep-visible"));
    assert!(!handoff.body.contains("Tool:"));
    assert!(!handoff.body.contains("RUN_COMMAND"));
}

#[test]
fn schedule_create_commits_handoff_options_with_the_row() {
    let dir = tempdir().unwrap();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    let workspace = workspace.to_string_lossy().into_owned();
    let store = HubStore::open(dir.path()).unwrap();
    let err = store.create_scheduled_task_with_handoff(
        "bad",
        "INSTR",
        ScheduleTarget::Agent {
            agent_id: "cursor".into(),
        },
        &workspace,
        "2020-01-01T00:00:00Z",
        None,
        3600,
        vec![],
        GateCombinator::All,
        BudgetPolicy::default(),
        1,
        None,
        "human",
        Some(serde_json::json!({"agent": "root"})),
        Some("drop_all"),
    );
    assert!(err.is_err());
    assert!(store.list_scheduled_tasks().unwrap().is_empty());

    let created = store
        .create_scheduled_task_with_handoff(
            "due",
            "INSTR-SCHED",
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            &workspace,
            "2020-01-01T00:00:00Z",
            None,
            3600,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            1,
            None,
            "human",
            Some(serde_json::json!({"reason": "REASON-SCHED", "priority": 1})),
            Some("strip_tools"),
        )
        .unwrap();
    let stored = store.get_scheduled_task(&created.id).unwrap().unwrap();
    assert_eq!(stored.context_filter.as_deref(), Some("strip_tools"));
    assert_eq!(stored.handoff_metadata.unwrap()["reason"], "REASON-SCHED");
}

fn audit_ops(store: &HubStore) -> Vec<String> {
    store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .map(|event| event.operation)
        .collect()
}

fn allow_wakes(store: &HubStore) {
    store
        .set_wake_policy(&WakePolicy {
            default_requires_human_gate: false,
            allow_auto_wake: true,
        })
        .unwrap();
}

#[test]
fn sequential_filtered_handoffs_grow_linearly_and_keep_original_replies_once() {
    let (store, _dir) = open();
    allow_wakes(&store);
    let steps: Vec<_> = (0..16)
        .map(|_| {
            let mut s = step("grok", "instruction");
            s.context_filter = Some("strip_tools".into());
            s
        })
        .collect();
    let task = store
        .create_task_with_parallel("linear", None, &steps, 4, false)
        .unwrap();
    for index in 0..16 {
        let running = store.advance_task(&task.id, None, None).unwrap();
        let message = store
            .get_message(running.last_message_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(
            message.body.matches("Prior context:").count(),
            usize::from(index > 0)
        );
        assert_eq!(message.body.matches("instruction").count(), index + 1);
        assert!(message.body.len() < 200 * (index + 1));
        for previous in 0..index {
            assert_eq!(
                message
                    .body
                    .matches(&format!("reply-{previous:02}"))
                    .count(),
                1
            );
        }
        store
            .send_message(
                "grok",
                "human",
                MessageKind::Message,
                &format!("reply-{index:02}"),
                None,
                None,
                Some(&task.id),
            )
            .unwrap();
    }
}

#[test]
fn parallel_filtered_dispatch_uses_the_preflight_snapshot() {
    use crate::guardrails::{GuardrailMode, GuardrailSpec, GuardrailStage};
    let (store, _dir) = open();
    allow_wakes(&store);
    store
        .add_guardrail(
            "short input",
            None,
            GuardrailStage::Input,
            GuardrailMode::Blocking,
            GuardrailSpec::MaxChars { max: 50 },
        )
        .unwrap();
    let steps: Vec<_> = ["grok", "claude"]
        .into_iter()
        .map(|agent| {
            let mut s = step(agent, "instruction");
            s.context_filter = Some("strip_tools".into());
            s.parallel_group = Some("both".into());
            s
        })
        .collect();
    let task = store
        .create_task("parallel snapshot", None, &steps)
        .unwrap();
    let running = store.advance_task(&task.id, None, None).unwrap();
    assert_eq!(running.open_agents.len(), 2);
    for agent in ["grok", "claude"] {
        let messages = store.poll_messages(agent, true).unwrap();
        let handoff = messages.iter().find(|m| m.kind == "handoff").unwrap();
        assert_eq!(handoff.body, "instruction");
        assert_eq!(store.list_wakes(Some(agent), true).unwrap().len(), 1);
    }
}

#[test]
fn filtered_remote_result_is_retained_once_without_its_seeded_input() {
    use super::peer_dispatch::{stub_peer_once, PEER_TOKEN_LOCK};
    let _lock = PEER_TOKEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("CA_TCP_AUTH_TOKEN", "context-test-token");
    let (store, _dir) = open();
    allow_wakes(&store);
    let mut remote = step("dev", "remote instruction");
    remote.peer = Some(stub_peer_once(
        "context-test-token",
        "genuine-remote-result",
    ));
    remote.context_filter = Some("strip_tools".into());
    let mut local = step("grok", "local instruction");
    local.context_filter = Some("strip_tools".into());
    let task = store
        .create_task_with_parallel(
            "remote context",
            None,
            &[remote, local.clone(), local],
            4,
            false,
        )
        .unwrap();
    store
        .send_message(
            "human",
            "dev",
            MessageKind::Message,
            "original-history",
            None,
            None,
            Some(&task.id),
        )
        .unwrap();
    store.advance_task(&task.id, None, None).unwrap();
    std::env::remove_var("CA_TCP_AUTH_TOKEN");
    for _ in 0..2 {
        let running = store.advance_task(&task.id, None, None).unwrap();
        let message = store
            .get_message(running.last_message_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(message.body.matches("genuine-remote-result").count(), 1);
        assert_eq!(message.body.matches("original-history").count(), 1);
        assert_eq!(message.body.matches("Prior context:").count(), 1);
    }
}

#[test]
fn repeated_schedule_attempts_do_not_reseed_nested_history() {
    let (store, dir) = open();
    let workspace = dir.path().to_string_lossy();
    let schedule = store
        .create_scheduled_task_with_handoff(
            "linear schedule",
            "instruction",
            ScheduleTarget::Agent {
                agent_id: "cursor".into(),
            },
            &workspace,
            "2020-01-01T00:00:00Z",
            Some(60),
            1,
            vec![],
            GateCombinator::All,
            BudgetPolicy::default(),
            32,
            None,
            "human",
            None,
            Some("strip_tools"),
        )
        .unwrap();
    for minute in 0..16 {
        store
            .tick_scheduled_tasks(&format!("2020-01-01T00:{minute:02}:01Z"))
            .unwrap();
    }
    let messages: Vec<_> = store
        .list_messages(None, None)
        .unwrap()
        .into_iter()
        .filter(|m| m.task_id.as_deref() == Some(&schedule.id) && m.kind == "handoff")
        .collect();
    assert_eq!(messages.len(), 16);
    for message in messages {
        assert!(message.body.matches("Prior context:").count() <= 1);
        assert!(message.body.len() < 200 * 16);
    }
}

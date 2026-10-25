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

fn audit_ops(store: &HubStore) -> Vec<String> {
    store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .map(|event| event.operation)
        .collect()
}

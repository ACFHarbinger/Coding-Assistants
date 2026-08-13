use super::super::*;
use tempfile::tempdir;

#[test]
fn team_broadcast_uses_enrolled_roster_and_includes_human() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    for agent in ["claude", "chat", "gemini", "grok"] {
        store.set_team_member(agent, true).unwrap();
    }

    store.upsert_agent("process:1", "Codex · PID 1").unwrap();
    store.upsert_agent("a2a-peer", "a2a-peer").unwrap();

    let team = store
        .send_message_to_team(
            "grok",
            MessageKind::Message,
            "M6 roster check",
            Some("channel:general"),
            None,
            None,
        )
        .unwrap();
    let recipients: Vec<&str> = team.iter().map(|m| m.to_agent.as_str()).collect();
    assert!(recipients.contains(&"human"), "{recipients:?}");
    assert!(recipients.contains(&"claude"), "{recipients:?}");
    assert!(recipients.contains(&"chat"), "{recipients:?}");
    assert!(recipients.contains(&"gemini"), "{recipients:?}");
    assert!(!recipients.contains(&"grok"), "{recipients:?}");
    assert!(!recipients.contains(&"process:1"), "{recipients:?}");
    assert!(!recipients.contains(&"a2a-peer"), "{recipients:?}");
    assert!(!recipients.contains(&"ollama"), "{recipients:?}");
    assert!(!recipients.contains(&"system"), "{recipients:?}");
    assert!(team
        .iter()
        .all(|m| m.subject.as_deref() == Some("channel:general")));

    store.set_team_member("ollama", true).unwrap();
    store.set_team_member("claude", false).unwrap();
    let updated = store
        .send_message_to_team(
            "grok",
            MessageKind::Message,
            "roster after enroll change",
            None,
            None,
            None,
        )
        .unwrap();
    let recipients: Vec<&str> = updated.iter().map(|m| m.to_agent.as_str()).collect();
    assert!(recipients.contains(&"ollama"), "{recipients:?}");
    assert!(!recipients.contains(&"claude"), "{recipients:?}");
    assert!(recipients.contains(&"human"), "{recipients:?}");

    store.set_team_member("claude", true).unwrap();
    store.set_team_member("ollama", false).unwrap();
    let wakes = store
        .request_team_wakes("human", Some("Slack #general"), Some("msg-team-1"), false)
        .unwrap();
    let woke: Vec<&str> = wakes.iter().map(|w| w.target_agent.as_str()).collect();
    assert!(woke.contains(&"claude"), "{woke:?}");
    assert!(woke.contains(&"chat"), "{woke:?}");
    assert!(woke.contains(&"gemini"), "{woke:?}");
    assert!(woke.contains(&"grok"), "{woke:?}");
    assert!(!woke.contains(&"human"), "{woke:?}");
    assert!(!woke.contains(&"ollama"), "{woke:?}");
    assert!(!woke.contains(&"process:1"), "{woke:?}");
    assert_eq!(woke.len(), 4, "{woke:?}");
}

#[test]
fn ca106_edit_and_delete_a_team_broadcast_updates_every_copy() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    store.set_team_member("claude", true).unwrap();
    store.set_team_member("grok", true).unwrap();
    store.set_team_member("chat", true).unwrap();

    let subject = "channel:general:11111111-1111-1111-1111-111111111111";
    let posted = store
        .send_message_to_team(
            "human",
            MessageKind::Message,
            "hi",
            Some(subject),
            None,
            None,
        )
        .unwrap();
    assert!(posted.len() >= 3, "{posted:?}");

    // Editing any one copy of the broadcast must update every sibling
    // row sharing the subject, not just the row that happened to render.
    let edited = store
        .update_broadcast(&posted[0].id, "hi (edited)")
        .unwrap();
    assert_eq!(edited.len(), posted.len());
    assert!(edited.iter().all(|m| m.body == "hi (edited)"));
    for original in &posted {
        let refreshed = store.get_message(&original.id).unwrap().unwrap();
        assert_eq!(refreshed.body, "hi (edited)");
    }

    let deleted_count = store.delete_broadcast(&posted[1].id).unwrap();
    assert_eq!(deleted_count, posted.len());
    for original in &posted {
        let refreshed = store.get_message(&original.id).unwrap().unwrap();
        assert_eq!(refreshed.status, "cancelled");
    }
}

#[test]
fn ca106_edit_and_delete_a_legacy_broadcast_groups_by_sender_body_and_second() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    // Legacy posts share the exact `channel:<name>` subject with no
    // per-broadcast uuid suffix; grouping falls back to
    // (from_agent, body, subject, created-at-to-the-second).
    let a = store
        .send_message(
            "grok",
            "claude",
            MessageKind::Message,
            "legacy note",
            Some("channel:general"),
            None,
            None,
        )
        .unwrap();
    let b = store
        .send_message(
            "grok",
            "chat",
            MessageKind::Message,
            "legacy note",
            Some("channel:general"),
            None,
            None,
        )
        .unwrap();
    // A distinct send (different body) must not be swept into the group.
    let unrelated = store
        .send_message(
            "grok",
            "gemini",
            MessageKind::Message,
            "unrelated note",
            Some("channel:general"),
            None,
            None,
        )
        .unwrap();

    let deleted_count = store.delete_broadcast(&a.id).unwrap();
    assert_eq!(deleted_count, 2);
    assert_eq!(
        store.get_message(&a.id).unwrap().unwrap().status,
        "cancelled"
    );
    assert_eq!(
        store.get_message(&b.id).unwrap().unwrap().status,
        "cancelled"
    );
    assert_eq!(
        store.get_message(&unrelated.id).unwrap().unwrap().status,
        "pending"
    );
}

#[test]
fn m3_export_markdown_git_commits_inside_a_work_tree() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    // No repo yet: commit is skipped, not an error.
    let outcome = store.export_markdown_git(None, None).unwrap();
    assert!(!outcome.committed);
    assert!(outcome.path.exists());

    let md_dir = dir.path().join("markdown");
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(&md_dir)
            .args(args)
            .output()
            .expect("git available for test");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "hub-test@example.com"]);
    git(&["config", "user.name", "Hub Test"]);

    store
        .write_memory(
            MemoryTier::Episodic,
            MemoryScope::Global,
            Some("claude"),
            None,
            Some("git export"),
            "Verify markdown export auto-commits inside a work tree.",
            &[],
        )
        .unwrap();

    let outcome = store
        .export_markdown_git(None, Some("chore(hub): test export"))
        .unwrap();
    assert!(
        outcome.committed,
        "expected a commit, got: {}",
        outcome.detail
    );

    let log = Command::new("git")
        .arg("-C")
        .arg(&md_dir)
        .args(["log", "-1", "--pretty=%s"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&log.stdout).trim(),
        "chore(hub): test export"
    );

    // The export body always rewrites its "Generated:" timestamp, so a
    // second call still has a diff and commits again rather than being a
    // no-op; that's the git-tracked-history behavior M3 asks for.
    let second = store
        .export_markdown_git(None, Some("chore(hub): test export 2"))
        .unwrap();
    assert!(
        second.committed,
        "expected a second commit, got: {}",
        second.detail
    );
}

#[test]
fn c5_sequential_task_advance_plan_code_review() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let steps = vec![
        WorkflowStep {
            agent: "grok".into(),
            role: Some("Planner".into()),
            instruction: "Plan the dual-mode pathing fix.".into(),
            max_retries: 0,
            parallel_group: None,
        },
        WorkflowStep {
            agent: "claude".into(),
            role: Some("Developer".into()),
            instruction: "Implement the plan.".into(),
            max_retries: 0,
            parallel_group: None,
        },
        WorkflowStep {
            agent: "gemini".into(),
            role: Some("Reviewer".into()),
            instruction: "Review the implementation.".into(),
            max_retries: 0,
            parallel_group: None,
        },
    ];
    let task = store
        .create_task("Slice pathing", Some("/tmp/pmf"), &steps)
        .unwrap();
    assert_eq!(task.status, "pending");
    assert_eq!(task.step_index, 0);

    let t1 = store.advance_task(&task.id, None, None).unwrap();
    assert_eq!(t1.status, "running");
    assert_eq!(t1.step_index, 0);
    assert!(t1.last_message_id.is_some());
    let inbox = store.poll_messages("grok", true).unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].kind, "handoff");

    let t2 = store
        .advance_task(&task.id, Some("grok"), Some("plan ready"))
        .unwrap();
    assert_eq!(t2.step_index, 1);
    let for_claude = store.poll_messages("claude", true).unwrap();
    assert!(!for_claude.is_empty());
    assert!(for_claude[0].body.contains("Implement"));

    let t3 = store.advance_task(&task.id, Some("claude"), None).unwrap();
    assert_eq!(t3.step_index, 2);

    let done = store.advance_task(&task.id, Some("gemini"), None).unwrap();
    assert_eq!(done.status, "done");

    let listed = store.list_tasks(Some(TaskStatus::Done)).unwrap();
    assert_eq!(listed.len(), 1);
}

#[test]
fn c5_bounded_parallel_and_retry() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let steps = vec![
        WorkflowStep {
            agent: "planner".into(),
            role: None,
            instruction: "Plan".into(),
            max_retries: 0,
            parallel_group: None,
        },
        WorkflowStep {
            agent: "dev_a".into(),
            role: None,
            instruction: "Code path A".into(),
            max_retries: 1,
            parallel_group: Some("impl".into()),
        },
        WorkflowStep {
            agent: "dev_b".into(),
            role: None,
            instruction: "Code path B".into(),
            max_retries: 1,
            parallel_group: Some("impl".into()),
        },
        WorkflowStep {
            agent: "dev_c".into(),
            role: None,
            instruction: "Code path C".into(),
            max_retries: 1,
            parallel_group: Some("impl".into()),
        },
        WorkflowStep {
            agent: "reviewer".into(),
            role: None,
            instruction: "Review all".into(),
            max_retries: 0,
            parallel_group: None,
        },
    ];
    // max_parallel=2 → wake two of three implementers first
    let task = store
        .create_task_with_parallel("parallel slice", None, &steps, 2, true)
        .unwrap();
    let stages = HubStore::workflow_stages(&task.steps);
    assert_eq!(stages.len(), 3); // plan | parallel impl | review

    let t1 = store.advance_task(&task.id, None, None).unwrap();
    assert_eq!(t1.step_index, 0); // sequential plan
    assert!(t1.open_agents.is_empty());

    let t2 = store.advance_task(&task.id, Some("planner"), None).unwrap();
    assert_eq!(t2.step_index, 1);
    assert_eq!(t2.open_agents.len(), 2);
    assert_eq!(t2.pending_agents.len(), 1);

    // Cannot advance while parallel open
    assert!(store.advance_task(&task.id, None, None).is_err());

    let a = t2.open_agents[0].clone();
    let b = t2.open_agents[1].clone();
    let mid = store.complete_parallel_member(&task.id, &a, None).unwrap();
    // one free slot → pending agent wakes
    assert_eq!(mid.open_agents.len(), 2);
    assert!(mid.pending_agents.is_empty());

    let mid2 = store.complete_parallel_member(&task.id, &b, None).unwrap();
    let mid3 = store
        .complete_parallel_member(&task.id, &mid2.open_agents[0], None)
        .unwrap();
    // after draining the third
    let drained = if mid3.open_agents.is_empty() {
        mid3
    } else {
        store
            .complete_parallel_member(&task.id, &mid3.open_agents[0], None)
            .unwrap()
    };
    assert!(drained.open_agents.is_empty());
    assert!(drained.pending_agents.is_empty());

    // Retry current parallel stage once (max_retries=1 on those steps)
    let retried = store
        .retry_task(&task.id, Some("human"), Some("impl flaked"))
        .unwrap();
    assert_eq!(retried.step_index, 1);
    assert_eq!(retried.open_agents.len(), 2);

    // Drain again
    let mut cur = retried;
    while !cur.open_agents.is_empty() || !cur.pending_agents.is_empty() {
        let agent = cur.open_agents[0].clone();
        cur = store
            .complete_parallel_member(&task.id, &agent, None)
            .unwrap();
    }

    let done = store.advance_task(&task.id, Some("dev_a"), None).unwrap(); // review stage
    assert_eq!(done.step_index, 2);
    let finished = store
        .advance_task(&task.id, Some("reviewer"), None)
        .unwrap();
    assert_eq!(finished.status, "done");
}

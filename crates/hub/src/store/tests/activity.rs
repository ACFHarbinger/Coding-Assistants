use super::super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn activity_view_aggregates_tasks_with_agents_commands_and_files() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let root = dir.path().join("workspace");
    fs::create_dir_all(&root).unwrap();

    let steps = vec![
        WorkflowStep {
            agent: "claude".into(),
            role: Some("lead".into()),
            instruction: "Plan feature implementation".into(),
            max_retries: 0,
            parallel_group: None,
        },
        WorkflowStep {
            agent: "gemini".into(),
            role: Some("ui".into()),
            instruction: "Implement Dashboard activity view".into(),
            max_retries: 0,
            parallel_group: None,
        },
    ];

    let task = store
        .create_task(
            "Implement D4 Activity",
            Some(&root.to_string_lossy()),
            &steps,
        )
        .unwrap();

    store
        .record_audit_event(
            &root,
            Path::new("src/components/DashboardPanel.tsx"),
            "modified",
            r#"{"pid": 1234, "cmdline": ["cargo", "test", "-p", "hub"], "agent": "gemini"}"#,
            Some("hash123"),
        )
        .unwrap();

    let filter = ActivityFilter::default();
    let items = store.get_activity_view(&filter).unwrap();

    assert!(!items.is_empty());
    let task_item = items
        .iter()
        .find(|i| i.id == task.id)
        .expect("task item exists");
    assert_eq!(task_item.kind, "task");
    assert_eq!(task_item.title, "Implement D4 Activity");
    assert!(task_item.agents.contains(&"claude".to_string()));
    assert!(task_item.agents.contains(&"gemini".to_string()));

    assert_eq!(task_item.files.len(), 1);
    assert_eq!(task_item.files[0].path, "src/components/DashboardPanel.tsx");
    assert_eq!(task_item.files[0].operation, "modified");

    assert_eq!(task_item.commands.len(), 1);
    assert_eq!(task_item.commands[0].raw, "cargo test -p hub");
    assert_eq!(task_item.commands[0].attribution.as_deref(), Some("gemini"));
}

#[test]
fn activity_view_aggregates_work_sessions_with_members_and_captures() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    store.upsert_agent("claude", "Claude").unwrap();
    store.upsert_agent("gemini", "Gemini").unwrap();

    let members = vec!["claude".to_string(), "gemini".to_string()];
    let session = store
        .create_work_session_with_members("Test Work Session", Some(&members))
        .unwrap();

    store
        .record_harness_capture(
            "gemini",
            "gemini",
            Some(&session.id),
            "Executing verification:\n$ npm test\nCompleted clean.",
            None,
        )
        .unwrap();

    let filter = ActivityFilter::default();
    let items = store.get_activity_view(&filter).unwrap();

    let session_item = items
        .iter()
        .find(|i| i.id == session.id)
        .expect("session item exists");

    assert_eq!(session_item.kind, "work_session");
    assert_eq!(session_item.title, "Test Work Session");
    assert!(session_item.agents.contains(&"claude".to_string()));
    assert!(session_item.agents.contains(&"gemini".to_string()));
    assert_eq!(session_item.capture_count, 1);

    assert!(session_item
        .commands
        .iter()
        .any(|c| c.raw == "npm test" && c.source == "capture"));
}

#[test]
fn activity_view_filters_by_agent() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let steps_claude = vec![WorkflowStep {
        agent: "claude".into(),
        role: None,
        instruction: "Work on backend".into(),
        max_retries: 0,
        parallel_group: None,
    }];
    let steps_grok = vec![WorkflowStep {
        agent: "grok".into(),
        role: None,
        instruction: "Work on leader".into(),
        max_retries: 0,
        parallel_group: None,
    }];

    store
        .create_task("Claude Task", None, &steps_claude)
        .unwrap();
    store.create_task("Grok Task", None, &steps_grok).unwrap();

    let filter_claude = ActivityFilter {
        agent: Some("claude".into()),
        ..Default::default()
    };
    let items_claude = store.get_activity_view(&filter_claude).unwrap();
    assert_eq!(items_claude.len(), 1);
    assert_eq!(items_claude[0].title, "Claude Task");

    let filter_grok = ActivityFilter {
        agent: Some("grok".into()),
        ..Default::default()
    };
    let items_grok = store.get_activity_view(&filter_grok).unwrap();
    assert_eq!(items_grok.len(), 1);
    assert_eq!(items_grok[0].title, "Grok Task");

    let filter_none = ActivityFilter {
        agent: Some("nonexistent_agent".into()),
        ..Default::default()
    };
    let items_none = store.get_activity_view(&filter_none).unwrap();
    assert!(items_none.is_empty());
}

#[test]
fn activity_view_filters_by_time_range_and_limit() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();

    let steps = vec![WorkflowStep {
        agent: "gemini".into(),
        role: None,
        instruction: "Work on dashboard".into(),
        max_retries: 0,
        parallel_group: None,
    }];

    let task1 = store.create_task("Old Task", None, &steps).unwrap();
    let task2 = store.create_task("New Task", None, &steps).unwrap();

    let filter_limit = ActivityFilter {
        limit: Some(1),
        ..Default::default()
    };
    let items_limited = store.get_activity_view(&filter_limit).unwrap();
    assert_eq!(items_limited.len(), 1);

    let filter_since = ActivityFilter {
        since: Some(task2.created_at.clone()),
        ..Default::default()
    };
    let items_since = store.get_activity_view(&filter_since).unwrap();
    assert!(items_since.iter().any(|i| i.id == task2.id));

    let filter_kind = ActivityFilter {
        kind: Some("work_session".into()),
        ..Default::default()
    };
    let items_sessions = store.get_activity_view(&filter_kind).unwrap();
    assert!(items_sessions.iter().all(|i| i.kind == "work_session"));
    assert!(!items_sessions.iter().any(|i| i.id == task1.id));
}

#[test]
fn activity_view_workspace_filter_excludes_unscoped_and_prefix_workspaces() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let steps = vec![WorkflowStep {
        agent: "gemini".into(),
        role: None,
        instruction: "Inspect activity".into(),
        max_retries: 0,
        parallel_group: None,
    }];

    let matching = store
        .create_task("Matching workspace", Some("/tmp/activity-project"), &steps)
        .unwrap();
    let prefixed = store
        .create_task(
            "Prefix workspace",
            Some("/tmp/activity-project-old"),
            &steps,
        )
        .unwrap();
    let unscoped = store.create_task("Unscoped", None, &steps).unwrap();
    let session = store.create_work_session("Unscoped session").unwrap();

    let items = store
        .get_activity_view(&ActivityFilter {
            workspace_path: Some("/tmp/activity-project".into()),
            ..Default::default()
        })
        .unwrap();

    assert!(items.iter().any(|item| item.id == matching.id));
    assert!(!items.iter().any(|item| item.id == prefixed.id));
    assert!(!items.iter().any(|item| item.id == unscoped.id));
    assert!(!items.iter().any(|item| item.id == session.id));
}

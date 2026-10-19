use super::super::*;
use std::fs;
use tempfile::tempdir;

fn open() -> (HubStore, std::path::PathBuf, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    let store = HubStore::open(&home).unwrap();
    (store, dir.path().to_path_buf(), dir)
}

fn make_task(
    store: &HubStore,
    workspace: &str,
    criteria: Vec<GateCriterion>,
    combinator: GateCombinator,
) -> ScheduledTask {
    store
        .create_scheduled_task(
            "n",
            "i",
            ScheduleTarget::Team,
            workspace,
            "2026-10-20T00:00:00Z",
            None,
            60,
            criteria,
            combinator,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap()
}

#[test]
fn empty_gate_criteria_always_passes() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("empty-ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![],
        GateCombinator::All,
    );
    let result = store.evaluate_gate_criteria(&task).unwrap();
    assert!(result.passed);
    assert!(result.outcomes.is_empty());
}

#[test]
fn command_exit_code_zero_passes_and_nonzero_fails() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("cmd-ws");
    fs::create_dir_all(&workspace).unwrap();
    let pass = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::Command {
            program: "true".into(),
            args: vec![],
            check: GateCheck::ExitCodeZero,
        }],
        GateCombinator::All,
    );
    assert!(store.evaluate_gate_criteria(&pass).unwrap().passed);

    let fail = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::Command {
            program: "false".into(),
            args: vec![],
            check: GateCheck::ExitCodeZero,
        }],
        GateCombinator::All,
    );
    assert!(!store.evaluate_gate_criteria(&fail).unwrap().passed);
}

#[test]
fn max_lines_per_file_flags_only_the_file_over_the_cap() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("loc-ws");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(workspace.join("small.rs"), "a\nb\nc\n").unwrap();
    let big: String = (0..10).map(|i| format!("line {i}\n")).collect();
    fs::write(workspace.join("big.rs"), big).unwrap();

    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::MaxLinesPerFile {
            max_lines: 5,
            globs: vec!["*.rs".into()],
        }],
        GateCombinator::All,
    );
    let result = store.evaluate_gate_criteria(&task).unwrap();
    assert!(!result.passed);
    assert!(result.outcomes[0].detail.contains("big.rs"));
    assert!(!result.outcomes[0].detail.contains("small.rs"));
}

#[test]
fn max_lines_per_file_passes_when_everything_matched_is_under_the_cap() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("loc-ok-ws");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(workspace.join("small.rs"), "a\nb\n").unwrap();

    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::MaxLinesPerFile {
            max_lines: 500,
            globs: vec!["*.rs".into()],
        }],
        GateCombinator::All,
    );
    assert!(store.evaluate_gate_criteria(&task).unwrap().passed);
}

#[test]
fn min_tokens_written_measures_delta_from_baseline() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("tok-ws");
    fs::create_dir_all(&workspace).unwrap();

    store.record_agent_metrics("grok", 0, 1000, 0, 0).unwrap();

    let agent_task = store
        .create_scheduled_task(
            "n2",
            "i",
            ScheduleTarget::Agent {
                agent_id: "grok".into(),
            },
            &workspace.to_string_lossy(),
            "2026-10-20T00:00:00Z",
            None,
            60,
            vec![GateCriterion::MinTokensWritten { min_tokens: 500 }],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap();
    assert_eq!(agent_task.token_baseline.get("grok"), Some(&1000));

    // No new tokens written yet -> delta 0 < 500 -> fails.
    let result = store.evaluate_gate_criteria(&agent_task).unwrap();
    assert!(!result.passed);

    // Write 600 more tokens -> delta 600 >= 500 -> passes.
    store.record_agent_metrics("grok", 0, 600, 0, 0).unwrap();
    let result = store.evaluate_gate_criteria(&agent_task).unwrap();
    assert!(result.passed);
}

#[test]
fn any_combinator_passes_when_one_criterion_passes() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("any-ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![
            GateCriterion::Command {
                program: "false".into(),
                args: vec![],
                check: GateCheck::ExitCodeZero,
            },
            GateCriterion::Command {
                program: "true".into(),
                args: vec![],
                check: GateCheck::ExitCodeZero,
            },
        ],
        GateCombinator::Any,
    );
    assert!(store.evaluate_gate_criteria(&task).unwrap().passed);
}

fn proposal_count(store: &HubStore) -> usize {
    store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .filter(|event| event.operation == "os_tool.proposed")
        .count()
}

#[test]
fn pending_gate_proposal_is_reused_then_its_approved_run_is_consumed() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("approval-ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::Command {
            program: "cargo".into(),
            args: vec!["--version".into()],
            check: GateCheck::ExitCodeZero,
        }],
        GateCombinator::All,
    );

    let first = store.evaluate_gate_criteria(&task).unwrap();
    assert!(!first.passed);
    assert!(first.pending);
    let audit_id = first.outcomes[0].audit_id.clone().unwrap();

    // Repeated gate checks must reuse the same outstanding proposal.
    let second = store.evaluate_gate_criteria(&task).unwrap();
    assert!(!second.passed);
    assert!(second.pending);
    assert_eq!(
        second.outcomes[0].audit_id.as_deref(),
        Some(audit_id.as_str())
    );
    assert_eq!(proposal_count(&store), 1);

    let ran = crate::os_tool::approve(&store, &audit_id, crate::os_tool::DEFAULT_TIMEOUT).unwrap();
    assert_eq!(ran.status, "ran");

    let third = store.evaluate_gate_criteria(&task).unwrap();
    assert!(third.passed);
    assert!(!third.pending);
    assert!(third.outcomes[0].detail.contains("approved"));
    assert_eq!(proposal_count(&store), 1, "the approved run is consumed");
}

#[test]
fn quarantined_gate_proposal_fails_closed_without_resubmitting() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("quarantined-ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::Command {
            program: "cargo".into(),
            args: vec!["--version".into()],
            check: GateCheck::ExitCodeZero,
        }],
        GateCombinator::All,
    );
    let pending = store.evaluate_gate_criteria(&task).unwrap();
    let audit_id = pending.outcomes[0].audit_id.clone().unwrap();
    store.set_audit_status(&audit_id, "quarantined").unwrap();

    let denied = store.evaluate_gate_criteria(&task).unwrap();
    assert!(!denied.passed);
    assert!(!denied.pending);
    assert!(denied.outcomes[0].detail.contains("denied"));
    assert_eq!(proposal_count(&store), 1);
}

#[test]
fn approved_but_failed_run_is_reported_as_failure_not_pending() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("failed-run-ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::Command {
            program: "cargo".into(),
            args: vec!["definitely-not-a-cargo-command".into()],
            check: GateCheck::ExitCodeZero,
        }],
        GateCombinator::All,
    );
    let pending = store.evaluate_gate_criteria(&task).unwrap();
    let audit_id = pending.outcomes[0].audit_id.clone().unwrap();
    crate::os_tool::approve(&store, &audit_id, crate::os_tool::DEFAULT_TIMEOUT).unwrap();

    let failed = store.evaluate_gate_criteria(&task).unwrap();
    assert!(!failed.passed);
    assert!(!failed.pending);
    assert!(failed.outcomes[0]
        .detail
        .contains("approved exit=Some(101)"));
    assert_eq!(proposal_count(&store), 1);
}

#[test]
fn hard_denied_command_fails_without_creating_a_proposal() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("denied-ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::Command {
            program: "bash".into(),
            args: vec!["-c".into(), "echo 0".into()],
            check: GateCheck::ExitCodeZero,
        }],
        GateCombinator::All,
    );
    let first = store.evaluate_gate_criteria(&task).unwrap();
    assert!(!first.passed);
    assert!(!first.pending);
    assert_eq!(proposal_count(&store), 0);
}

#[test]
fn empty_token_baseline_and_missing_measurement_fail_closed() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("token-ws");
    fs::create_dir_all(&workspace).unwrap();

    let mut task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::MinTokensWritten { min_tokens: 1 }],
        GateCombinator::All,
    );
    task.token_baseline.clear();
    let empty = store.evaluate_gate_criteria(&task).unwrap();
    assert!(!empty.passed);
    assert!(empty.outcomes[0].detail.contains("no token baseline"));

    let ghost = store
        .create_scheduled_task(
            "ghost",
            "i",
            ScheduleTarget::Agent {
                agent_id: "ghost".into(),
            },
            &workspace.to_string_lossy(),
            "2026-10-20T00:00:00Z",
            None,
            60,
            vec![GateCriterion::MinTokensWritten { min_tokens: 1 }],
            GateCombinator::All,
            BudgetPolicy::default(),
            3,
            None,
            "owner",
        )
        .unwrap();
    let unavailable = store.evaluate_gate_criteria(&ghost).unwrap();
    assert!(!unavailable.passed);
    assert!(unavailable.outcomes[0].detail.contains("no measurement"));
}

#[test]
fn missing_workspace_fails_the_gate_instead_of_erroring_or_passing() {
    let (store, ws, _guard) = open();
    let missing = ws.join("does-not-exist");
    let task = make_task(
        &store,
        &missing.to_string_lossy(),
        vec![GateCriterion::MaxLinesPerFile {
            max_lines: 500,
            globs: vec!["**/*.rs".into()],
        }],
        GateCombinator::All,
    );
    let result = store.evaluate_gate_criteria(&task).unwrap();
    assert!(!result.passed);
    assert!(!result.pending);
    assert!(result.outcomes[0].detail.contains("workspace"));
}

#[test]
fn command_details_are_bounded() {
    let (store, ws, _guard) = open();
    let workspace = ws.join("bounded-ws");
    fs::create_dir_all(&workspace).unwrap();
    let task = make_task(
        &store,
        &workspace.to_string_lossy(),
        vec![GateCriterion::Command {
            program: "echo".into(),
            args: vec!["x".repeat(4_000)],
            check: GateCheck::ExitCodeZero,
        }],
        GateCombinator::All,
    );
    let result = store.evaluate_gate_criteria(&task).unwrap();
    assert!(result.passed);
    assert!(result.outcomes[0].detail.chars().count() <= 481);
}

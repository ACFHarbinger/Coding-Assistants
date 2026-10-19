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

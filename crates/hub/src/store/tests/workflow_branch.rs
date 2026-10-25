//! P20 (#367): conditional branching on `WorkflowStep`.
use super::super::*;
use std::fs;
use tempfile::tempdir;

fn open() -> (HubStore, String, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path().join("home")).unwrap();
    // These tests exercise branch routing, not the #365 approval-gate
    // wait state — keep wakes ungated.
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

fn command(program: &str) -> GateCriterion {
    GateCriterion::Command {
        program: program.into(),
        args: vec![],
        check: GateCheck::ExitCodeZero,
    }
}

fn branch(program: &str, on_pass: BranchTarget, on_fail: BranchTarget) -> Option<StepBranch> {
    Some(StepBranch {
        criteria: vec![command(program)],
        combinator: GateCombinator::All,
        on_pass,
        on_fail,
    })
}

#[test]
fn passing_branch_skips_to_its_target_stage_and_records_the_route() {
    let (store, ws, _guard) = open();
    let mut plan = step("planner", "plan");
    plan.branch = branch("true", BranchTarget::Stage { index: 2 }, BranchTarget::Next);
    let steps = vec![plan, step("dev", "implement"), step("reviewer", "review")];
    let task = store
        .create_task_with_parallel("branchy", Some(&ws), &steps, 4, false)
        .unwrap();

    store.advance_task(&task.id, None, None).unwrap();
    let routed = store
        .advance_task(&task.id, Some("planner"), Some("nothing to implement"))
        .unwrap();
    assert_eq!(routed.status, "running");
    assert_eq!(routed.step_index, 2, "dev stage skipped");
    assert!(store.poll_messages("dev", true).unwrap().is_empty());
    assert_eq!(store.poll_messages("reviewer", true).unwrap().len(), 1);

    let ops: Vec<String> = store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .map(|event| event.operation)
        .collect();
    assert!(ops.iter().any(|op| op == "task.branch_checked"));
    assert!(ops.iter().any(|op| op == "task.branch_routed"));
    assert!(!ops.iter().any(|op| op == "schedule.gate_checked"));
}

#[test]
fn failing_branch_takes_on_fail_and_linear_default_still_advances() {
    let (store, ws, _guard) = open();
    let mut plan = step("planner", "plan");
    plan.branch = branch("false", BranchTarget::Done, BranchTarget::Next);
    let steps = vec![plan, step("dev", "implement")];
    let task = store
        .create_task_with_parallel("branchy", Some(&ws), &steps, 4, false)
        .unwrap();

    store.advance_task(&task.id, None, None).unwrap();
    let routed = store.advance_task(&task.id, Some("planner"), None).unwrap();
    assert_eq!(routed.step_index, 1);
    assert_eq!(routed.status, "running");
    let done = store.advance_task(&task.id, Some("dev"), None).unwrap();
    assert_eq!(done.status, "done");
}

#[test]
fn done_and_fail_targets_finish_the_task_early() {
    let (store, ws, _guard) = open();
    let mut plan = step("planner", "plan");
    plan.branch = branch("true", BranchTarget::Done, BranchTarget::Next);
    let task = store
        .create_task_with_parallel(
            "early-done",
            Some(&ws),
            &[plan, step("dev", "implement")],
            4,
            false,
        )
        .unwrap();
    store.advance_task(&task.id, None, None).unwrap();
    let done = store.advance_task(&task.id, Some("planner"), None).unwrap();
    assert_eq!(done.status, "done");
    assert!(store.poll_messages("dev", true).unwrap().is_empty());

    let mut check = step("tester", "run the suite");
    check.branch = branch("false", BranchTarget::Next, BranchTarget::Fail);
    let task = store
        .create_task_with_parallel(
            "early-fail",
            Some(&ws),
            &[check, step("dev", "ship")],
            4,
            false,
        )
        .unwrap();
    store.advance_task(&task.id, None, None).unwrap();
    let failed = store.advance_task(&task.id, Some("tester"), None).unwrap();
    assert_eq!(failed.status, "failed");
    assert!(store.poll_messages("dev", true).unwrap().is_empty());
}

#[test]
fn back_edge_loops_within_max_retries_then_fails() {
    let (store, ws, _guard) = open();
    let mut dev = step("dev", "implement");
    dev.max_retries = 1;
    let mut tester = step("tester", "test");
    // Tests never pass in this fixture: loop back to dev until its budget ends.
    tester.branch = branch(
        "false",
        BranchTarget::Next,
        BranchTarget::Stage { index: 0 },
    );
    let task = store
        .create_task_with_parallel("loop", Some(&ws), &[dev, tester], 4, false)
        .unwrap();

    store.advance_task(&task.id, None, None).unwrap(); // dev #1
    store.advance_task(&task.id, Some("dev"), None).unwrap(); // tester #1
    let looped = store.advance_task(&task.id, Some("tester"), None).unwrap();
    assert_eq!(looped.status, "running");
    assert_eq!(looped.step_index, 0, "back-edge re-activated dev");
    assert_eq!(looped.attempts.get("0"), Some(&2));
    assert_eq!(store.poll_messages("dev", true).unwrap().len(), 2);

    store.advance_task(&task.id, Some("dev"), None).unwrap(); // tester #2
    let exhausted = store.advance_task(&task.id, Some("tester"), None).unwrap();
    assert_eq!(exhausted.status, "failed", "dev's max_retries (1) is spent");
    assert!(
        store.poll_messages("dev", true).unwrap().is_empty(),
        "no third dispatch once the back-edge budget is spent"
    );
}

#[test]
fn create_task_validates_branches() {
    let (store, ws, _guard) = open();

    let mut no_criteria = step("a", "x");
    no_criteria.branch = Some(StepBranch {
        criteria: vec![],
        combinator: GateCombinator::All,
        on_pass: BranchTarget::Next,
        on_fail: BranchTarget::Next,
    });
    let err = store
        .create_task("t", Some(&ws), &[no_criteria])
        .unwrap_err()
        .to_string();
    assert!(err.contains("at least one criterion"), "{err}");

    let mut out_of_range = step("a", "x");
    out_of_range.branch = branch("true", BranchTarget::Stage { index: 5 }, BranchTarget::Next);
    let err = store
        .create_task("t", Some(&ws), &[out_of_range])
        .unwrap_err()
        .to_string();
    assert!(err.contains("targets stage 5"), "{err}");

    let mut needs_ws = step("a", "x");
    needs_ws.branch = branch("true", BranchTarget::Next, BranchTarget::Next);
    let err = store
        .create_task("t", None, &[needs_ws])
        .unwrap_err()
        .to_string();
    assert!(err.contains("workspace_path"), "{err}");

    let mut left = step("a", "x");
    left.parallel_group = Some("g".into());
    left.branch = branch("true", BranchTarget::Next, BranchTarget::Next);
    let mut right = step("b", "y");
    right.parallel_group = Some("g".into());
    right.branch = branch("true", BranchTarget::Next, BranchTarget::Next);
    let err = store
        .create_task("t", Some(&ws), &[left, right])
        .unwrap_err()
        .to_string();
    assert!(err.contains("only one step may define a branch"), "{err}");
}

#[test]
fn branch_json_round_trips_and_is_omitted_when_unset() {
    let plain = serde_json::to_value(step("a", "x")).unwrap();
    assert!(plain.get("branch").is_none());

    let parsed: WorkflowStep = serde_json::from_value(serde_json::json!({
        "agent": "tester",
        "instruction": "run tests",
        "branch": {
            "criteria": [{ "kind": "command", "program": "cargo", "args": ["test"], "check": "exit_code_zero" }],
            "on_pass": { "kind": "done" },
            "on_fail": { "kind": "stage", "index": 0 }
        }
    }))
    .unwrap();
    let branch = parsed.branch.unwrap();
    assert_eq!(branch.combinator, GateCombinator::All);
    assert_eq!(branch.on_pass, BranchTarget::Done);
    assert_eq!(branch.on_fail, BranchTarget::Stage { index: 0 });
}

#[test]
fn parallel_stage_branch_uses_the_whole_stage_as_its_subject() {
    let (store, ws, _guard) = open();
    let mut a = step("dev_a", "part a");
    a.parallel_group = Some("impl".into());
    a.branch = branch("true", BranchTarget::Done, BranchTarget::Next);
    let mut b = step("dev_b", "part b");
    b.parallel_group = Some("impl".into());
    let task = store
        .create_task_with_parallel(
            "par",
            Some(&ws),
            &[a, b, step("reviewer", "review")],
            4,
            false,
        )
        .unwrap();
    store.advance_task(&task.id, None, None).unwrap();
    store
        .complete_parallel_member(&task.id, "dev_a", None)
        .unwrap();
    store
        .complete_parallel_member(&task.id, "dev_b", None)
        .unwrap();
    let done = store.advance_task(&task.id, Some("dev_b"), None).unwrap();
    assert_eq!(done.status, "done");
    assert!(store.poll_messages("reviewer", true).unwrap().is_empty());
}

#[test]
fn branch_approval_is_keyed_by_stage_not_just_task_and_attempt() {
    let (store, ws, _guard) = open();
    let mut first = step("planner", "plan");
    first.branch = Some(StepBranch {
        criteria: vec![GateCriterion::Command {
            program: "cargo".into(),
            args: vec!["--version".into()],
            check: GateCheck::ExitCodeZero,
        }],
        combinator: GateCombinator::All,
        on_pass: BranchTarget::Next,
        on_fail: BranchTarget::Fail,
    });
    let mut second = step("tester", "test");
    second.branch = Some(StepBranch {
        criteria: vec![GateCriterion::Command {
            program: "cargo".into(),
            args: vec!["definitely-not-a-cargo-command".into()],
            check: GateCheck::ExitCodeZero,
        }],
        combinator: GateCombinator::All,
        on_pass: BranchTarget::Done,
        on_fail: BranchTarget::Fail,
    });
    let task = store
        .create_task_with_parallel("keyed", Some(&ws), &[first, second], 4, false)
        .unwrap();
    store.advance_task(&task.id, None, None).unwrap();

    let pending = store
        .advance_task(&task.id, Some("planner"), None)
        .unwrap_err()
        .to_string();
    assert!(pending.contains("awaiting human approval"), "{pending}");
    let events = store.list_audit_events(false).unwrap();
    let proposal = events
        .iter()
        .rev()
        .find(|event| event.operation == "os_tool.proposed")
        .unwrap();
    crate::os_tool::approve(&store, &proposal.id, crate::os_tool::DEFAULT_TIMEOUT).unwrap();

    let routed = store.advance_task(&task.id, Some("planner"), None).unwrap();
    assert_eq!(routed.step_index, 1);

    let stage1 = store
        .advance_task(&task.id, Some("tester"), None)
        .unwrap_err()
        .to_string();
    assert!(
        stage1.contains("awaiting human approval"),
        "stage 1 must not reuse stage 0's approved run: {stage1}"
    );
    let proposals: Vec<_> = store
        .list_audit_events(false)
        .unwrap()
        .into_iter()
        .filter(|event| event.operation == "os_tool.proposed")
        .collect();
    assert_eq!(proposals.len(), 2, "each stage needs its own proposal");
}

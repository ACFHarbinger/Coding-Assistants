//! Tests for [`super`] (`ca schedule`, S4 / #354). Split out to keep both
//! files under the 500-LoC cap.

use super::*;
use crate::app::{Cli, Command, ScheduleCommand};
use clap::Parser;

fn open() -> (HubStore, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let store = HubStore::open(&home).unwrap();
    let workspace = dir.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();
    (store, workspace.to_string_lossy().into_owned(), dir)
}

fn base_args<'a>(workspace: &'a str, fire_at: &'a str) -> CreateSchedule<'a> {
    CreateSchedule {
        name: "n",
        instruction: "do the thing",
        target: "cursor",
        workspace,
        fire_at,
        repeat_interval_secs: None,
        response_timeout_secs: 3600,
        gate_criteria: vec![],
        gate_combinator: "all",
        credit_allowance: None,
        stop_on_quota_exhausted: false,
        max_attempts: 3,
        deadline: None,
        created_by: "human",
    }
}

#[test]
fn clap_wires_create_and_tick() {
    let cli = Cli::try_parse_from([
        "ca",
        "schedule",
        "create",
        "--name",
        "nightly",
        "--instruction",
        "sweep",
        "--target",
        "team",
        "--workspace",
        "/tmp",
        "--fire-at",
        "2026-01-01T00:00:00Z",
        "--max-attempts",
        "5",
    ])
    .unwrap();
    match cli.command {
        Command::Schedule {
            action: ScheduleCommand::Create(args),
        } => {
            assert_eq!(args.name, "nightly");
            assert_eq!(args.max_attempts, 5);
        }
        _ => panic!("expected ca schedule create"),
    }

    let cli =
        Cli::try_parse_from(["ca", "schedule", "tick", "--now", "2026-01-01T00:00:00Z"]).unwrap();
    match cli.command {
        Command::Schedule {
            action: ScheduleCommand::Tick { now },
        } => assert_eq!(now.as_deref(), Some("2026-01-01T00:00:00Z")),
        _ => panic!("expected ca schedule tick"),
    }
}

#[test]
fn parse_target_accepts_team_case_insensitive_and_agent_ids() {
    assert_eq!(parse_target("team").unwrap(), ScheduleTarget::Team);
    assert_eq!(parse_target("TEAM").unwrap(), ScheduleTarget::Team);
    assert_eq!(
        parse_target("cursor").unwrap(),
        ScheduleTarget::Agent {
            agent_id: "cursor".into()
        }
    );
    assert!(parse_target("   ").is_err());
}

#[test]
fn parse_combinator_rejects_unknown_values() {
    assert_eq!(parse_combinator("all").unwrap(), GateCombinator::All);
    assert_eq!(parse_combinator("any").unwrap(), GateCombinator::Any);
    assert!(parse_combinator("xor").is_err());
}

#[test]
fn parse_gate_criteria_accepts_hub_shape_and_rejects_junk() {
    let ok = parse_gate_criteria(
        r#"[{"kind":"max_lines_per_file","max_lines":100,"globs":["**/*.rs"]}]"#,
    )
    .unwrap();
    assert_eq!(ok.len(), 1);
    assert!(parse_gate_criteria("not json").is_err());
    assert!(parse_gate_criteria(r#"[{"kind":"nope"}]"#).is_err());
    assert!(parse_gate_criteria(r#"{"kind":"command"}"#).is_err());
}

#[test]
fn create_validates_inputs_before_touching_the_store() {
    let (store, workspace, _dir) = open();

    let bad: Vec<CreateSchedule<'_>> = vec![
        CreateSchedule {
            name: "",
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            instruction: "",
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            target: "",
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            workspace: "relative/path",
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            fire_at: "tomorrow",
            ..base_args(&workspace, "tomorrow")
        },
        CreateSchedule {
            deadline: Some("not-a-time"),
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            repeat_interval_secs: Some(0),
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            response_timeout_secs: 0,
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            credit_allowance: Some(-1.0),
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            credit_allowance: Some(f64::NAN),
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            max_attempts: 0,
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            created_by: "",
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
        CreateSchedule {
            gate_combinator: "xor",
            ..base_args(&workspace, "2026-01-01T00:00:00Z")
        },
    ];
    for args in bad {
        assert!(create_task(&store, args).is_err(), "expected rejection");
    }
    assert!(list_tasks(&store, None).unwrap().is_empty());
}

#[test]
fn create_get_list_cancel_round_trip() {
    let (store, workspace, _dir) = open();
    let created = create_task(&store, base_args(&workspace, "2099-01-01T00:00:00Z")).unwrap();
    assert_eq!(created.status, ScheduleStatus::Scheduled);
    assert_eq!(created.created_by, "human");

    let fetched = get_task(&store, &created.id).unwrap();
    assert_eq!(fetched.id, created.id);
    assert!(get_task(&store, "missing").is_err());

    assert_eq!(list_tasks(&store, None).unwrap().len(), 1);
    assert_eq!(list_tasks(&store, Some("scheduled")).unwrap().len(), 1);
    assert!(list_tasks(&store, Some("bogus")).is_err());

    let cancelled = cancel_task(&store, &created.id).unwrap();
    assert_eq!(cancelled.status, ScheduleStatus::Cancelled);
    assert_eq!(list_tasks(&store, Some("cancelled")).unwrap().len(), 1);
    assert_eq!(list_tasks(&store, Some("scheduled")).unwrap().len(), 0);

    // Cancelling a terminal task is a no-op, not an error.
    let again = cancel_task(&store, &created.id).unwrap();
    assert_eq!(again.status, ScheduleStatus::Cancelled);
}

#[test]
fn create_keeps_stop_on_quota_exhausted_flag() {
    let (store, workspace, _dir) = open();
    let mut args = base_args(&workspace, "2099-01-01T00:00:00Z");
    args.stop_on_quota_exhausted = true;
    let created = create_task(&store, args).unwrap();
    assert!(created.budget.stop_on_quota_exhausted);
}

/// S5c: headless tick honors `--stop-on-quota-exhausted` — a measurable
/// session/weekly window at 0% remaining pauses instead of dispatching.
#[test]
fn tick_pauses_quota_exhausted_task_headless() {
    let (store, workspace, _dir) = open();
    let mut args = base_args(&workspace, "2020-01-01T00:00:00Z");
    args.stop_on_quota_exhausted = true;
    args.repeat_interval_secs = Some(60);
    let created = create_task(&store, args).unwrap();

    let summary = tick_with_probe(&store, "2020-01-01T00:00:01Z", |agent_id| {
        assert_eq!(agent_id, "cursor");
        ProviderQuota {
            agent_id: agent_id.into(),
            provider: agent_id.into(),
            harness_title: agent_id.into(),
            status: "ok".into(),
            detail: None,
            windows: vec![quota::ProviderQuotaWindow {
                label: "5h".into(),
                family: None,
                used_percent: 100,
                remaining_percent: 0,
                resets_at: None,
                window_minutes: Some(300),
            }],
            fetched_at: 0,
            balance: None,
            balance_info: None,
            local_usage: None,
        }
    })
    .unwrap();
    assert!(summary.outcomes.iter().any(|o| o.action == "paused"));
    assert!(!summary.outcomes.iter().any(|o| o.action == "dispatched"));
    let fetched = get_task(&store, &created.id).unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Paused);
    assert_eq!(fetched.attempts, 0);
}

/// S5c: tasks without the flag never trigger a probe at all (no provider
/// API cost), and unavailable quota never invents a stop.
#[test]
fn tick_never_probes_tasks_without_the_quota_flag() {
    let (store, workspace, _dir) = open();
    let created = create_task(&store, base_args(&workspace, "2020-01-01T00:00:00Z")).unwrap();

    let summary = tick_with_probe(&store, "2020-01-01T00:00:01Z", |_| {
        panic!("probe must not run for a task without stop_on_quota_exhausted")
    })
    .unwrap();
    assert_eq!(summary.outcomes[0].id, created.id);
    assert_eq!(summary.outcomes[0].action, "completed");
}

#[test]
fn tick_dispatches_due_one_shot_and_summary_is_the_cli_result() {
    let (store, workspace, _dir) = open();
    let created = create_task(&store, base_args(&workspace, "2020-01-01T00:00:00Z")).unwrap();

    let summary = tick(&store, Some("2020-01-01T00:00:01Z")).unwrap();
    assert_eq!(summary.outcomes.len(), 1);
    assert_eq!(summary.outcomes[0].id, created.id);
    assert_eq!(summary.outcomes[0].action, "completed");

    // The hub's TickSummary is reused verbatim as the CLI's JSON output.
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["outcomes"][0]["action"], "completed");

    let fetched = get_task(&store, &created.id).unwrap();
    assert_eq!(fetched.status, ScheduleStatus::Completed);
    assert_eq!(fetched.attempts, 1);
}

#[test]
fn tick_is_empty_when_nothing_is_due() {
    let (store, workspace, _dir) = open();
    create_task(&store, base_args(&workspace, "2099-01-01T00:00:00Z")).unwrap();
    let summary = tick(&store, Some("2026-01-01T00:00:00Z")).unwrap();
    assert!(summary.outcomes.is_empty());
}

#[test]
fn tick_rejects_a_non_rfc3339_now() {
    let (store, _workspace, _dir) = open();
    assert!(tick(&store, Some("later")).is_err());
}

#[test]
fn pending_proposal_count_is_zero_on_a_fresh_store() {
    let (store, _workspace, _dir) = open();
    assert_eq!(pending_proposal_count(&store).unwrap(), 0);
}

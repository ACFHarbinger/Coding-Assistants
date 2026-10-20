//! `ca schedule` runner (Scheduled Goals S4 / #354).
//!
//! Thin wrapper over the shared Hub API: dispatch, gate evaluation, budget,
//! and wake policy all live in `hub` (S1-S3) and are reused, never
//! re-implemented here. Headless `tick` can only enforce the in-hub
//! `credit_allowance` — the live provider-quota probe is `src-tauri`-side
//! and unreachable from the CLI (roadmap S5c bridges that gap; the
//! distinction is surfaced in command help, `--stop-on-quota-exhausted`
//! notes, and post-tick stderr so a headless operator is never told quota
//! was checked when it was not).

use anyhow::{bail, Context};
use chrono::DateTime;
use hub::{
    BudgetPolicy, GateCombinator, GateCriterion, HubStore, ScheduleStatus, ScheduleTarget,
    ScheduledTask, TickSummary,
};

use crate::app::ScheduleCommand;

pub(crate) fn run(store: &HubStore, action: ScheduleCommand) -> anyhow::Result<()> {
    match action {
        ScheduleCommand::List { status } => {
            let tasks = list_tasks(store, status.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&tasks)?);
        }
        ScheduleCommand::Create(args) => {
            let crate::app::ScheduleCreateArgs {
                name,
                instruction,
                target,
                workspace,
                fire_at,
                repeat_interval_secs,
                response_timeout_secs,
                gate_criteria,
                gate_combinator,
                credit_allowance,
                stop_on_quota_exhausted,
                max_attempts,
                deadline,
                created_by,
            } = *args;
            let criteria = parse_gate_criteria(&gate_criteria)?;
            let task = create_task(
                store,
                CreateSchedule {
                    name: &name,
                    instruction: &instruction,
                    target: &target,
                    workspace: &workspace,
                    fire_at: &fire_at,
                    repeat_interval_secs,
                    response_timeout_secs,
                    gate_criteria: criteria,
                    gate_combinator: &gate_combinator,
                    credit_allowance,
                    stop_on_quota_exhausted,
                    max_attempts,
                    deadline: deadline.as_deref(),
                    created_by: &created_by,
                },
            )?;
            if stop_on_quota_exhausted {
                eprintln!(
                    "note: --stop-on-quota-exhausted needs the in-app poller (S6); headless \
                     `ca schedule tick` only enforces the in-hub credit allowance, never live \
                     provider quota (S5c bridges that gap)"
                );
            }
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        ScheduleCommand::Get { id } => {
            let task = get_task(store, &id)?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        ScheduleCommand::Cancel { id } => {
            let task = cancel_task(store, &id)?;
            println!("{}", serde_json::to_string_pretty(&task)?);
        }
        ScheduleCommand::Tick { now } => {
            let summary = tick(store, now.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
            if quota_aware_open_task_exists(store)? {
                eprintln!(
                    "note: live provider-quota windows are NOT checked by headless tick — only \
                     the in-hub credit allowance is enforced here (S5c bridges that gap); \
                     quota-based pauses require the in-app poller (S6)"
                );
            }
            let pending = pending_proposal_count(store)?;
            if pending > 0 {
                eprintln!(
                    "note: {pending} os_tool proposal(s) pending approval — unmet gate criteria \
                     cannot pass until resolved; review with `ca tool pending`, approve with `ca \
                     tool approve`"
                );
            }
        }
    }
    Ok(())
}

/// Fields [`create_task`] needs, kept as a struct so validation stays in
/// one place and tests can drive it directly.
pub(crate) struct CreateSchedule<'a> {
    pub name: &'a str,
    pub instruction: &'a str,
    pub target: &'a str,
    pub workspace: &'a str,
    pub fire_at: &'a str,
    pub repeat_interval_secs: Option<i64>,
    pub response_timeout_secs: i64,
    pub gate_criteria: Vec<GateCriterion>,
    pub gate_combinator: &'a str,
    pub credit_allowance: Option<f64>,
    pub stop_on_quota_exhausted: bool,
    pub max_attempts: u32,
    pub deadline: Option<&'a str>,
    pub created_by: &'a str,
}

fn parse_target(raw: &str) -> anyhow::Result<ScheduleTarget> {
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("--target must not be empty (an agent id, or \"team\")");
    }
    if raw.eq_ignore_ascii_case("team") {
        Ok(ScheduleTarget::Team)
    } else {
        Ok(ScheduleTarget::Agent {
            agent_id: raw.to_string(),
        })
    }
}

fn parse_combinator(raw: &str) -> anyhow::Result<GateCombinator> {
    match raw {
        "all" => Ok(GateCombinator::All),
        "any" => Ok(GateCombinator::Any),
        other => bail!("--gate-combinator must be \"all\" or \"any\", got \"{other}\""),
    }
}

fn parse_gate_criteria(json: &str) -> anyhow::Result<Vec<GateCriterion>> {
    serde_json::from_str(json)
        .with_context(|| "--gate-criteria must be a JSON array of GateCriterion objects")
}

fn parse_rfc3339(flag: &str, value: &str) -> anyhow::Result<()> {
    DateTime::parse_from_rfc3339(value)
        .map(|_| ())
        .with_context(|| format!("{flag} must be an RFC3339 timestamp, got \"{value}\""))
}

fn validate_create(args: &CreateSchedule<'_>) -> anyhow::Result<()> {
    if args.name.trim().is_empty() {
        bail!("--name must not be empty");
    }
    if args.instruction.trim().is_empty() {
        bail!("--instruction must not be empty");
    }
    if !std::path::Path::new(args.workspace).is_absolute() {
        bail!(
            "--workspace must be an absolute path, got \"{}\"",
            args.workspace
        );
    }
    parse_rfc3339("--fire-at", args.fire_at)?;
    if let Some(deadline) = args.deadline {
        parse_rfc3339("--deadline", deadline)?;
    }
    if let Some(secs) = args.repeat_interval_secs {
        if secs <= 0 {
            bail!("--repeat-interval-secs must be positive, got {secs}");
        }
    }
    if args.response_timeout_secs <= 0 {
        bail!(
            "--response-timeout-secs must be positive, got {}",
            args.response_timeout_secs
        );
    }
    if let Some(allowance) = args.credit_allowance {
        if !allowance.is_finite() || allowance < 0.0 {
            bail!("--credit-allowance must be a non-negative finite number, got {allowance}");
        }
    }
    if args.max_attempts == 0 {
        bail!("--max-attempts must be at least 1 — it is a mandatory stop condition");
    }
    if args.created_by.trim().is_empty() {
        bail!("--created-by must not be empty");
    }
    Ok(())
}

pub(crate) fn list_tasks(
    store: &HubStore,
    status: Option<&str>,
) -> anyhow::Result<Vec<ScheduledTask>> {
    let tasks = store.list_scheduled_tasks()?;
    match status {
        None => Ok(tasks),
        Some(raw) => {
            let wanted = ScheduleStatus::parse(raw).ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown --status \"{raw}\" (expected one of: scheduled, active, paused, \
                     completed, cancelled, failed)"
                )
            })?;
            Ok(tasks.into_iter().filter(|t| t.status == wanted).collect())
        }
    }
}

pub(crate) fn create_task(
    store: &HubStore,
    args: CreateSchedule<'_>,
) -> anyhow::Result<ScheduledTask> {
    validate_create(&args)?;
    let target = parse_target(args.target)?;
    let combinator = parse_combinator(args.gate_combinator)?;
    let budget = BudgetPolicy {
        credit_allowance: args.credit_allowance,
        credit_spent: 0.0,
        stop_on_quota_exhausted: args.stop_on_quota_exhausted,
    };
    store
        .create_scheduled_task(
            args.name,
            args.instruction,
            target,
            args.workspace,
            args.fire_at,
            args.repeat_interval_secs,
            args.response_timeout_secs,
            args.gate_criteria,
            combinator,
            budget,
            args.max_attempts,
            args.deadline,
            args.created_by,
        )
        .map_err(Into::into)
}

pub(crate) fn get_task(store: &HubStore, id: &str) -> anyhow::Result<ScheduledTask> {
    store
        .get_scheduled_task(id)?
        .ok_or_else(|| anyhow::anyhow!("scheduled task not found: {id}"))
}

pub(crate) fn cancel_task(store: &HubStore, id: &str) -> anyhow::Result<ScheduledTask> {
    store.cancel_scheduled_task(id).map_err(Into::into)
}

/// Headless tick: defaults `now` to the real clock, validates an explicit
/// override, and reuses the hub's `TickSummary` verbatim as the printed
/// result — the CLI adds no dispatch, gate, or budget logic of its own.
pub(crate) fn tick(store: &HubStore, now: Option<&str>) -> anyhow::Result<TickSummary> {
    let now = match now {
        Some(value) => {
            parse_rfc3339("--now", value)?;
            value.to_string()
        }
        None => chrono::Utc::now().to_rfc3339(),
    };
    store.tick_scheduled_tasks(&now).map_err(Into::into)
}

/// Whether any non-terminal task asked for live-quota pauses — headless
/// tick cannot honor those, so the caller surfaces the S5c distinction.
fn quota_aware_open_task_exists(store: &HubStore) -> anyhow::Result<bool> {
    Ok(store
        .list_scheduled_tasks()?
        .into_iter()
        .any(|t| !t.status.is_terminal() && t.budget.stop_on_quota_exhausted))
}

/// Pending `os_tool.proposed` audit rows: unmet gate criteria depending on
/// a proposed command cannot pass until the owner approves via `ca tool`.
fn pending_proposal_count(store: &HubStore) -> anyhow::Result<usize> {
    Ok(store
        .list_audit_events(true)?
        .into_iter()
        .filter(|e| e.operation == "os_tool.proposed")
        .count())
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;

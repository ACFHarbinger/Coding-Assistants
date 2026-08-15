use crate::app::*;
use crate::helpers::{audit_file_hash, audit_operation, audit_process_context, default_home};
use hub::{HubStore, TaskStatus, WakeStatus, WorkflowStep};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

mod harness;
mod memory;
mod msg;
mod preflight;
mod shutdown;
mod tui_command;
pub(crate) fn run(cli: Cli) -> anyhow::Result<()> {
    let home = cli.home.clone().unwrap_or_else(default_home);
    let command = cli.command;
    if let Some(result) = preflight::run_if_requested(&command, home.clone()) {
        return result;
    }
    let store = HubStore::open(&home)?;

    match command {
        Command::Init => {
            println!("initialized hub at {}", store.data_dir().display());
        }
        Command::Agent { action } => crate::agent::run(&store, action)?,
        Command::Agents => {
            println!("{}", serde_json::to_string_pretty(&store.list_agents()?)?);
        }
        Command::ExportMarkdown {
            out,
            commit,
            message,
        } => {
            if commit {
                let outcome = store.export_markdown_git(out.as_deref(), message.as_deref())?;
                println!(
                    "exported to {} ({})",
                    outcome.path.display(),
                    if outcome.committed {
                        "committed"
                    } else {
                        &outcome.detail
                    }
                );
            } else {
                let path = store.export_markdown(out.as_deref())?;
                println!("exported to {}", path.display());
            }
        }
        Command::Memory { action } => memory::run(&store, action)?,
        Command::Msg { action } => msg::run(&store, action)?,
        Command::Wake { action } => match action {
            WakeCommand::Request {
                target,
                reason,
                message_id,
                human_gate,
            } => {
                let record = store.request_wake(
                    &target,
                    reason.as_deref(),
                    message_id.as_deref(),
                    human_gate,
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            WakeCommand::List {
                target,
                pending_only,
            } => {
                let records = store.list_wakes(target.as_deref(), pending_only)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            WakeCommand::Resolve { id, status } => {
                let status = match status.as_str() {
                    "delivered" => WakeStatus::Delivered,
                    "cancelled" => WakeStatus::Cancelled,
                    "pending" => WakeStatus::Pending,
                    other => anyhow::bail!("unknown wake status: {other}"),
                };
                store.set_wake_status(&id, status)?;
                println!("ok");
            }
            WakeCommand::Policy {
                set_default_gate,
                set_allow_auto,
            } => {
                let mut policy = store.get_wake_policy()?;
                if let Some(v) = set_default_gate {
                    policy.default_requires_human_gate = v;
                }
                if let Some(v) = set_allow_auto {
                    policy.allow_auto_wake = v;
                }
                if set_default_gate.is_some() || set_allow_auto.is_some() {
                    store.set_wake_policy(&policy)?;
                }
                println!("{}", serde_json::to_string_pretty(&policy)?);
            }
        },
        Command::Journal { action } => match action {
            JournalCommand::Append { agent, entry } => {
                let path = store.append_private_journal(&agent, &entry)?;
                println!("appended to {}", path.display());
            }
        },
        Command::Task { action } => match action {
            TaskCommand::Create {
                title,
                workspace,
                steps,
                max_parallel,
                require_approval,
            } => {
                let steps: Vec<WorkflowStep> = serde_json::from_str(&steps)
                    .map_err(|e| anyhow::anyhow!("--steps JSON: {e}"))?;
                let record = store.create_task_with_parallel(
                    &title,
                    workspace.as_deref(),
                    &steps,
                    max_parallel,
                    require_approval,
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::List { status } => {
                let status = status.map(|s| TaskStatus::parse(&s)).transpose()?;
                let records = store.list_tasks(status)?;
                println!("{}", serde_json::to_string_pretty(&records)?);
            }
            TaskCommand::Get { id } => {
                let record = store
                    .get_task(&id)?
                    .ok_or_else(|| anyhow::anyhow!("task not found: {id}"))?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::Advance { id, from, note } => {
                let record = store.advance_task(&id, from.as_deref(), note.as_deref())?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::Complete { id, agent, note } => {
                let record = store.complete_parallel_member(&id, &agent, note.as_deref())?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::Retry { id, from, note } => {
                let record = store.retry_task(&id, from.as_deref(), note.as_deref())?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            TaskCommand::Cancel { id } => {
                let record = store.cancel_task(&id)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
        },
        Command::Budget { action } => match action {
            BudgetCommand::Set { agent, limit } => {
                let status = store.set_agent_budget(&agent, limit)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            BudgetCommand::Status { agent } => {
                let status = store
                    .get_budget(&agent)?
                    .ok_or_else(|| anyhow::anyhow!("no budget set for {agent}"))?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            BudgetCommand::Spend { agent, amount } => {
                let status = store.record_budget_usage(&agent, amount)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            BudgetCommand::Consume { agent, amount } => {
                let status = store.try_consume_budget(&agent, amount)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            BudgetCommand::Pause {
                agent,
                task,
                objective,
                completed,
                missing,
                delegate_to,
            } => {
                let outcome = store.pause_for_budget(
                    &agent,
                    task.as_deref(),
                    &objective,
                    &completed,
                    &missing,
                    delegate_to.as_deref(),
                )?;
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            }
            BudgetCommand::Resume { agent } => {
                let status = store.resume_agent(&agent)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
        },
        Command::Audit { action } => match action {
            AuditCommand::Watch { root } => {
                let root = std::fs::canonicalize(&root)?;
                if !root.is_dir() {
                    anyhow::bail!("audit root is not a directory: {}", root.display());
                }
                let (sender, receiver) = std::sync::mpsc::channel();
                let mut watcher = RecommendedWatcher::new(sender, Config::default())?;
                watcher.watch(&root, RecursiveMode::Recursive)?;
                println!("watching {} (Ctrl-C to stop)", root.display());
                while let Ok(result) = receiver.recv() {
                    match result {
                        Ok(event) => {
                            let operation = audit_operation(&event.kind);
                            for path in event.paths {
                                let relative = path.strip_prefix(&root).unwrap_or(&path);
                                let process = audit_process_context();
                                let hash = audit_file_hash(&path);
                                let record = store.record_audit_event(
                                    &root,
                                    relative,
                                    operation,
                                    &process,
                                    hash.as_deref(),
                                )?;
                                println!("{} {} {}", record.id, record.operation, record.path);
                            }
                        }
                        Err(error) => eprintln!("audit watcher error: {error}"),
                    }
                }
            }
            AuditCommand::Pending => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&store.list_audit_events(true)?)?
                );
            }
            AuditCommand::List => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&store.list_audit_events(false)?)?
                );
            }
            AuditCommand::Approve { id, all } => {
                if all {
                    for event in store.list_audit_events(true)? {
                        store.set_audit_status(&event.id, "approved")?;
                    }
                } else {
                    let id = id.ok_or_else(|| anyhow::anyhow!("provide an event id or --all"))?;
                    store.set_audit_status(&id, "approved")?;
                }
                println!("approved");
            }
            AuditCommand::Quarantine { id } => {
                store.set_audit_status(&id, "quarantined")?;
                println!("quarantined");
            }
            AuditCommand::Verify => {
                let count = store.verify_audit_chain()?;
                println!("verified {count} audit events");
            }
        },
        Command::Inbox { action } => return crate::io::run(&store, action),
        Command::Harness { action } => harness::run(&store, action)?,
        Command::Shutdown {
            agent,
            task,
            objective,
            reason,
            delegate_to,
        } => shutdown::record(
            &store,
            &agent,
            task.as_deref(),
            &objective,
            &reason,
            delegate_to.as_deref(),
        )?,
        Command::Tui {
            workspace,
            session,
            set_as_default_workspace_settings,
            set_as_default_session_settings,
        } => tui_command::run(
            cli.home,
            workspace,
            session,
            set_as_default_workspace_settings,
            set_as_default_session_settings,
        )?,
        Command::Preflight { .. } => unreachable!("preflight returns before HubStore::open"),
    }
    Ok(())
}

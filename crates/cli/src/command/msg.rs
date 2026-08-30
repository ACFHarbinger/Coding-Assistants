//! ca msg subcommand dispatch (split from command/mod.rs for the
//! 500-LoC cap, #158).

use crate::app::MsgCommand;
use crate::helpers::{require_human_authored, tagged_dispatch_workspace};
use hub::{inject_harness_with_store, HarnessInjectRequest, HubStore, MessageKind, MessageStatus};

pub(super) fn run(store: &HubStore, action: MsgCommand) -> anyhow::Result<()> {
    match action {
        MsgCommand::Send {
            from,
            to,
            kind,
            subject,
            workspace,
            task,
            body,
        } => {
            let kind = MessageKind::parse(&kind)?;
            if kind.requires_tagged_send() {
                anyhow::bail!(
                        "kind {} must use `ca msg tag --wake` so enrollment and wake policy are recorded",
                        kind.as_str()
                    );
            }
            if to == "team" {
                let records = store.send_message_to_team(
                    &from,
                    kind,
                    &body,
                    subject.as_deref(),
                    workspace.as_deref(),
                    task.as_deref(),
                )?;
                println!("{}", serde_json::to_string_pretty(&records)?);
                return Ok(());
            }
            let record = store.send_message(
                &from,
                &to,
                kind,
                &body,
                subject.as_deref(),
                workspace.as_deref(),
                task.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        MsgCommand::Tag {
            from,
            to,
            task,
            wake,
            subject,
            workspace,
            task_id,
            session,
            dispatch,
            body,
        } => {
            let dispatch_workspace = tagged_dispatch_workspace(dispatch, workspace.as_deref())?;
            let outcomes = store.send_tagged_message(
                &from,
                &to,
                task,
                wake,
                &body,
                subject.as_deref(),
                workspace.as_deref(),
                task_id.as_deref(),
                session.as_deref(),
            )?;
            if let Some(workspace) = dispatch_workspace {
                // Settings are shared by every recipient in this dispatch.
                // Load and parse them once rather than once per accepted
                // recipient, which matters for a team-wide wake.
                let settings = hub::SettingsStore::open(hub::default_hub_home());
                let ws_str = workspace.to_string_lossy();
                for outcome in outcomes.iter().filter(|outcome| outcome.accepted) {
                    let (model, effort) = match settings
                        .effective_harness(Some(ws_str.as_ref()), &outcome.to_agent)
                    {
                        Ok(eff) => (eff.selected_model, eff.selected_effort),
                        Err(_) => (None, None),
                    };
                    let dispatch_result = inject_harness_with_store(
                        store,
                        &HarnessInjectRequest {
                            harness: outcome.to_agent.clone(),
                            workspace: workspace.clone(),
                            session_id: session.clone(),
                            message_id: outcome.message_id.clone(),
                            body: body.clone(),
                            is_task: task,
                            is_wake: wake,
                            model,
                            effort,
                        },
                    );
                    let dispatch_event = match dispatch_result {
                        Ok(result) => serde_json::json!({
                            "type": "harness_dispatch",
                            "target": outcome.to_agent,
                            "message_id": outcome.message_id,
                            "result": result,
                            "error": null,
                        }),
                        Err(error) => serde_json::json!({
                            "type": "harness_dispatch",
                            "target": outcome.to_agent,
                            "message_id": outcome.message_id,
                            "result": null,
                            "error": error.to_string(),
                        }),
                    };
                    eprintln!("{dispatch_event}");
                }
            }
            println!("{}", serde_json::to_string_pretty(&outcomes)?);
        }
        MsgCommand::Poll { to, no_ack } => {
            let records = store.poll_messages(&to, !no_ack)?;
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
        MsgCommand::List { to, status } => {
            let status = status.map(|s| MessageStatus::parse(&s)).transpose()?;
            let records = store.list_messages(to.as_deref(), status)?;
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
        MsgCommand::Channel { channel, limit } => {
            let records = store.list_channel_messages(&channel, limit)?;
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
        MsgCommand::Read { agent, scope, at } => {
            let marker = store.mark_read(&agent, &scope, at.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&marker)?);
        }
        MsgCommand::Readers { scope } => {
            let markers = store.list_read_markers(&scope)?;
            println!("{}", serde_json::to_string_pretty(&markers)?);
        }
        MsgCommand::Memories { message_id } => {
            let memories = store.list_message_memories(&message_id)?;
            println!("{}", serde_json::to_string_pretty(&memories)?);
        }
        MsgCommand::Status { id, status } => {
            let status = MessageStatus::parse(&status)?;
            let record = store.set_message_status(&id, status)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        MsgCommand::Edit { id, from, body } => {
            require_human_authored(store, &from, &id)?;
            let records = store.update_broadcast(&id, &body)?;
            println!("{}", serde_json::to_string_pretty(&records)?);
        }
        MsgCommand::Delete { id, from } => {
            require_human_authored(store, &from, &id)?;
            let count = store.delete_broadcast(&id)?;
            println!("{{\"deleted\": {count}}}");
        }
    }
    Ok(())
}

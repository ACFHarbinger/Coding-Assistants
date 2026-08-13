use crate::app::InboxCommand;
use hub::{HubStore, MessageStatus, WakeStatus};

pub(crate) fn run(store: &HubStore, action: InboxCommand) -> anyhow::Result<()> {
    match action {
        InboxCommand::Watch {
            agent,
            interval_ms,
            accept_gated,
            forward,
            forward_args,
        } => {
            if interval_ms == 0 {
                anyhow::bail!("--interval-ms must be greater than zero");
            }
            use std::io::Write;
            let mut forwarder = if let Some(program) = forward {
                let mut child = std::process::Command::new(program)
                    .args(forward_args)
                    .stdin(std::process::Stdio::piped())
                    .spawn()?;
                child.stdin.take()
            } else {
                if !forward_args.is_empty() {
                    anyhow::bail!("--forward-arg requires --forward");
                }
                None
            };
            let ready = serde_json::json!({
                "type": "ready",
                "agent": agent,
                "interval_ms": interval_ms,
                "accept_gated": accept_gated
            })
            .to_string();
            println!("{ready}");
            if let Some(stdin) = forwarder.as_mut() {
                writeln!(stdin, "{ready}")?;
                stdin.flush()?;
            }
            std::io::stdout().flush()?;
            loop {
                let pending_wakes = store.list_wakes(Some(&agent), true)?;
                let messages = store.list_messages(Some(&agent), Some(MessageStatus::Pending))?;
                let mut delivered_ids = Vec::new();
                for message in messages {
                    let gated = pending_wakes.iter().any(|wake| {
                        wake.message_id.as_deref() == Some(message.id.as_str())
                            && wake.requires_human_gate
                    });
                    if gated && !accept_gated {
                        // The durable message remains available, but the
                        // adapter must not cross the human gate silently.
                        continue;
                    }
                    let message = store.set_message_status(&message.id, MessageStatus::Acked)?;
                    delivered_ids.push(message.id.clone());
                    let line = serde_json::json!({
                        "type": "message",
                        "agent": agent,
                        "message": message
                    })
                    .to_string();
                    println!("{line}");
                    if let Some(stdin) = forwarder.as_mut() {
                        writeln!(stdin, "{line}")?;
                        stdin.flush()?;
                    }
                    std::io::stdout().flush()?;
                }
                for wake in pending_wakes {
                    if wake.requires_human_gate && !accept_gated {
                        continue;
                    }
                    if let Some(message_id) = &wake.message_id {
                        if !delivered_ids.iter().any(|id| id == message_id) {
                            continue;
                        }
                    }
                    store.set_wake_status(&wake.id, WakeStatus::Delivered)?;
                }
                std::thread::sleep(std::time::Duration::from_millis(interval_ms));
            }
        }
    }
}

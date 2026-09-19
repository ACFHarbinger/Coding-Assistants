//! Stream a spawned CLI child's stdout/stderr onto the hub event bus.

use crate::agent::AgentEvent;
use hub::bus::{InProcessBus, TOPIC_AGENT_EVENT};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::oneshot;
use tokio::time::Duration;

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.start_kill();
    }
}

pub(crate) async fn stream_cli_child(
    mut child: Child,
    bus: InProcessBus,
    source: &str,
    token: Option<Arc<AtomicBool>>,
    fail_label: &str,
) -> Result<String, String> {
    let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to open stderr")?;
    let mut child_guard = KillOnDrop(child);
    let bus_err = bus.clone();
    let source_clone = source.to_string();

    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line).await {
            if n == 0 {
                break;
            }
            bus_err.emit(
                TOPIC_AGENT_EVENT,
                AgentEvent {
                    source: source_clone.clone(),
                    event_type: "log".to_string(),
                    content: line.clone(),
                },
            );
            line.clear();
        }
    });

    let (cancel_tx, mut cancel_rx) = oneshot::channel();
    if let Some(token) = token {
        tokio::spawn(async move {
            loop {
                if token.load(Ordering::SeqCst) {
                    let _ = cancel_tx.send(());
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let mut full_output = String::new();

    loop {
        tokio::select! {
            result = reader.read_line(&mut line) => {
                 match result {
                     Ok(0) => break,
                     Ok(_) => {
                        bus.emit(TOPIC_AGENT_EVENT, AgentEvent {
                            source: source.to_string(),
                            event_type: "stream".to_string(),
                            content: line.clone(),
                        });
                        full_output.push_str(&line);
                        line.clear();
                     }
                     Err(e) => return Err(e.to_string()),
                 }
            }
            _ = &mut cancel_rx => {
                 return Err("Task cancelled".to_string());
            }
        }
    }

    let status = child_guard.0.wait().await.map_err(|e| e.to_string())?;
    if status.success() {
        Ok(full_output)
    } else {
        Err(format!("{fail_label} failed with status: {status}."))
    }
}

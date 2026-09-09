//! Pure MCP payload shaping: tool schemas and the `ToolResult` bodies for
//! `reply`/`check_inbox` tool calls. No I/O, no Hub connection — everything
//! here is a value-in, value-out function so it's exercised directly by
//! unit tests without spawning a real stdio server. `mcp-core` owns the
//! JSON-RPC envelope; this module only decides the text and the ok/err flag.

use hub::ChannelEvent;
use mcp_core::ToolResult;
use serde_json::{json, Value};

pub fn reply_tool_schema() -> Value {
    json!({
        "name": "reply",
        "description": "Send a reply back to the Coding-Assistants Hub, routed to the original sender or session that reached this Channel.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The reply body." },
                "in_reply_to": { "type": "string", "description": "Hub message id this replies to, if known." },
                "session_id": { "type": "string", "description": "Hub session id to reply within, if known." },
            },
            "required": ["text"],
        },
    })
}

pub fn check_inbox_tool_schema() -> Value {
    json!({
        "name": "check_inbox",
        "description": "Read and ack Hub chat traffic addressed to this session: any wake / task-tagged sends that were pushed as `notifications/claude/channel` (returned here too, so nothing is lost if this client doesn't surface that notification), followed by quieter plain messages and handoffs. Call this whenever you want to catch up; nothing is lost by not calling it, it just waits here.",
        "inputSchema": { "type": "object", "properties": {} },
    })
}

/// Renders drained quiet events as one line each, so Claude sees exactly
/// what a `notifications/claude/channel` push would have shown, just
/// pulled instead of pushed.
pub fn format_quiet_events(events: &[ChannelEvent]) -> String {
    if events.is_empty() {
        return "No new messages.".to_string();
    }
    events
        .iter()
        .map(|event| format!("[{}] {}: {}", event.kind, event.from_agent, event.body))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One line per disturb event (wake / task-tagged), tagged and carrying the
/// session id and message id so Claude can route a `reply` back correctly.
fn format_disturb_events(events: &[ChannelEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| {
            let tag = if event.kind == "wake" { "WAKE" } else { "TASK" };
            let session = event.session_id.as_deref().unwrap_or("-");
            format!(
                "⚡ [{tag}] {} (session {session}, msg {}): {}",
                event.from_agent, event.message_id, event.body
            )
        })
        .collect()
}

/// `disturb` are wake / task-tagged events already drained + acked from the
/// Hub by the background poll loop and buffered for this pull; `quiet` is
/// the fresh drain of plain traffic. Disturb events are shown first.
pub fn check_inbox_outcome(
    quiet: Result<Vec<ChannelEvent>, hub::HubError>,
    disturb: Vec<ChannelEvent>,
) -> ToolResult {
    let quiet = match quiet {
        Ok(events) => events,
        Err(error) => return ToolResult::Err(format!("failed to check inbox: {error}")),
    };
    if quiet.is_empty() && disturb.is_empty() {
        return ToolResult::Ok("No new messages.".to_string());
    }
    let mut lines = format_disturb_events(&disturb);
    if !quiet.is_empty() {
        lines.push(format_quiet_events(&quiet));
    }
    ToolResult::Ok(lines.join("\n"))
}

pub fn reply_outcome(result: Result<hub::MessageRecord, hub::HubError>) -> ToolResult {
    match result {
        Ok(message) => ToolResult::Ok(format!("relayed to Hub as message {}", message.id)),
        Err(error) => ToolResult::Err(format!("failed to relay reply: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(outcome: &ToolResult) -> (&str, bool) {
        match outcome {
            ToolResult::Ok(t) => (t.as_str(), false),
            ToolResult::Err(t) => (t.as_str(), true),
        }
    }

    #[test]
    fn reply_tool_schema_requires_text_only() {
        let schema = reply_tool_schema();
        assert_eq!(schema["inputSchema"]["required"], json!(["text"]));
    }

    #[test]
    fn check_inbox_tool_schema_takes_no_arguments() {
        let schema = check_inbox_tool_schema();
        assert_eq!(schema["name"], "check_inbox");
        assert_eq!(schema["inputSchema"]["properties"], json!({}));
    }

    fn sample_event(kind: &str, from: &str, body: &str) -> ChannelEvent {
        ChannelEvent {
            message_id: "msg-1".into(),
            from_agent: from.into(),
            session_id: None,
            kind: kind.into(),
            task_id: None,
            body: body.into(),
        }
    }

    #[test]
    fn format_quiet_events_reports_when_nothing_is_waiting() {
        assert_eq!(format_quiet_events(&[]), "No new messages.");
    }

    #[test]
    fn format_quiet_events_renders_one_line_per_message() {
        let events = vec![
            sample_event("message", "grok", "hey"),
            sample_event("handoff", "gemini", "handing this off"),
        ];
        assert_eq!(
            format_quiet_events(&events),
            "[message] grok: hey\n[handoff] gemini: handing this off"
        );
    }

    #[test]
    fn check_inbox_outcome_reports_success_and_failure_distinctly() {
        let ok = check_inbox_outcome(Ok(vec![sample_event("message", "grok", "hi")]), Vec::new());
        assert_eq!(text_of(&ok), ("[message] grok: hi", false));

        let empty = check_inbox_outcome(Ok(Vec::new()), Vec::new());
        assert_eq!(text_of(&empty), ("No new messages.", false));

        let err = check_inbox_outcome(Err(hub::HubError::Invalid("bad".into())), Vec::new());
        assert!(text_of(&err).1);
    }

    #[test]
    fn check_inbox_outcome_shows_buffered_disturb_events_first() {
        let mut task = sample_event("message", "human", "do the thing");
        task.session_id = Some("sess-9".into());
        task.message_id = "msg-task".into();
        task.task_id = Some("t-1".into());
        let outcome = check_inbox_outcome(
            Ok(vec![sample_event("message", "grok", "fyi")]),
            vec![task],
        );
        let (text, is_err) = text_of(&outcome);
        assert!(!is_err);
        assert_eq!(
            text,
            "⚡ [TASK] human (session sess-9, msg msg-task): do the thing\n[message] grok: fyi"
        );
    }

    #[test]
    fn reply_outcome_reports_success_and_failure_distinctly() {
        let ok = reply_outcome(Ok(hub::MessageRecord {
            id: "msg-1".into(),
            from_agent: "claude".into(),
            to_agent: "human".into(),
            workspace_path: None,
            task_id: None,
            kind: "message".into(),
            status: "pending".into(),
            subject: None,
            body: "hi".into(),
            created_at: "now".into(),
            acked_at: None,
        }));
        assert_eq!(text_of(&ok), ("relayed to Hub as message msg-1", false));

        let err = reply_outcome(Err(hub::HubError::Invalid("bad".into())));
        assert!(text_of(&err).1);
    }
}

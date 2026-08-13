//! Pure MCP payload shaping: tool schemas and the response bodies for
//! `reply`/`check_inbox` tool calls. No I/O, no Hub connection — everything
//! here is a value-in, `Value`-out function so it's exercised directly by
//! unit tests without spawning a real stdio server.

use hub::ChannelEvent;
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
        "description": "Read and ack quieter Hub chat traffic addressed to this session — plain messages and handoffs that were deliberately *not* pushed as an interruption (only wakes and task-tagged sends are pushed proactively). Call this whenever you want to catch up; nothing is lost by not calling it, it just waits here.",
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

pub fn check_inbox_response(id: Value, result: Result<Vec<ChannelEvent>, hub::HubError>) -> Value {
    match result {
        Ok(events) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "content": [{ "type": "text", "text": format_quiet_events(&events) }] },
        }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "content": [{ "type": "text", "text": format!("failed to check inbox: {error}") }], "isError": true },
        }),
    }
}

pub fn tool_call_response(id: Value, result: Result<hub::MessageRecord, hub::HubError>) -> Value {
    match result {
        Ok(message) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "content": [{ "type": "text", "text": format!("relayed to Hub as message {}", message.id) }] },
        }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "content": [{ "type": "text", "text": format!("failed to relay reply: {error}") }], "isError": true },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn check_inbox_response_reports_success_and_failure_distinctly() {
        let ok = check_inbox_response(json!(1), Ok(vec![sample_event("message", "grok", "hi")]));
        assert_eq!(ok["result"]["isError"], Value::Null);
        assert_eq!(ok["result"]["content"][0]["text"], "[message] grok: hi");

        let err = check_inbox_response(json!(2), Err(hub::HubError::Invalid("bad".into())));
        assert_eq!(err["result"]["isError"], json!(true));
    }

    #[test]
    fn tool_call_response_reports_success_and_failure_distinctly() {
        let ok = tool_call_response(
            json!(1),
            Ok(hub::MessageRecord {
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
            }),
        );
        assert_eq!(ok["result"]["isError"], Value::Null);

        let err = tool_call_response(json!(2), Err(hub::HubError::Invalid("bad".into())));
        assert_eq!(err["result"]["isError"], json!(true));
    }
}

//! Hub event helpers for direct HTTP completions.

use crate::agent::AgentEvent;
use crate::client::http::ChatResult;
use hub::bus::{InProcessBus, TOPIC_AGENT_EVENT};

pub fn emit_http_result(
    bus: &InProcessBus,
    source: &str,
    result: &ChatResult,
    emit_full_stream: bool,
) {
    if emit_full_stream && !result.text.is_empty() {
        bus.emit(
            TOPIC_AGENT_EVENT,
            AgentEvent {
                source: source.to_string(),
                event_type: "stream".to_string(),
                content: result.text.clone(),
            },
        );
    }
    if let Some(usage) = result.usage_json() {
        bus.emit(
            TOPIC_AGENT_EVENT,
            AgentEvent {
                source: source.to_string(),
                event_type: "usage".to_string(),
                content: usage.to_string(),
            },
        );
    }
    bus.emit(
        TOPIC_AGENT_EVENT,
        AgentEvent {
            source: source.to_string(),
            event_type: "response".to_string(),
            content: result.text.clone(),
        },
    );
}

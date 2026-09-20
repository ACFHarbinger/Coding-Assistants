//! Orchestrate turn for the Grok Bot consumer API (`platform.md` P15).

use crate::agent::AgentEvent;
use crate::client::events::emit_http_result;
use crate::client::http::{
    chat_stream, grok_bot_is_authenticated, grok_bot_unavailable_unauthenticated,
    resolve_grok_bot_key, GROK_BOT_DEFAULT_BASE, GROK_BOT_DEFAULT_MODEL,
    GROK_BOT_REQUEST_TIMEOUT_SECS,
};
use hub::bus::{InProcessBus, TOPIC_AGENT_EVENT};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub async fn grok_bot_completion(
    endpoint: Option<&str>,
    model: &str,
    prompt: &str,
    bus: &InProcessBus,
    source: &str,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    let api_key_secret = resolve_grok_bot_key();
    if !grok_bot_is_authenticated(api_key_secret.as_ref().map(|s| s.expose())) {
        return Err(grok_bot_unavailable_unauthenticated());
    }
    let api_key = api_key_secret
        .as_ref()
        .map(|secret| secret.expose())
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(grok_bot_unavailable_unauthenticated)?;
    let api_base = endpoint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(GROK_BOT_DEFAULT_BASE);
    let model = model.trim();
    let model = if model.is_empty() {
        GROK_BOT_DEFAULT_MODEL
    } else {
        model
    };
    let bus_delta = bus.clone();
    let source_delta = source.to_string();
    let result = chat_stream(
        api_base,
        api_key,
        model,
        prompt,
        std::time::Duration::from_secs(GROK_BOT_REQUEST_TIMEOUT_SECS),
        token,
        |delta| {
            bus_delta.emit(
                TOPIC_AGENT_EVENT,
                AgentEvent {
                    source: source_delta.clone(),
                    event_type: "stream".to_string(),
                    content: delta.to_string(),
                },
            );
        },
    )
    .await?;
    emit_http_result(bus, source, &result, false);
    Ok(result.text)
}

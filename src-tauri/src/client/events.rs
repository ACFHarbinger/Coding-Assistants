//! Hub event helpers for direct HTTP completions.

use crate::agent::AgentEvent;
use crate::client::http::ChatResult;
use hub::bus::{InProcessBus, TOPIC_AGENT_EVENT};
use hub::{HubStore, ProviderUsage, SliceStamp};

pub fn emit_http_result(
    bus: &InProcessBus,
    source: &str,
    result: &ChatResult,
    emit_full_stream: bool,
    provider: &str,
    via_gateway: Option<&str>,
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
    persist_provider_usage(source, provider, via_gateway, result);
}

fn persist_provider_usage(
    source: &str,
    provider: &str,
    via_gateway: Option<&str>,
    result: &ChatResult,
) {
    let (upstream, via) = match via_gateway {
        Some(gateway) => {
            ProviderUsage::gateway_route(gateway, result.model.as_deref().unwrap_or(""))
        }
        None => (provider.to_string(), None),
    };
    let usage = ProviderUsage {
        provider: upstream,
        actor_agent: source.to_string(),
        prompt_tokens: result.usage.as_ref().map(|usage| usage.prompt_tokens),
        completion_tokens: result.usage.as_ref().map(|usage| usage.completion_tokens),
        cache_tokens: None,
        cost: result.cost,
        via_gateway: via,
        model: result.model.clone(),
        stamp: SliceStamp::registered(None, None, Some(source.to_string())),
    };
    if !usage.has_measurement() {
        return;
    }
    let Ok(store) = HubStore::open(hub::default_hub_home()) else {
        return;
    };
    let _ = store.record_provider_usage(&usage);
}

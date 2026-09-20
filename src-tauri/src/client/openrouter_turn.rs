//! Orchestrate turn for the OpenRouter gateway (`platform.md` P13).

use crate::client::events::emit_http_result;
use crate::client::http::{
    chat_with_fallback, openrouter_is_authenticated, openrouter_unavailable_unauthenticated,
    split_fallback_models, OPENROUTER_DEFAULT_BASE, OPENROUTER_REQUEST_TIMEOUT_SECS,
};
use hub::bus::InProcessBus;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub async fn openrouter_completion(
    endpoint: Option<&str>,
    model: &str,
    prompt: &str,
    bus: &InProcessBus,
    source: &str,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    let api_key_secret = hub::secret::resolve("OPENROUTER_API_KEY");
    if !openrouter_is_authenticated(api_key_secret.as_ref().map(|s| s.expose())) {
        return Err(openrouter_unavailable_unauthenticated());
    }
    let api_key = api_key_secret
        .as_ref()
        .map(|secret| secret.expose())
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(openrouter_unavailable_unauthenticated)?;
    let api_base = endpoint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(OPENROUTER_DEFAULT_BASE);
    let models = split_fallback_models(model);
    let result = chat_with_fallback(
        api_base,
        api_key,
        &models,
        prompt,
        std::time::Duration::from_secs(OPENROUTER_REQUEST_TIMEOUT_SECS),
        token,
    )
    .await?;
    emit_http_result(bus, source, &result, true);
    Ok(result.text)
}

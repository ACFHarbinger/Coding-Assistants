//! OpenRouter model-routing gateway (`platform.md` P13 / #318).
//!
//! One `OPENROUTER_API_KEY` + base URL exposes many upstream models as
//! plain strings (no per-model adapter). Fallback chains try the next
//! model on retryable transport/API errors. Cost is read from OpenRouter's
//! `usage.cost` when present. Presence-only auth; the key travels only in
//! `Authorization`.

use super::complete::{normalize_api_base, ChatResult, TokenUsage};
use super::error::DirectHttpError;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const OPENROUTER_DEFAULT_BASE: &str = "https://openrouter.ai/api/v1";
pub const OPENROUTER_DEFAULT_MODEL: &str = "openai/gpt-4o-mini";
pub const OPENROUTER_FALLBACK_MODELS: &[&str] = &[
    "openai/gpt-4o-mini",
    "openai/gpt-4o",
    "anthropic/claude-3.5-sonnet",
];
pub const OPENROUTER_REQUEST_TIMEOUT_SECS: u64 = 120;

const APP_REFERER: &str = "https://github.com/ACFHarbinger/Coding-Assistants";
const APP_TITLE: &str = "Coding Assistants";

pub fn is_openrouter_provider(provider: &str) -> bool {
    matches!(provider.trim(), "openrouter")
}

pub fn openrouter_is_authenticated(api_key: Option<&str>) -> bool {
    api_key
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

pub fn openrouter_unavailable_unauthenticated() -> String {
    "OpenRouter unavailable: not authenticated. Add OPENROUTER_API_KEY in Settings → Credentials or export it in the environment.".into()
}

/// Comma-separated model ids, first wins. Empty tokens dropped.
pub fn split_fallback_models(model: &str) -> Vec<String> {
    let models: Vec<String> = model
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if models.is_empty() {
        vec![OPENROUTER_DEFAULT_MODEL.to_string()]
    } else {
        models
    }
}

pub fn usage_from_openrouter_json(usage: &Value) -> Option<(TokenUsage, Option<f64>)> {
    let prompt_tokens = usage.get("prompt_tokens")?.as_u64()? as u32;
    let completion_tokens = usage.get("completion_tokens")?.as_u64()? as u32;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .map(|n| n as u32)
        .unwrap_or(prompt_tokens.saturating_add(completion_tokens));
    let cost = usage.get("cost").and_then(Value::as_f64);
    Some((
        TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        },
        cost,
    ))
}

fn assistant_text(body: &Value) -> Result<String, DirectHttpError> {
    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| DirectHttpError::InvalidResponse {
            message: "response had no choices[0].message.content text".into(),
        })
}

fn map_reqwest(error: reqwest::Error) -> DirectHttpError {
    if error.is_timeout() {
        DirectHttpError::Timeout
    } else {
        DirectHttpError::Transport {
            message: error.to_string(),
        }
    }
}

pub fn openrouter_chat_url(api_base: &str) -> String {
    format!("{}/chat/completions", normalize_api_base(api_base))
}

/// One non-streaming OpenRouter turn. Cost is taken from `usage.cost`.
pub async fn openrouter_chat_to(
    url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<ChatResult, DirectHttpError> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(map_reqwest)?;
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .header("HTTP-Referer", APP_REFERER)
        .header("X-Title", APP_TITLE)
        .json(&serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": prompt }],
            "stream": false
        }))
        .send()
        .await
        .map_err(map_reqwest)?;
    let status = response.status();
    let body = response.text().await.map_err(map_reqwest)?;
    if !status.is_success() {
        let message = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .unwrap_or(body);
        return Err(DirectHttpError::Api { message });
    }
    let payload: Value =
        serde_json::from_str(&body).map_err(|error| DirectHttpError::InvalidResponse {
            message: error.to_string(),
        })?;
    let (usage, cost) = payload
        .get("usage")
        .and_then(usage_from_openrouter_json)
        .map(|(usage, cost)| (Some(usage), cost))
        .unwrap_or((None, None));
    Ok(ChatResult {
        text: assistant_text(&payload)?,
        usage,
        cost,
    })
}

/// Try each model in order until one succeeds. Auth/cancel stop the chain.
pub async fn chat_with_fallback(
    api_base: &str,
    api_key: &str,
    models: &[String],
    prompt: &str,
    timeout: Duration,
    token: Option<Arc<AtomicBool>>,
) -> Result<ChatResult, DirectHttpError> {
    if token
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::SeqCst))
    {
        return Err(DirectHttpError::Cancelled);
    }
    let url = openrouter_chat_url(api_base);
    let mut last_error: Option<DirectHttpError> = None;
    for model in models {
        if token
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            return Err(DirectHttpError::Cancelled);
        }
        match openrouter_chat_to(&url, api_key, model, prompt, timeout).await {
            Ok(result) => return Ok(result),
            Err(error) if error.is_retryable() => {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(
        last_error.unwrap_or_else(|| DirectHttpError::InvalidResponse {
            message: "OpenRouter fallback chain was empty".into(),
        }),
    )
}

#[cfg(test)]
#[path = "gateway_tests.rs"]
mod tests;

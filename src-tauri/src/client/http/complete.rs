//! OpenAI-compatible chat completions via `async-openai` (`platform.md` P4).
//!
//! Transport-only: callers check auth presence and pass the key at call
//! time. The key is never logged. Offline hosts fail at connect; hung hosts
//! die on `timeout`. Rate-limit retries stay inside `async-openai` (429
//! only); connect/timeout errors are permanent.

use super::error::{map_openai_error, DirectHttpError};
use async_openai::config::OpenAIConfig;
#[cfg(test)]
use async_openai::types::CreateChatCompletionResponse;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs,
    ChatCompletionStreamOptions, CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Documented OpenAI Chat Completions base (`async-openai` default).
pub const OPENAI_DEFAULT_BASE: &str = "https://api.openai.com/v1";

/// Default model for Orchestrate roles (`rolesConfig.ts` Planner/Reviewer).
pub const OPENAI_DEFAULT_MODEL: &str = "gpt-4o";

/// `get_available_models` entries when `OPENAI_API_KEY` is present. The
/// API listing endpoint is a metered call, so presence gates a fixed set
/// matching the in-app role defaults.
pub const OPENAI_FALLBACK_MODELS: &[&str] = &["gpt-4o", "gpt-4o-mini"];

/// Request timeout for a chat completion. Agent turns can run long;
/// offline hosts fail fast at connect instead.
pub const OPENAI_REQUEST_TIMEOUT_SECS: u64 = 120;

/// Per-request token usage from a chat completion (stream or not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn from_openai(usage: &async_openai::types::CompletionUsage) -> Self {
        Self {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "prompt_tokens": self.prompt_tokens,
            "completion_tokens": self.completion_tokens,
            "total_tokens": self.total_tokens,
        })
    }

    /// Include optional gateway cost (USD / credits) without losing tokens.
    pub fn to_json_with_cost(&self, cost: Option<f64>) -> serde_json::Value {
        let mut value = self.to_json();
        if let Some(cost) = cost {
            value["cost"] = serde_json::json!(cost);
        }
        value
    }
}

/// Assistant text plus optional usage from one chat turn.
/// `cost` is OpenRouter/gateway credits when the upstream reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatResult {
    pub text: String,
    pub usage: Option<TokenUsage>,
    pub cost: Option<f64>,
}

impl ChatResult {
    pub fn usage_json(&self) -> Option<serde_json::Value> {
        self.usage
            .as_ref()
            .map(|usage| usage.to_json_with_cost(self.cost))
    }
}

/// Strip a trailing slash and ensure the base ends with `/v1`, matching
/// the existing `ModelConfig.endpoint` attach path.
pub fn normalize_api_base(endpoint: &str) -> String {
    let base = endpoint.trim().trim_end_matches('/');
    if base.is_empty() {
        return OPENAI_DEFAULT_BASE.to_string();
    }
    if base.ends_with("/v1") {
        base.to_string()
    } else {
        format!("{base}/v1")
    }
}

fn build_client(
    api_base: &str,
    api_key: &str,
    timeout: Duration,
) -> Result<Client<OpenAIConfig>, DirectHttpError> {
    let http = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| DirectHttpError::Transport {
            message: error.to_string(),
        })?;
    let config = OpenAIConfig::new()
        .with_api_base(normalize_api_base(api_base))
        .with_api_key(api_key);
    Ok(Client::with_config(config).with_http_client(http))
}

fn build_request(
    model: &str,
    prompt: &str,
    stream: bool,
) -> Result<CreateChatCompletionRequest, DirectHttpError> {
    let user = ChatCompletionRequestUserMessageArgs::default()
        .content(prompt)
        .build()
        .map_err(map_openai_error)?;
    let mut args = CreateChatCompletionRequestArgs::default();
    args.model(model)
        .messages(vec![ChatCompletionRequestMessage::User(user)]);
    if stream {
        args.stream_options(ChatCompletionStreamOptions {
            include_usage: true,
        });
    }
    args.build().map_err(map_openai_error)
}

#[cfg(test)]
fn text_from_response(response: &CreateChatCompletionResponse) -> Result<String, DirectHttpError> {
    response
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .filter(|text| !text.is_empty())
        .ok_or_else(|| DirectHttpError::InvalidResponse {
            message: "response had no choices[0].message.content text".into(),
        })
}

/// Non-streaming chat completion. Exercises the JSON body/`usage` path
/// without SSE (the live Orchestrate path streams).
#[cfg(test)]
pub async fn chat(
    api_base: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<ChatResult, DirectHttpError> {
    let client = build_client(api_base, api_key, timeout)?;
    let request = build_request(model, prompt, false)?;
    let response = client
        .chat()
        .create(request)
        .await
        .map_err(map_openai_error)?;
    Ok(ChatResult {
        text: text_from_response(&response)?,
        usage: response.usage.as_ref().map(TokenUsage::from_openai),
        cost: None,
    })
}

/// Streaming chat completion. `on_delta` receives each assistant text
/// fragment. Usage, when the server honors `stream_options.include_usage`,
/// is returned on the final result.
pub async fn chat_stream<F>(
    api_base: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    timeout: Duration,
    token: Option<Arc<AtomicBool>>,
    mut on_delta: F,
) -> Result<ChatResult, DirectHttpError>
where
    F: FnMut(&str),
{
    if token
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::SeqCst))
    {
        return Err(DirectHttpError::Cancelled);
    }
    let client = build_client(api_base, api_key, timeout)?;
    let request = build_request(model, prompt, true)?;
    let mut stream = client
        .chat()
        .create_stream(request)
        .await
        .map_err(map_openai_error)?;
    let mut text = String::new();
    let mut usage = None;
    while let Some(chunk) = stream.next().await {
        if token
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            return Err(DirectHttpError::Cancelled);
        }
        let chunk = chunk.map_err(map_openai_error)?;
        if let Some(reported) = chunk.usage.as_ref() {
            usage = Some(TokenUsage::from_openai(reported));
        }
        for choice in &chunk.choices {
            if let Some(delta) = choice.delta.content.as_deref() {
                if !delta.is_empty() {
                    on_delta(delta);
                    text.push_str(delta);
                }
            }
        }
    }
    if text.is_empty() {
        return Err(DirectHttpError::InvalidResponse {
            message: "stream produced no assistant text".into(),
        });
    }
    Ok(ChatResult {
        text,
        usage,
        cost: None,
    })
}

#[cfg(test)]
#[path = "complete_tests.rs"]
mod tests;

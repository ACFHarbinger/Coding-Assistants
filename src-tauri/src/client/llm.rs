use crate::agent::AgentEvent;
use crate::client::providers::{
    canonical_rate_limit_key, deepseek_unavailable_opencode, is_muse_provider, muse_chat_request,
    muse_is_authenticated, muse_unavailable_unauthenticated, opencode_run_args,
    parse_opencode_models, vibe_home_from_env, vibe_is_authenticated, vibe_programmatic_supported,
    vibe_run_args, vibe_unavailable_not_installed, vibe_unavailable_unauthenticated,
    vibe_unavailable_unsupported, MUSE_DEFAULT_MODEL, MUSE_FALLBACK_MODELS, VIBE_FALLBACK_MODELS,
};
use crate::client::stream::stream_cli_child;
use governor::clock::{Clock, DefaultClock};
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use hub::bus::{InProcessBus, TOPIC_AGENT_EVENT};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};
use tokio::process::Command;

/// Per-provider token-bucket rate limiter for outbound LLM calls.
///
/// Prevents a runaway agent retry loop from hammering a provider's API (or a
/// local CLI's process-spawn overhead) faster than it can reasonably handle.
/// Burst of 3, sustained 1/sec per provider — generous enough for normal
/// interactive use, tight enough to catch a tight retry loop.
type ProviderLimiter = RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>;

static PROVIDER_LIMITER: OnceLock<ProviderLimiter> = OnceLock::new();

fn provider_limiter() -> &'static ProviderLimiter {
    PROVIDER_LIMITER.get_or_init(|| {
        let quota =
            Quota::per_second(NonZeroU32::new(1).unwrap()).allow_burst(NonZeroU32::new(3).unwrap());
        RateLimiter::keyed(quota)
    })
}

/// Blocks until the given provider is under its rate limit.
async fn wait_for_rate_limit(provider: &str) {
    let limiter = provider_limiter();
    let key = provider.to_string();
    loop {
        match limiter.check_key(&key) {
            Ok(_) => return,
            Err(not_until) => {
                let wait = not_until.wait_time_from(DefaultClock::default().now());
                tokio::time::sleep(wait).await;
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    /// Optional OpenAI-compatible endpoint for an already-running model
    /// service. When set, the app sends requests to that service and never
    /// spawns or terminates a child process for this role.
    pub endpoint: Option<String>,
    pub prompt_file: Option<String>,
    pub rule_file: Option<String>,
    pub workflow_file: Option<String>,
    #[serde(default)]
    pub skill_file: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "opencode".to_string(),
            model: "big-pickle".to_string(),
            endpoint: None,
            prompt_file: None,
            rule_file: None,
            workflow_file: None,
            skill_file: None,
        }
    }
}

fn is_vibe_provider(provider: &str) -> bool {
    matches!(provider, "mistral" | "vibe")
}

pub struct LLMClient;

impl LLMClient {
    pub fn new() -> Self {
        Self
    }

    // TODO(RD2): this argument list should collapse once request state moves
    // into a dedicated struct as part of the actor-model daemon migration.
    #[allow(clippy::too_many_arguments)]
    pub async fn chat_completion(
        &self,
        config: &ModelConfig,
        prompt: &str,
        work_dir: Option<&str>,
        bus: &InProcessBus,
        source: &str,
        mcp_config_path: Option<&str>,
        token: Option<Arc<AtomicBool>>,
    ) -> Result<String, String> {
        // Aliases of one upstream share a rate-limit bucket (`muse`/`meta`
        // both hit the Model API); every other provider keys on itself.
        wait_for_rate_limit(canonical_rate_limit_key(&config.provider)).await;

        if let Some(endpoint) = config
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return existing_endpoint_completion(endpoint, config, prompt, bus, source).await;
        }

        if config.provider == "ollama" {
            let mut command = Command::new("ollama");
            command
                .arg("run")
                .arg(&config.model)
                .arg(prompt)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            if let Some(dir) = work_dir {
                command.current_dir(dir);
            }
            let child = command
                .spawn()
                .map_err(|e| format!("Failed to spawn ollama: {}", e))?;
            return stream_cli_child(child, bus.clone(), source, token, "Ollama").await;
        }

        if config.provider == "lm_studio" {
            return Err(
                "LM Studio support is partially implemented. Please ensure it is running on 127.0.0.1:1234"
                    .to_string(),
            );
        }

        if is_vibe_provider(&config.provider) {
            return vibe_completion(config, prompt, work_dir, bus, source, token).await;
        }

        if is_muse_provider(&config.provider) {
            return muse_completion(config, prompt, work_dir, bus, source, token).await;
        }

        opencode_completion(
            config,
            prompt,
            work_dir,
            bus,
            source,
            mcp_config_path,
            token,
        )
        .await
    }

    pub async fn list_models(&self) -> Result<Vec<String>, String> {
        let mut models = Vec::new();

        if let Ok(output) = Command::new("opencode").arg("models").output().await {
            if output.status.success() {
                if let Ok(content) = String::from_utf8(output.stdout) {
                    models.extend(parse_opencode_models(&content));
                }
            }
        }

        if let Ok(output) = Command::new("ollama").arg("list").output().await {
            if output.status.success() {
                if let Ok(content) = String::from_utf8(output.stdout) {
                    for line in content.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if let Some(name) = parts.first() {
                            models.push(format!("ollama/{}", name));
                        }
                    }
                }
            }
        }

        if let Ok(output) = Command::new("vibe").arg("--help").output().await {
            if output.status.success() {
                let help = String::from_utf8_lossy(&output.stdout);
                let help_err = String::from_utf8_lossy(&output.stderr);
                if vibe_programmatic_supported(&help) || vibe_programmatic_supported(&help_err) {
                    for model in VIBE_FALLBACK_MODELS {
                        models.push(format!("mistral/{model}"));
                    }
                }
            }
        }

        // The Model API publishes no listing endpoint: an authenticated
        // install contributes the fixed cookbook default, mirroring the
        // Mistral fallback pattern above.
        if muse_is_authenticated(
            hub::secret::resolve("MODEL_API_KEY")
                .as_ref()
                .map(|s| s.expose()),
        ) {
            for model in MUSE_FALLBACK_MODELS {
                models.push(format!("muse/{model}"));
            }
        }

        Ok(models)
    }
}

async fn existing_endpoint_completion(
    endpoint: &str,
    config: &ModelConfig,
    prompt: &str,
    bus: &InProcessBus,
    source: &str,
) -> Result<String, String> {
    let base = endpoint.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    };
    let response = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({
            "model": config.model,
            "messages": [{ "role": "user", "content": prompt }],
            "stream": false
        }))
        .send()
        .await
        .map_err(|e| format!("Existing model process request failed: {e}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Existing model process response read failed: {e}"))?;
    if !status.is_success() {
        return Err(format!("Existing model process returned {status}: {body}"));
    }
    let payload: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Existing model process returned invalid JSON: {e}"))?;
    let output = payload["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Existing model process response had no choices[0].message.content")?
        .to_string();
    bus.emit(
        TOPIC_AGENT_EVENT,
        AgentEvent {
            source: source.to_string(),
            event_type: "response".to_string(),
            content: output.clone(),
        },
    );
    Ok(output)
}

/// Muse Spark chat completion against the Meta Model API (OpenAI-compatible,
/// per the official cookbook). The key comes from `MODEL_API_KEY` at call
/// time and travels only in the `Authorization` header — it is never
/// stored, logged, or placed in argv. An empty configured model falls back
/// to the cookbook default. `work_dir`/`token` are dispatch-parity
/// parameters the workspace-free API ignores.
async fn muse_completion(
    config: &ModelConfig,
    prompt: &str,
    _work_dir: Option<&str>,
    bus: &InProcessBus,
    source: &str,
    _token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    // Resolved from the vault (keychain / file) or env as fallback via
    // hub::secret::resolve — never stored, logged, or placed in argv. The
    // key travels only in the outbound `Authorization` header.
    let api_key_secret = hub::secret::resolve("MODEL_API_KEY");
    if !muse_is_authenticated(api_key_secret.as_ref().map(|s| s.expose())) {
        return Err(muse_unavailable_unauthenticated());
    }
    // Keep the key in SecretString until reqwest constructs its Authorization
    // header; do not create an ordinary String that survives the request.
    let api_key = api_key_secret
        .as_ref()
        .map(|secret| secret.expose())
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(muse_unavailable_unauthenticated)?;
    let model = config.model.trim();
    let model = if model.is_empty() {
        MUSE_DEFAULT_MODEL
    } else {
        model
    };
    let output = muse_chat_request(api_key, model, prompt).await?;
    bus.emit(
        TOPIC_AGENT_EVENT,
        AgentEvent {
            source: source.to_string(),
            event_type: "response".to_string(),
            content: output.clone(),
        },
    );
    Ok(output)
}

async fn vibe_completion(
    config: &ModelConfig,
    prompt: &str,
    work_dir: Option<&str>,
    bus: &InProcessBus,
    source: &str,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    let help = Command::new("vibe").arg("--help").output().await;
    let help = match help {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            format!("{stdout}{stderr}")
        }
        Ok(output) => {
            return Err(vibe_unavailable_not_installed(format!(
                "vibe --help exited {}",
                output.status
            )));
        }
        Err(error) => return Err(vibe_unavailable_not_installed(error)),
    };
    if !vibe_programmatic_supported(&help) {
        return Err(vibe_unavailable_unsupported());
    }

    let home = vibe_home_from_env(
        std::env::var("VIBE_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    );
    // Resolved from the vault or env fallback via hub::secret::resolve —
    // used for presence only; the actual key is managed by the vibe CLI.
    let env_key_secret = hub::secret::resolve("MISTRAL_API_KEY");
    let env_key = env_key_secret.as_ref().map(|secret| secret.expose());
    if !vibe_is_authenticated(&config.model, &home, env_key) {
        return Err(vibe_unavailable_unauthenticated());
    }

    let args = vibe_run_args(prompt, work_dir)?;
    let mut command = Command::new("vibe");
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A key supplied through the vault has to reach Vibe's child process;
    // otherwise vault-only authentication would pass the presence check but
    // the CLI itself would run unauthenticated.
    if let Some(key) = env_key {
        command.env("MISTRAL_API_KEY", key);
    }
    if !config.model.trim().is_empty() {
        command.env("VIBE_ACTIVE_MODEL", &config.model);
    }
    if let Some(dir) = work_dir {
        command.current_dir(dir);
    }
    let child = command.spawn().map_err(vibe_unavailable_not_installed)?;
    stream_cli_child(child, bus.clone(), source, token, "Mistral Vibe").await
}

async fn opencode_completion(
    config: &ModelConfig,
    prompt: &str,
    work_dir: Option<&str>,
    bus: &InProcessBus,
    source: &str,
    mcp_config_path: Option<&str>,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    let args = opencode_run_args(&config.provider, &config.model, prompt, work_dir)?;
    let mut command = Command::new("opencode");
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(dir) = work_dir {
        command.current_dir(dir);
        if let Some(mcp_file) = mcp_config_path {
            let mcp_path = Path::new(mcp_file);
            let full_mcp_path = if mcp_path.is_absolute() {
                mcp_path.to_path_buf()
            } else {
                Path::new(dir).join(mcp_file)
            };
            command.env("MCP_CONFIG_FILE", full_mcp_path);
        }
    }

    let child = command.spawn().map_err(|error| {
        if config.provider == "deepseek" {
            deepseek_unavailable_opencode(error)
        } else {
            format!("Failed to spawn opencode: {error}")
        }
    })?;
    let label = if config.provider == "deepseek" {
        "DeepSeek (OpenCode)"
    } else {
        "Opencode"
    };
    stream_cli_child(child, bus.clone(), source, token, label).await
}

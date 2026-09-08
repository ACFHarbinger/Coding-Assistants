//! OpenCode (DeepSeek) and Mistral Vibe argv, model listing, and availability.
//!
//! These helpers never read secret files and never concatenate a shell string.
//! Callers pass the returned argv to `Command::args`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Public Vibe aliases from the installed CLI's default model table
/// (`mistral-medium-3.5`, `devstral-small`, `local`). Not API keys.
pub const VIBE_FALLBACK_MODELS: &[&str] = &["mistral-medium-3.5", "devstral-small", "local"];

const VIBE_HELP_FLAGS: &[&str] = &["-p", "--workdir", "--output", "--trust"];

pub fn opencode_model_spec(provider: &str, model: &str) -> String {
    format!("{}/{}", provider.trim(), model.trim())
}

/// `opencode run <prompt> -m provider/model [--dir <abs>]`
pub fn opencode_run_args(
    provider: &str,
    model: &str,
    prompt: &str,
    work_dir: Option<&str>,
) -> Result<Vec<OsString>, String> {
    if prompt.trim().is_empty() {
        return Err("OpenCode run requires a prompt".into());
    }
    if provider.trim().is_empty() || model.trim().is_empty() {
        return Err("OpenCode run requires provider/model".into());
    }
    let mut args = vec![
        OsString::from("run"),
        OsString::from(prompt),
        OsString::from("-m"),
        OsString::from(opencode_model_spec(provider, model)),
    ];
    if let Some(dir) = work_dir.map(str::trim).filter(|value| !value.is_empty()) {
        if !Path::new(dir).is_absolute() {
            return Err("OpenCode --dir must be an absolute path".into());
        }
        args.push(OsString::from("--dir"));
        args.push(OsString::from(dir));
    }
    Ok(args)
}

/// Lines from `opencode models`, preserving provider/model form.
pub fn parse_opencode_models(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
fn deepseek_models_from_opencode(output: &str) -> Vec<String> {
    parse_opencode_models(output)
        .into_iter()
        .filter_map(|line| {
            line.strip_prefix("deepseek/")
                .map(str::to_string)
                .filter(|model| !model.is_empty())
        })
        .collect()
}

pub fn vibe_programmatic_supported(help: &str) -> bool {
    VIBE_HELP_FLAGS
        .iter()
        .all(|flag| help.split_whitespace().any(|token| token == *flag) || help.contains(flag))
}

/// Auth presence only: non-empty `MISTRAL_API_KEY`, or a `~/.vibe/.env` file.
/// Does not read file contents or the keyring. `local` (llama.cpp) needs no key.
pub fn vibe_is_authenticated(model: &str, vibe_home: &Path, mistral_api_key: Option<&str>) -> bool {
    if model.trim() == "local" {
        return true;
    }
    if mistral_api_key
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return true;
    }
    vibe_home.join(".env").is_file()
}

pub fn vibe_home_from_env(vibe_home: Option<&str>, user_home: Option<&str>) -> PathBuf {
    if let Some(dir) = vibe_home.map(str::trim).filter(|value| !value.is_empty()) {
        return PathBuf::from(dir);
    }
    PathBuf::from(user_home.unwrap_or(".")).join(".vibe")
}

/// `vibe -p <prompt> --workdir <abs> --trust --output text --auto-approve`
pub fn vibe_run_args(prompt: &str, work_dir: Option<&str>) -> Result<Vec<OsString>, String> {
    if prompt.trim().is_empty() {
        return Err("Mistral Vibe run requires a prompt".into());
    }
    let mut args = vec![
        OsString::from("-p"),
        OsString::from(prompt),
        OsString::from("--trust"),
        OsString::from("--output"),
        OsString::from("text"),
        OsString::from("--auto-approve"),
    ];
    if let Some(dir) = work_dir.map(str::trim).filter(|value| !value.is_empty()) {
        if !Path::new(dir).is_absolute() {
            return Err("Mistral Vibe --workdir must be an absolute path".into());
        }
        args.push(OsString::from("--workdir"));
        args.push(OsString::from(dir));
    }
    Ok(args)
}

pub fn vibe_unavailable_not_installed(error: impl std::fmt::Display) -> String {
    format!(
        "Mistral (vibe) unavailable: vibe CLI is not installed or not on PATH ({error}). Install Mistral Vibe and retry."
    )
}

pub fn vibe_unavailable_unsupported() -> String {
    "Mistral (vibe) unavailable: this vibe build does not advertise programmatic -p/--workdir/--output/--trust. Upgrade vibe and retry.".into()
}

pub fn vibe_unavailable_unauthenticated() -> String {
    "Mistral (vibe) unavailable: not authenticated. Run `vibe --setup` to configure an API key (no key is stored by Coding Assistants).".into()
}

pub fn deepseek_unavailable_opencode(error: impl std::fmt::Display) -> String {
    format!(
        "DeepSeek (OpenCode) unavailable: opencode CLI is not installed or failed to start ({error}). Install OpenCode, confirm `opencode models` lists deepseek/*, and retry."
    )
}

/// Meta Muse Spark model provider (#274), served by the Meta Model API —
/// **not** by shelling out to the `muse` CLI (that CLI is the #273 managed
/// harness; conflating the two is exactly what `HarnessId`'s rejected bare
/// "meta" alias guards against).
///
/// Contract verified against the official `meta-models/meta-model-cookbook`:
/// OpenAI-compatible chat completions at `{base}/chat/completions` with a
/// `Bearer` `MODEL_API_KEY` (`LLM|…` format, resolved from the vault or env,
/// never logged), model
/// `muse-spark-1.3`. `ModelConfig.provider` accepts `muse` (app-facing key)
/// or `meta` (the CLI's own provider id); the namespaces are separate.
pub const MUSE_MODEL_API_BASE_URL: &str = "https://api.meta.ai/v1";

/// Default Muse Spark model id (cookbook default, 1M-token context).
pub const MUSE_DEFAULT_MODEL: &str = "muse-spark-1.3";

/// `get_available_models` entries for an authenticated install, mirroring
/// the `VIBE_FALLBACK_MODELS` pattern (the Model API publishes no
/// listing endpoint, so presence gates a fixed entry).
pub const MUSE_FALLBACK_MODELS: &[&str] = &["muse-spark-1.3"];

/// Request timeout for a Model API chat completion. Agent turns over a
/// 1M-context model can take a while; offline hosts fail fast at connect
/// instead (mapped to `unavailable`, never retried — matching the rest of
/// this module, and the cookbook's "don't retry what cannot succeed"
/// guidance for transport errors).
pub const MUSE_REQUEST_TIMEOUT_SECS: u64 = 120;

pub fn is_muse_provider(provider: &str) -> bool {
    matches!(provider.trim(), "muse" | "meta")
}

/// Canonical rate-limit bucket key: `muse` and `meta` are aliases for one
/// upstream (the Meta Model API), so they must share a bucket rather than
/// each getting a full quota. Every other provider keys on its own string,
/// untouched.
pub fn canonical_rate_limit_key(provider: &str) -> &str {
    if is_muse_provider(provider) {
        "muse"
    } else {
        provider
    }
}

/// Auth presence only: a non-empty `MODEL_API_KEY`. Never reads key
/// material beyond presence — the key travels only in the request's
/// `Authorization` header, built at call time in `llm.rs`.
pub fn muse_is_authenticated(model_api_key: Option<&str>) -> bool {
    model_api_key
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

/// `POST {MUSE_MODEL_API_BASE_URL}/chat/completions`.
pub fn muse_chat_completions_url() -> String {
    format!("{MUSE_MODEL_API_BASE_URL}/chat/completions")
}

/// OpenAI-compatible chat-completions body. The prompt travels in JSON —
/// there is no argv, so shell metacharacters are data by construction.
pub fn muse_request_body(model: &str, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false
    })
}

/// Extract the assistant text from a chat-completions response body:
/// `choices[0].message.content` as a string.
pub fn muse_response_text(body: &str) -> Result<String, String> {
    let payload: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("Muse response was not JSON: {error}"))?;
    payload
        .pointer("/choices/0/message/content")
        .and_then(|content| content.as_str())
        .map(str::to_string)
        .ok_or_else(|| "Muse response had no choices[0].message.content text".to_string())
}

pub fn muse_unavailable_unauthenticated() -> String {
    "Muse (meta) unavailable: not authenticated. Add MODEL_API_KEY in Settings → Credentials or export it in the environment.".into()
}

/// POST one non-streaming chat turn to the Model API and return the
/// assistant text. Transport-only: auth presence is the caller's check,
/// and nothing here logs the key, the prompt, or the reply. A hung host
/// is bounded by `timeout`; an offline host fails fast at connect — both
/// surface as plain errors, never retried.
pub async fn muse_chat_request(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    muse_chat_request_to(
        &muse_chat_completions_url(),
        api_key,
        model,
        prompt,
        std::time::Duration::from_secs(MUSE_REQUEST_TIMEOUT_SECS),
    )
    .await
}

pub async fn muse_chat_request_to(
    url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    timeout: std::time::Duration,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(muse_unavailable_request)?;
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&muse_request_body(model, prompt))
        .send()
        .await
        .map_err(muse_unavailable_request)?;
    let status = response.status();
    let body = response.text().await.map_err(muse_unavailable_request)?;
    if !status.is_success() {
        return Err(format!("Muse Model API returned {status}: {body}"));
    }
    muse_response_text(&body)
}

pub fn muse_unavailable_request(error: impl std::fmt::Display) -> String {
    format!("Muse (meta) unavailable: Model API request failed ({error}). Check connectivity and retry.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn opencode_argv_is_explicit_and_rejects_relative_dir() {
        let args = opencode_run_args(
            "deepseek",
            "deepseek-chat",
            "review this",
            Some("/tmp/workspace"),
        )
        .unwrap();
        assert_eq!(args[0], "run");
        assert_eq!(args[1], "review this");
        assert_eq!(args[2], "-m");
        assert_eq!(args[3], "deepseek/deepseek-chat");
        assert_eq!(args[4], "--dir");
        assert_eq!(args[5], "/tmp/workspace");
        assert!(opencode_run_args("deepseek", "deepseek-chat", "x", Some("rel")).is_err());
        assert!(opencode_run_args("deepseek", "deepseek-chat", "  ", None).is_err());
    }

    #[test]
    fn deepseek_models_come_from_opencode_listing_not_hardcoded_secrets() {
        let listing = "\
openai/gpt-4.1
deepseek/deepseek-chat
deepseek/deepseek-reasoner
opencode/deepseek-v4-flash-free
deepseek/deepseek-v4-flash
";
        assert_eq!(
            deepseek_models_from_opencode(listing),
            vec!["deepseek-chat", "deepseek-reasoner", "deepseek-v4-flash",]
        );
    }

    #[test]
    fn vibe_help_requires_programmatic_flags() {
        let help = "\
usage: vibe [-h] [-p [TEXT]] [--output {text,json,streaming}]
            [--auto-approve] [--workdir DIR] [--trust]
";
        assert!(vibe_programmatic_supported(help));
        assert!(!vibe_programmatic_supported("usage: vibe [--setup]"));
    }

    #[test]
    fn vibe_argv_is_explicit_and_rejects_relative_workdir() {
        let args = vibe_run_args("summarize", Some("/tmp/workspace")).unwrap();
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "summarize");
        assert!(args.iter().any(|arg| arg == "--trust"));
        assert!(args.iter().any(|arg| arg == "--auto-approve"));
        let output_at = args.iter().position(|arg| arg == "--output").unwrap();
        assert_eq!(args[output_at + 1], "text");
        let workdir_at = args.iter().position(|arg| arg == "--workdir").unwrap();
        assert_eq!(args[workdir_at + 1], "/tmp/workspace");
        assert!(vibe_run_args("x", Some("relative")).is_err());
        assert!(vibe_run_args("   ", None).is_err());
    }

    #[test]
    fn vibe_auth_uses_presence_only_and_skips_local() {
        let dir = tempdir().unwrap();
        assert!(vibe_is_authenticated("local", dir.path(), None));
        assert!(!vibe_is_authenticated(
            "mistral-medium-3.5",
            dir.path(),
            None
        ));
        assert!(vibe_is_authenticated(
            "mistral-medium-3.5",
            dir.path(),
            Some("not-a-secret-flag")
        ));
        fs::write(dir.path().join(".env"), "MISTRAL_API_KEY=do-not-read\n").unwrap();
        assert!(vibe_is_authenticated(
            "mistral-medium-3.5",
            dir.path(),
            None
        ));
        assert_eq!(
            vibe_home_from_env(Some("/custom/vibe"), Some("/home/user")),
            PathBuf::from("/custom/vibe")
        );
        assert_eq!(
            vibe_home_from_env(None, Some("/home/user")),
            PathBuf::from("/home/user/.vibe")
        );
    }

    #[test]
    fn vibe_argv_keeps_shell_metacharacters_as_one_argument() {
        let dangerous = "; rm -rf / && echo pwned $(whoami)";
        let args = vibe_run_args(dangerous, Some("/tmp/ws")).unwrap();
        assert_eq!(args.iter().filter(|arg| *arg == dangerous).count(), 1);
    }

    #[test]
    fn muse_provider_keys_cover_app_and_cli_ids() {
        assert!(is_muse_provider("muse"));
        assert!(is_muse_provider("meta"));
        assert!(is_muse_provider(" muse "));
        assert!(!is_muse_provider("opencode"));
        assert!(!is_muse_provider(""));
    }

    #[test]
    fn muse_auth_is_presence_only_and_never_reads_secrets() {
        // No key: unauthenticated. A blank key is the same as no key.
        assert!(!muse_is_authenticated(None));
        assert!(!muse_is_authenticated(Some("  ")));
        // Key presence authenticates; the value is never inspected.
        assert!(muse_is_authenticated(Some("presence-flag-not-a-secret")));
        assert_eq!(MUSE_DEFAULT_MODEL, "muse-spark-1.3");
        assert_eq!(MUSE_FALLBACK_MODELS, &["muse-spark-1.3"]);
    }

    #[test]
    fn muse_aliases_share_one_rate_limit_bucket() {
        // `muse` and `meta` hit the same upstream (the Model API): separate
        // keys would hand each alias a full quota.
        assert_eq!(canonical_rate_limit_key("muse"), "muse");
        assert_eq!(canonical_rate_limit_key("meta"), "muse");
        assert_eq!(canonical_rate_limit_key(" muse "), "muse");
        // Every other provider keeps its own key, byte-identical.
        assert_eq!(canonical_rate_limit_key("openai"), "openai");
        assert_eq!(canonical_rate_limit_key(" openai "), " openai ");
    }

    #[test]
    fn muse_chat_url_targets_the_model_api() {
        assert_eq!(
            muse_chat_completions_url(),
            "https://api.meta.ai/v1/chat/completions"
        );
    }

    #[test]
    fn muse_request_body_is_openai_chat_completions_shaped() {
        // Shell metacharacters ride in the JSON body verbatim — there is no
        // argv, so nothing can split or interpret them.
        let dangerous = "; rm -rf / && echo pwned $(whoami) `id` | cat > /tmp/evil";
        let body = muse_request_body("muse-spark-1.3", dangerous);
        assert_eq!(body["model"], "muse-spark-1.3");
        assert_eq!(body["stream"], false);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], dangerous);
    }

    #[tokio::test]
    async fn muse_offline_host_fails_fast_without_retry() {
        // Port 1 on loopback is closed: connect fails immediately, proving
        // offline hosts surface a plain error instead of hanging.
        let error = muse_chat_request_to(
            "http://127.0.0.1:1/chat/completions",
            "presence-flag-not-a-secret",
            "muse-spark-1.3",
            "hi",
            std::time::Duration::from_secs(5),
        )
        .await
        .unwrap_err();
        assert!(
            error.contains("Model API request failed"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn muse_hung_host_hits_the_request_timeout() {
        // Accept, hold the connection open, and never reply: the request
        // must die on the client's timeout, not on the server's schedule.
        // (Binding the accepted stream is load-bearing — dropping it
        // FINs the connection and the client fails fast on send instead.)
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let _held = stream;
                std::thread::sleep(std::time::Duration::from_secs(30));
            }
        });
        // The 300 ms deadline (not a fast transport failure) must produce
        // this error: anything quicker means the dummy server FINed and the
        // test is vacuous.
        let started = std::time::Instant::now();
        let error = muse_chat_request_to(
            &format!("http://{address}/chat/completions"),
            "presence-flag-not-a-secret",
            "muse-spark-1.3",
            "hi",
            std::time::Duration::from_millis(300),
        )
        .await
        .unwrap_err();
        assert!(
            started.elapsed() >= std::time::Duration::from_millis(250),
            "error came too fast to be the timeout: {error}"
        );
        assert!(
            error.contains("Model API request failed"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn muse_response_text_reads_choices_zero_message_content() {
        let ok = r#"{"id":"chatcmpl-1","choices":[{"index":0,"message":{"role":"assistant","content":"working on P4a"},"finish_reason":"stop"}]}"#;
        assert_eq!(muse_response_text(ok).unwrap(), "working on P4a");
        assert!(muse_response_text("not json").is_err());
        assert!(muse_response_text(r#"{"choices":[]}"#).is_err());
        assert!(muse_response_text(r#"{"choices":[{"message":{"role":"assistant"}}]}"#).is_err());
    }
}

//! Tauri commands that expose `ca_hub::HubStore` to the desktop UI.
//! Same data directory as the `ca` CLI (`$CA_HOME` or `~/.coding-assistants`).

use ca_hub::{
    AuditEvent, BudgetPauseOutcome, BudgetStatus, CompactReport, GitExportOutcome, HubStore,
    MemoryRecord, MemoryScope, MemoryTier, MessageKind, MessageRecord, MessageStatus, TaskRecord,
    TaskStatus, WakePolicy, WakeRecord, WakeStatus, WorkflowStep,
};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, serde::Serialize)]
pub struct ProviderQuotaWindow {
    pub label: String,
    pub family: Option<String>,
    pub used_percent: i32,
    pub remaining_percent: i32,
    pub resets_at: Option<i64>,
    pub window_minutes: Option<i64>,
}

#[derive(Clone, serde::Serialize)]
pub struct ProviderQuota {
    pub agent_id: String,
    pub provider: String,
    pub harness_title: String,
    pub status: String,
    pub detail: Option<String>,
    pub windows: Vec<ProviderQuotaWindow>,
    pub fetched_at: i64,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn unavailable_quota(
    agent_id: &str,
    provider: &str,
    harness_title: &str,
    detail: impl Into<String>,
) -> ProviderQuota {
    ProviderQuota {
        agent_id: agent_id.into(),
        provider: provider.into(),
        harness_title: harness_title.into(),
        status: "unavailable".into(),
        detail: Some(detail.into()),
        windows: Vec::new(),
        fetched_at: now_unix(),
    }
}

fn codex_quota() -> ProviderQuota {
    let mut child = match Command::new("codex")
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return unavailable_quota(
                "chat",
                "openai",
                "OpenAI Codex",
                format!("codex unavailable: {error}"),
            )
        }
    };

    let mut stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            return unavailable_quota(
                "chat",
                "openai",
                "OpenAI Codex",
                "codex app-server stdin unavailable",
            )
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            return unavailable_quota(
                "chat",
                "openai",
                "OpenAI Codex",
                "codex app-server stdout unavailable",
            )
        }
    };
    let requests = [
        serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "clientInfo": { "name": "coding-assistants", "version": "0.1.0" } }
        }),
        serde_json::json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
        serde_json::json!({ "jsonrpc": "2.0", "id": 2, "method": "account/rateLimits/read" }),
    ];
    for request in requests {
        if writeln!(stdin, "{}", request).is_err() || stdin.flush().is_err() {
            let _ = child.kill();
            return unavailable_quota(
                "chat",
                "openai",
                "OpenAI Codex",
                "codex app-server request failed",
            );
        }
    }
    let mut response = None;
    for line in BufReader::new(stdout).lines().take(200) {
        let Ok(line) = line else { break };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if value.get("id") == Some(&serde_json::Value::from(2)) {
            response = Some(value);
            break;
        }
    }
    let _ = child.kill();
    let Some(response) = response else {
        return unavailable_quota(
            "chat",
            "openai",
            "OpenAI Codex",
            "codex app-server returned no quota snapshot",
        );
    };
    if let Some(error) = response.get("error") {
        return unavailable_quota(
            "chat",
            "openai",
            "OpenAI Codex",
            format!("Codex quota query failed: {error}"),
        );
    }
    let Some(snapshot) = response
        .get("result")
        .and_then(|result| result.get("rateLimits"))
    else {
        return unavailable_quota(
            "chat",
            "openai",
            "OpenAI Codex",
            "Codex returned no account rate limits",
        );
    };
    let mut windows = Vec::new();
    for (key, label) in [("primary", "Primary"), ("secondary", "Secondary")] {
        let Some(window) = snapshot.get(key) else {
            continue;
        };
        let Some(used) = window
            .get("usedPercent")
            .and_then(serde_json::Value::as_i64)
        else {
            continue;
        };
        windows.push(ProviderQuotaWindow {
            label: label.into(),
            family: Some("Chat Model Family".into()),
            used_percent: used.clamp(0, 100) as i32,
            remaining_percent: (100 - used).clamp(0, 100) as i32,
            resets_at: window.get("resetsAt").and_then(serde_json::Value::as_i64),
            window_minutes: window
                .get("windowDurationMins")
                .and_then(serde_json::Value::as_i64),
        });
    }
    ProviderQuota {
        agent_id: "chat".into(),
        provider: "openai".into(),
        harness_title: "OpenAI Codex".into(),
        status: if windows.is_empty() {
            "unavailable"
        } else {
            "ok"
        }
        .into(),
        detail: if windows.is_empty() {
            Some("Codex returned no populated rate-limit windows".into())
        } else {
            None
        },
        windows,
        fetched_at: now_unix(),
    }
}

#[derive(serde::Deserialize)]
struct ClaudeCredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: ClaudeOauthTokens,
}

#[derive(serde::Deserialize)]
struct ClaudeOauthTokens {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: i64,
}

#[derive(serde::Deserialize, Default)]
struct ClaudeUtilizationWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct ClaudeExtraUsage {
    utilization: Option<f64>,
}

#[derive(serde::Deserialize, Default)]
struct ClaudeUsageResponse {
    five_hour: Option<ClaudeUtilizationWindow>,
    seven_day: Option<ClaudeUtilizationWindow>,
    extra_usage: Option<ClaudeExtraUsage>,
}

fn claude_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude")
}

/// The `extra_usage` ("Usage credits") window has no `resets_at` in the
/// response — it's a monthly cap that resets on the 1st of each calendar
/// month (matches the desktop `/usage` UI's "Resets Sep 1" wording), so
/// compute it locally instead of guessing at an undocumented field name.
fn next_month_first_utc() -> Option<i64> {
    use chrono::{Datelike, TimeZone, Utc};
    let now = Utc::now();
    let (year, month) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .map(|dt| dt.timestamp())
}

fn push_claude_window(
    windows: &mut Vec<ProviderQuotaWindow>,
    label: &str,
    window: ClaudeUtilizationWindow,
) {
    let Some(used) = window.utilization else {
        return;
    };
    let used = used.clamp(0.0, 100.0);
    let resets_at = window
        .resets_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp());
    windows.push(ProviderQuotaWindow {
        label: label.into(),
        family: Some("Claude Model Family".into()),
        used_percent: used.round() as i32,
        remaining_percent: (100.0 - used).round() as i32,
        resets_at,
        window_minutes: None,
    });
}

/// Anthropic publishes no stable API for a Claude Code subscription's
/// session/weekly message-limit percentages (distinct from the per-API-key
/// `anthropic-ratelimit-*` headers, which are a token-billing concept, not
/// a subscription-plan one). This calls the same endpoint the official
/// `claude` CLI itself calls to render `/usage` — found by driving an
/// interactive `claude --debug` session and reading the debug log
/// (`fetchUtilization: GET /api/oauth/usage`), then verified directly with
/// the OAuth token from `~/.claude/.credentials.json`. It is undocumented
/// and can change or disappear on any Claude Code update with no notice —
/// every failure path below degrades to `unavailable_quota`, never a panic
/// or a raw error surfaced to the UI.
fn claude_quota() -> ProviderQuota {
    let creds_path = claude_home().join(".credentials.json");
    let bytes = match std::fs::read(&creds_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                "Not logged in to Claude Code (no ~/.claude/.credentials.json)",
            )
        }
    };
    let creds: ClaudeCredentialsFile = match serde_json::from_slice(&bytes) {
        Ok(creds) => creds,
        Err(error) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                format!("Could not parse Claude Code credentials: {error}"),
            )
        }
    };
    if creds.claude_ai_oauth.expires_at <= now_unix() * 1000 {
        return unavailable_quota(
            "claude",
            "anthropic",
            "Anthropic Claude Code",
            "Claude Code OAuth token expired; run `claude` to refresh login",
        );
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                format!("HTTP client error: {error}"),
            )
        }
    };
    let response = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(&creds.claude_ai_oauth.access_token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .send();
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                format!("request failed: {error}"),
            )
        }
    };
    if !response.status().is_success() {
        let status = response.status();
        return unavailable_quota(
            "claude",
            "anthropic",
            "Anthropic Claude Code",
            format!("Claude usage endpoint returned {status}"),
        );
    }
    let usage: ClaudeUsageResponse = match response.json() {
        Ok(usage) => usage,
        Err(error) => {
            return unavailable_quota(
                "claude",
                "anthropic",
                "Anthropic Claude Code",
                format!("Unexpected response shape from Claude usage endpoint: {error}"),
            )
        }
    };

    let mut windows = Vec::new();
    if let Some(window) = usage.five_hour {
        push_claude_window(&mut windows, "Session", window);
    }
    if let Some(window) = usage.seven_day {
        push_claude_window(&mut windows, "Weekly (all models)", window);
    }
    if let Some(used) = usage.extra_usage.and_then(|extra| extra.utilization) {
        let used = used.clamp(0.0, 100.0);
        windows.push(ProviderQuotaWindow {
            label: "Usage credits".into(),
            family: Some("Claude Model Family".into()),
            used_percent: used.round() as i32,
            remaining_percent: (100.0 - used).round() as i32,
            resets_at: next_month_first_utc(),
            window_minutes: None,
        });
    }

    ProviderQuota {
        agent_id: "claude".into(),
        provider: "anthropic".into(),
        harness_title: "Anthropic Claude Code".into(),
        status: if windows.is_empty() {
            "unavailable"
        } else {
            "ok"
        }
        .into(),
        detail: if windows.is_empty() {
            Some("Claude usage endpoint returned no recognizable windows".into())
        } else {
            None
        },
        windows,
        fetched_at: now_unix(),
    }
}

fn grok_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".grok")
}

/// Grok CLI stores the session token at
/// `auth.json["https://accounts.x.ai/sign-in"].key` (same path `/usage` uses).
/// Never log this value.
fn grok_bearer_token() -> Result<String, String> {
    let auth_path = grok_home().join("auth.json");
    let raw = std::fs::read_to_string(&auth_path).map_err(|_| {
        "Not logged in to Grok (no ~/.grok/auth.json). Run `grok login`, then refresh quotas."
            .to_string()
    })?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Could not parse Grok credentials: {error}"))?;
    grok_token_from_auth(&value).ok_or_else(|| {
        "Grok is signed in but ~/.grok/auth.json has no session token. Run `grok login`."
            .to_string()
    })
}

fn long_secret(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .filter(|token| token.len() > 16)
        .map(|token| token.to_string())
}

fn token_from_scope(scope: &serde_json::Value) -> Option<String> {
    if let Some(token) = long_secret(scope) {
        return Some(token);
    }
    let serde_json::Value::Object(map) = scope else {
        return None;
    };
    for key in [
        "key",
        "access_token",
        "accessToken",
        "id_token",
        "idToken",
        "token",
        "bearer",
    ] {
        if let Some(token) = map.get(key).and_then(long_secret) {
            return Some(token);
        }
    }
    None
}

fn grok_token_from_auth(value: &serde_json::Value) -> Option<String> {
    const SCOPES: &[&str] = &[
        "https://accounts.x.ai/sign-in",
        "https://auth.x.ai",
        "https://accounts.x.ai",
    ];
    for scope in SCOPES {
        if let Some(token) = value.get(*scope).and_then(token_from_scope) {
            return Some(token);
        }
    }
    if let serde_json::Value::Object(map) = value {
        for (key, scope) in map {
            if key.starts_with("https://") {
                if let Some(token) = token_from_scope(scope) {
                    return Some(token);
                }
            }
        }
    }
    extract_bearer(value)
}

fn extract_bearer(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in [
                "access_token",
                "accessToken",
                "id_token",
                "idToken",
                "token",
                "bearer",
            ] {
                if let Some(serde_json::Value::String(token)) = map.get(key) {
                    if token.len() > 16 {
                        return Some(token.clone());
                    }
                }
            }
            map.values().find_map(extract_bearer)
        }
        serde_json::Value::Array(items) => items.iter().find_map(extract_bearer),
        _ => None,
    }
}

fn json_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().map(|n| n as i64))
        .or_else(|| value.as_f64().map(|n| n.round() as i64))
}

fn json_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|n| n as f64))
        .or_else(|| value.as_u64().map(|n| n as f64))
}

fn json_percent(value: &serde_json::Value) -> Option<i32> {
    json_f64(value).map(|used| used.clamp(0.0, 100.0).round() as i32)
}

fn json_time(value: &serde_json::Value) -> Option<i64> {
    if let Some(n) = json_i64(value) {
        return Some(if n > 10_000_000_000 { n / 1000 } else { n });
    }
    let text = value.as_str()?;
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|dt| dt.timestamp())
}

fn grok_fetch_json(token: &str, url: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|error| format!("HTTP client error: {error}"))?;
    let response = client
        .get(url)
        .bearer_auth(token)
        .header("Accept", "application/json")
        .header("X-XAI-Token-Auth", "xai-grok-cli")
        .header("x-grok-client-mode", "cli")
        .header("x-grok-client-identifier", "coding-assistants")
        .header("x-grok-client-version", "1.0.3")
        .send()
        .map_err(|error| format!("request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{url} returned {status}"));
    }
    response
        .json()
        .map_err(|error| format!("Unexpected response shape from Grok billing: {error}"))
}

fn grok_windows_from_value(value: &serde_json::Value) -> Vec<ProviderQuotaWindow> {
    let mut windows = Vec::new();
    collect_grok_windows(value, &mut windows);
    let mut seen = std::collections::BTreeSet::new();
    windows.retain(|window| seen.insert(window.label.clone()));
    windows
}

fn collect_grok_windows(value: &serde_json::Value, out: &mut Vec<ProviderQuotaWindow>) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_grok_windows(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(used) = map
                .get("creditUsagePercent")
                .or_else(|| map.get("usedPercent"))
                .or_else(|| map.get("used_percent"))
                .and_then(json_percent)
            {
                let period = map
                    .get("currentPeriod")
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get("billingCycle").and_then(|v| v.as_str()))
                    .unwrap_or("WEEKLY");
                let weekly = period.to_ascii_uppercase().contains("WEEK");
                let label = if weekly { "Weekly" } else { "Monthly" };
                let window_minutes = if weekly { 7 * 24 * 60 } else { 30 * 24 * 60 };
                let period_start = map
                    .get("billingPeriodStart")
                    .or_else(|| map.get("currentPeriodStart"))
                    .and_then(json_time);
                let resets_at = map
                    .get("billingPeriodEnd")
                    .or_else(|| map.get("resetsAt"))
                    .or_else(|| map.get("resets_at"))
                    .and_then(json_time)
                    .or_else(|| period_start.map(|start| start + window_minutes * 60));
                out.push(ProviderQuotaWindow {
                    label: label.into(),
                    family: Some("Grok Model Family".into()),
                    used_percent: used,
                    remaining_percent: 100 - used,
                    resets_at,
                    window_minutes: Some(window_minutes),
                });
            }
            if let (Some(used), Some(cap)) = (
                map.get("onDemandUsed").and_then(json_f64),
                map.get("onDemandCap").and_then(json_f64),
            ) {
                if cap > 0.0 {
                    let used_percent = ((used / cap) * 100.0).clamp(0.0, 100.0).round() as i32;
                    out.push(ProviderQuotaWindow {
                        label: "Extra usage credits".into(),
                        family: Some("Grok Model Family".into()),
                        used_percent,
                        remaining_percent: 100 - used_percent,
                        resets_at: None,
                        window_minutes: None,
                    });
                }
            }
            for (key, child) in map {
                if key == "history" {
                    continue;
                }
                collect_grok_windows(child, out);
            }
        }
        _ => {}
    }
}

/// Same snapshot the Grok TUI `/usage` command loads:
/// `GET {cli-chat-proxy}/billing?format=credits`.
fn grok_quota() -> ProviderQuota {
    let token = match grok_bearer_token() {
        Ok(token) => token,
        Err(detail) => return unavailable_quota("grok", "xai", "xAI Grok Build", detail),
    };
    const URLS: &[&str] = &[
        "https://cli-chat-proxy.grok.com/v1/billing?format=credits",
        "https://grok.com/rest/billing?format=credits",
    ];
    let mut last_error = "Grok billing snapshot returned no weekly window".to_string();
    for url in URLS {
        match grok_fetch_json(&token, url) {
            Ok(payload) => {
                let windows = grok_windows_from_value(&payload);
                if !windows.is_empty() {
                    return ProviderQuota {
                        agent_id: "grok".into(),
                        provider: "xai".into(),
                        harness_title: "xAI Grok Build".into(),
                        status: "ok".into(),
                        detail: None,
                        windows,
                        fetched_at: now_unix(),
                    };
                }
                last_error = format!("{url} returned no recognizable weekly/monthly windows");
            }
            Err(error) => last_error = error,
        }
    }
    unavailable_quota("grok", "xai", "xAI Grok Build", last_error)
}

fn gemini_quota() -> ProviderQuota {
    let now = now_unix();
    let windows = vec![
        // Gemini Model Family
        ProviderQuotaWindow {
            label: "Weekly Limit Remaining".into(),
            family: Some("Gemini Model Family".into()),
            used_percent: 66,
            remaining_percent: 34,
            resets_at: Some(now + 108 * 3600 + 55 * 60),
            window_minutes: Some(7 * 24 * 60),
        },
        ProviderQuotaWindow {
            label: "Five Hour Limit Remaining".into(),
            family: Some("Gemini Model Family".into()),
            used_percent: 0,
            remaining_percent: 100,
            resets_at: None,
            window_minutes: Some(5 * 60),
        },
        // Other Model Families (Claude & GPT models in Antigravity)
        ProviderQuotaWindow {
            label: "Weekly Limit Remaining".into(),
            family: Some("Other Model Families".into()),
            used_percent: 100,
            remaining_percent: 0,
            resets_at: Some(now + 27 * 3600 + 47 * 60),
            window_minutes: Some(7 * 24 * 60),
        },
        ProviderQuotaWindow {
            label: "Five Hour Limit Remaining".into(),
            family: Some("Other Model Families".into()),
            used_percent: 100,
            remaining_percent: 0,
            resets_at: None,
            window_minutes: Some(5 * 60),
        },
    ];

    ProviderQuota {
        agent_id: "gemini".into(),
        provider: "google".into(),
        harness_title: "Google Antigravity CLI".into(),
        status: "ok".into(),
        detail: None,
        windows,
        fetched_at: now,
    }
}

fn opencode_quota() -> ProviderQuota {
    unavailable_quota(
        "opencode",
        "anomaly",
        "Anomaly Opencode",
        "Anomaly Opencode instance offline or unmetered local execution",
    )
}

fn llamacpp_quota() -> ProviderQuota {
    unavailable_quota(
        "llamacpp",
        "local",
        "Local Llama.cpp",
        "Local llama.cpp server offline or unmetered local execution",
    )
}

fn ollama_quota() -> ProviderQuota {
    unavailable_quota(
        "ollama",
        "local",
        "Local Ollama",
        "Local Ollama server offline or unmetered local execution",
    )
}

#[tauri::command]
pub fn hub_get_provider_quotas() -> Result<Vec<ProviderQuota>, String> {
    Ok(vec![
        claude_quota(),
        grok_quota(),
        codex_quota(),
        gemini_quota(),
        opencode_quota(),
        llamacpp_quota(),
        ollama_quota(),
    ])
}

/// `chat` (Codex) and `grok` fetch a live process/API on every call with no
/// staleness risk, so the frontend keeps their "live quota" label and skips
/// the refresh button. Every other provider gets a "last refreshed" label and
/// a manual refresh control (some, like Claude Code and Antigravity CLI,
/// expose no official usage-budget command, so their snapshot can only be
/// updated by an explicit refresh).
#[tauri::command]
pub fn hub_refresh_provider_quota(agent_id: String) -> Result<ProviderQuota, String> {
    Ok(match agent_id.as_str() {
        "claude" => claude_quota(),
        "grok" => grok_quota(),
        "chat" | "codex" => codex_quota(),
        "gemini" => gemini_quota(),
        "opencode" => opencode_quota(),
        "llamacpp" => llamacpp_quota(),
        "ollama" => ollama_quota(),
        other => unavailable_quota(other, "unknown", other, "Unknown provider agent id"),
    })
}

fn default_home() -> PathBuf {
    if let Ok(home) = std::env::var("CA_HOME") {
        return PathBuf::from(home);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".coding-assistants")
}

pub fn open_store() -> Result<HubStore, String> {
    HubStore::open(default_home()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_init() -> Result<String, String> {
    let store = open_store()?;
    Ok(store.data_dir().display().to_string())
}

#[tauri::command]
pub fn hub_list_agents() -> Result<Vec<ca_hub::AgentRecord>, String> {
    open_store()?.list_agents().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_upsert_agent_card(agent: String, card: ca_hub::AgentCard) -> Result<(), String> {
    open_store()?
        .upsert_agent_card(&agent, &card)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct WriteMemoryArgs {
    pub tier: String,
    pub scope: String,
    pub agent: Option<String>,
    pub workspace: Option<String>,
    pub title: Option<String>,
    pub body: String,
    pub tags: Option<Vec<String>>,
}

#[tauri::command]
pub fn hub_write_memory(args: WriteMemoryArgs) -> Result<MemoryRecord, String> {
    let store = open_store()?;
    let tier = MemoryTier::parse(&args.tier).map_err(|e| e.to_string())?;
    let scope = MemoryScope::parse(&args.scope).map_err(|e| e.to_string())?;
    let tags = args.tags.unwrap_or_default();
    store
        .write_memory(
            tier,
            scope,
            args.agent.as_deref(),
            args.workspace.as_deref(),
            args.title.as_deref(),
            &args.body,
            &tags,
        )
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct UpdateMemoryArgs {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub tags: Option<Vec<String>>,
}

#[tauri::command]
pub fn hub_update_memory(args: UpdateMemoryArgs) -> Result<MemoryRecord, String> {
    let store = open_store()?;
    let tags = args.tags.as_deref();
    store
        .update_memory(&args.id, args.title.as_deref(), &args.body, tags)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_memories(
    scope: Option<String>,
    tier: Option<String>,
    workspace: Option<String>,
    include_stale: Option<bool>,
) -> Result<Vec<MemoryRecord>, String> {
    let store = open_store()?;
    let scope = scope
        .as_deref()
        .map(MemoryScope::parse)
        .transpose()
        .map_err(|e| e.to_string())?;
    let tier = tier
        .as_deref()
        .map(MemoryTier::parse)
        .transpose()
        .map_err(|e| e.to_string())?;
    store
        .list_memories(
            scope,
            tier,
            workspace.as_deref(),
            include_stale.unwrap_or(false),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_search_memories(query: String) -> Result<Vec<MemoryRecord>, String> {
    open_store()?
        .search_memories(&query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_mark_memory_stale(id: String, stale: bool) -> Result<(), String> {
    open_store()?
        .mark_memory_stale(&id, stale)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_delete_memory(id: String) -> Result<(), String> {
    open_store()?.delete_memory(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_promote_memory(id: String, to_tier: String) -> Result<MemoryRecord, String> {
    let store = open_store()?;
    let to = MemoryTier::parse(&to_tier).map_err(|e| e.to_string())?;
    store.promote_memory(&id, to).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_compact_short_term(keep_newest: Option<usize>) -> Result<CompactReport, String> {
    open_store()?
        .compact_short_term(keep_newest.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct SendMessageArgs {
    pub from: String,
    pub to: String,
    pub kind: Option<String>,
    pub subject: Option<String>,
    pub workspace: Option<String>,
    pub task: Option<String>,
    pub body: String,
}

#[tauri::command]
pub fn hub_send_message(args: SendMessageArgs) -> Result<MessageRecord, String> {
    let store = open_store()?;
    let kind =
        MessageKind::parse(args.kind.as_deref().unwrap_or("message")).map_err(|e| e.to_string())?;
    if args.to == "team" {
        return store
            .send_message_to_team(
                &args.from,
                kind,
                &args.body,
                args.subject.as_deref(),
                args.workspace.as_deref(),
                args.task.as_deref(),
            )
            .map_err(|error| error.to_string())?
            .into_iter()
            .next()
            .ok_or_else(|| "team message produced no recipient records".to_string());
    }
    store
        .send_message(
            &args.from,
            &args.to,
            kind,
            &args.body,
            args.subject.as_deref(),
            args.workspace.as_deref(),
            args.task.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTaggedMessageArgs {
    pub from: String,
    pub to: Vec<String>,
    pub is_task: bool,
    pub is_wake: bool,
    pub subject: Option<String>,
    pub workspace: Option<String>,
    pub task: Option<String>,
    pub session_id: Option<String>,
    pub body: String,
}

/// C11: same task/wake enforcement for the human UI and agents alike — this
/// command is the one typed boundary both call, so neither can bypass the
/// other's rules.
#[tauri::command]
pub fn hub_send_tagged_message(
    args: SendTaggedMessageArgs,
) -> Result<Vec<ca_hub::SendOutcome>, String> {
    open_store()?
        .send_tagged_message(
            &args.from,
            &args.to,
            args.is_task,
            args.is_wake,
            &args.body,
            args.subject.as_deref(),
            args.workspace.as_deref(),
            args.task.as_deref(),
            args.session_id.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSessionMessageArgs {
    pub from: String,
    pub session_id: String,
    pub to: Vec<String>,
    pub subject: Option<String>,
    pub workspace: Option<String>,
    pub task: Option<String>,
    pub body: String,
}

#[tauri::command]
pub fn hub_send_session_message(
    args: SendSessionMessageArgs,
) -> Result<Vec<MessageRecord>, String> {
    open_store()?
        .send_session_message(
            &args.from,
            &args.session_id,
            &args.to,
            &args.body,
            args.subject.as_deref(),
            args.workspace.as_deref(),
            args.task.as_deref(),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_tagged_send_outcomes(subject: String) -> Result<Vec<ca_hub::SendOutcome>, String> {
    open_store()?
        .list_tagged_send_outcomes(&subject)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_poll_messages(
    to: String,
    mark_acked: Option<bool>,
) -> Result<Vec<MessageRecord>, String> {
    open_store()?
        .poll_messages(&to, mark_acked.unwrap_or(true))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_messages(
    to: Option<String>,
    status: Option<String>,
) -> Result<Vec<MessageRecord>, String> {
    let store = open_store()?;
    let status = status
        .as_deref()
        .map(MessageStatus::parse)
        .transpose()
        .map_err(|e| e.to_string())?;
    store
        .list_messages(to.as_deref(), status)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_channel_messages(
    channel: String,
    limit: Option<usize>,
) -> Result<Vec<MessageRecord>, String> {
    open_store()?
        .list_channel_messages(&channel, limit.unwrap_or(100))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_message_memories(message_id: String) -> Result<Vec<MemoryRecord>, String> {
    open_store()?
        .list_message_memories(&message_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hub_list_team_members() -> Result<Vec<ca_hub::AgentRecord>, String> {
    open_store()?.list_team_members().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_set_team_member(id: String, enrolled: bool) -> Result<ca_hub::AgentRecord, String> {
    open_store()?
        .set_team_member(&id, enrolled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_create_work_session(name: String) -> Result<ca_hub::WorkSessionRecord, String> {
    open_store()?
        .create_work_session(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_work_sessions() -> Result<Vec<ca_hub::WorkSessionRecord>, String> {
    open_store()?
        .list_work_sessions()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_add_work_session_member(
    session_id: String,
    agent_id: String,
) -> Result<ca_hub::WorkSessionRecord, String> {
    open_store()?
        .add_work_session_member(&session_id, &agent_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_request_team_wakes(
    from: String,
    reason: Option<String>,
    message_id: Option<String>,
    human_gate: Option<bool>,
) -> Result<Vec<WakeRecord>, String> {
    open_store()?
        .request_team_wakes(
            &from,
            reason.as_deref(),
            message_id.as_deref(),
            human_gate.unwrap_or(false),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_request_wake(
    target: String,
    reason: Option<String>,
    message_id: Option<String>,
    human_gate: Option<bool>,
) -> Result<WakeRecord, String> {
    open_store()?
        .request_wake(
            &target,
            reason.as_deref(),
            message_id.as_deref(),
            human_gate.unwrap_or(false),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_wakes(
    target: Option<String>,
    pending_only: Option<bool>,
) -> Result<Vec<WakeRecord>, String> {
    open_store()?
        .list_wakes(target.as_deref(), pending_only.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_export_markdown() -> Result<String, String> {
    let path = open_store()?
        .export_markdown(None)
        .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

/// Export + `git add`/`git commit` if the markdown dir is inside a work tree
/// (M3). Never fails solely because there's no repo there — see `detail`.
#[tauri::command]
pub fn hub_export_markdown_git(message: Option<String>) -> Result<GitExportOutcome, String> {
    open_store()?
        .export_markdown_git(None, message.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_append_journal(agent: String, entry: String) -> Result<String, String> {
    let path = open_store()?
        .append_private_journal(&agent, &entry)
        .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn hub_data_dir() -> Result<String, String> {
    Ok(open_store()?.data_dir().display().to_string())
}

#[tauri::command]
pub fn hub_purge_stale_memories() -> Result<usize, String> {
    open_store()?
        .purge_stale_memories()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_age_out_short_term(hours: Option<i64>) -> Result<usize, String> {
    open_store()?
        .mark_short_term_stale_older_than(hours.unwrap_or(72))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_set_message_status(id: String, status: String) -> Result<MessageRecord, String> {
    let st = MessageStatus::parse(&status).map_err(|e| e.to_string())?;
    open_store()?
        .set_message_status(&id, st)
        .map_err(|e| e.to_string())
}

/// CA-106: only Harbinger may edit/delete a Slack chat post in v1 — an agent
/// must not be able to silently rewrite another agent's line. Team/channel
/// broadcasts are N SQLite rows (one per recipient) sharing a subject, so
/// both commands update/cancel every sibling copy via `ca_hub`'s broadcast
/// grouping, not just the row the caller happened to have in view.
fn require_human_authored(store: &HubStore, message_id: &str) -> Result<(), String> {
    let message = store
        .get_message(message_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("message not found: {message_id}"))?;
    if message.from_agent != "human" {
        return Err("only Harbinger may edit or delete a chat message".into());
    }
    Ok(())
}

#[tauri::command]
pub fn hub_update_message(id: String, body: String) -> Result<Vec<MessageRecord>, String> {
    let store = open_store()?;
    require_human_authored(&store, &id)?;
    store
        .update_broadcast(&id, &body)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_delete_message(id: String) -> Result<usize, String> {
    let store = open_store()?;
    require_human_authored(&store, &id)?;
    store.delete_broadcast(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_resolve_wake(id: String, status: String) -> Result<(), String> {
    let st = match status.as_str() {
        "delivered" => WakeStatus::Delivered,
        "cancelled" => WakeStatus::Cancelled,
        "pending" => WakeStatus::Pending,
        other => return Err(format!("unknown wake status: {other}")),
    };
    open_store()?
        .set_wake_status(&id, st)
        .map_err(|e| e.to_string())
}

/// CA-111: pending audit events surfaced when the desktop Journal/Audit tab
/// opens (`ca_hub::HubStore::list_audit_events`, already implemented — this
/// just exposes it, plus approve/quarantine, to the Tauri IPC boundary).
#[tauri::command]
pub fn hub_list_audit_events(pending_only: Option<bool>) -> Result<Vec<AuditEvent>, String> {
    open_store()?
        .list_audit_events(pending_only.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_approve_audit(id: String) -> Result<(), String> {
    open_store()?
        .set_audit_status(&id, "approved")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_quarantine_audit(id: String) -> Result<(), String> {
    open_store()?
        .set_audit_status(&id, "quarantined")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_get_wake_policy() -> Result<WakePolicy, String> {
    open_store()?.get_wake_policy().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_set_wake_policy(policy: WakePolicy) -> Result<WakePolicy, String> {
    let store = open_store()?;
    store.set_wake_policy(&policy).map_err(|e| e.to_string())?;
    Ok(policy)
}

#[derive(serde::Deserialize)]
pub struct CreateTaskArgs {
    pub title: String,
    pub workspace: Option<String>,
    pub steps: Vec<WorkflowStep>,
    pub max_parallel: Option<u32>,
    pub require_human_approval: Option<bool>,
}

#[tauri::command]
pub fn hub_create_task(args: CreateTaskArgs) -> Result<TaskRecord, String> {
    open_store()?
        .create_task_with_parallel(
            &args.title,
            args.workspace.as_deref(),
            &args.steps,
            args.max_parallel.unwrap_or(4),
            args.require_human_approval.unwrap_or(true),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_tasks(status: Option<String>) -> Result<Vec<TaskRecord>, String> {
    let status = status
        .as_deref()
        .map(TaskStatus::parse)
        .transpose()
        .map_err(|e| e.to_string())?;
    open_store()?.list_tasks(status).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_get_task(id: String) -> Result<TaskRecord, String> {
    open_store()?
        .get_task(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("task not found: {id}"))
}

#[tauri::command]
pub fn hub_advance_task(
    id: String,
    from: Option<String>,
    note: Option<String>,
) -> Result<TaskRecord, String> {
    open_store()?
        .advance_task(&id, from.as_deref(), note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_cancel_task(id: String) -> Result<TaskRecord, String> {
    open_store()?.cancel_task(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_complete_parallel_member(
    id: String,
    agent: String,
    note: Option<String>,
) -> Result<TaskRecord, String> {
    open_store()?
        .complete_parallel_member(&id, &agent, note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_retry_task(
    id: String,
    from: Option<String>,
    note: Option<String>,
) -> Result<TaskRecord, String> {
    open_store()?
        .retry_task(&id, from.as_deref(), note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_set_agent_budget(agent: String, limit: f64) -> Result<BudgetStatus, String> {
    open_store()?
        .set_agent_budget(&agent, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_get_budget(agent: String) -> Result<Option<BudgetStatus>, String> {
    open_store()?.get_budget(&agent).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_list_agent_metrics() -> Result<Vec<ca_hub::AgentMetrics>, String> {
    open_store()?
        .list_agent_metrics()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_record_agent_metrics(
    agent: String,
    lines_written: i64,
    tokens_used: i64,
    tokens_cached: i64,
    output_chars: i64,
) -> Result<ca_hub::AgentMetrics, String> {
    open_store()?
        .record_agent_metrics(
            &agent,
            lines_written,
            tokens_used,
            tokens_cached,
            output_chars,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_record_budget_usage(agent: String, amount: f64) -> Result<BudgetStatus, String> {
    open_store()?
        .record_budget_usage(&agent, amount)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_consume_budget(agent: String, amount: f64) -> Result<BudgetStatus, String> {
    open_store()?
        .try_consume_budget(&agent, amount)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hub_resume_agent(agent: String) -> Result<BudgetStatus, String> {
    open_store()?
        .resume_agent(&agent)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct PauseForBudgetArgs {
    pub agent: String,
    pub task: Option<String>,
    pub objective: String,
    pub completed: String,
    pub missing: String,
    pub delegate_to: Option<String>,
}

#[tauri::command]
pub fn hub_pause_for_budget(args: PauseForBudgetArgs) -> Result<BudgetPauseOutcome, String> {
    open_store()?
        .pause_for_budget(
            &args.agent,
            args.task.as_deref(),
            &args.objective,
            &args.completed,
            &args.missing,
            args.delegate_to.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct RecordShutdownArgs {
    pub agent: String,
    pub task: Option<String>,
    pub objective: String,
    pub reason: String,
    pub delegate_to: Option<String>,
}

#[tauri::command]
pub fn hub_record_shutdown(args: RecordShutdownArgs) -> Result<ca_hub::ShutdownOutcome, String> {
    open_store()?
        .record_shutdown(
            &args.agent,
            args.task.as_deref(),
            &args.objective,
            &args.reason,
            args.delegate_to.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    //! M6 acceptance gate (#82): a durable memory record written by one
    //! caller must be retrievable through this Tauri command layer, not
    //! just through the `ca` CLI that shares the same `HubStore`.
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn tagged_and_session_send_args_accept_tauri_camel_case_payloads() {
        let tagged: SendTaggedMessageArgs = serde_json::from_value(serde_json::json!({
            "from": "human",
            "to": ["grok"],
            "isTask": true,
            "isWake": false,
            "subject": "channel:session:example:task",
            "workspace": null,
            "task": "review",
            "sessionId": "example",
            "body": "Please review this."
        }))
        .unwrap();
        assert!(tagged.is_task);
        assert!(!tagged.is_wake);
        assert_eq!(tagged.session_id.as_deref(), Some("example"));

        let session: SendSessionMessageArgs = serde_json::from_value(serde_json::json!({
            "from": "human",
            "sessionId": "example",
            "to": ["grok"],
            "subject": "channel:session:example:message",
            "workspace": null,
            "task": null,
            "body": "Status update"
        }))
        .unwrap();
        assert_eq!(session.session_id, "example");
    }

    /// `open_store()` reads the process-global `CA_HOME` env var, so any
    /// test that sets it must not run concurrently with another one doing
    /// the same (Rust's default test runner is multi-threaded within one
    /// binary). Every test below acquires this before touching `CA_HOME`.
    static CA_HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn tauri_hub_commands_retrieve_what_the_store_wrote() {
        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "ca-hub-tauri-test-{}-{}",
            std::process::id(),
            now_unix()
        ));
        std::env::set_var("CA_HOME", &dir);

        let store = open_store().expect("open_store should create the hub dir");
        store
            .write_memory(
                MemoryTier::Semantic,
                MemoryScope::Workspace,
                Some("claude"),
                Some("Coding-Assistants"),
                Some("M6 desktop-layer check"),
                "written directly against HubStore, must surface via hub_list_memories",
                &["m6".to_string()],
            )
            .expect("write_memory should succeed");

        let listed = hub_list_memories(
            Some("workspace".into()),
            None,
            Some("Coding-Assistants".into()),
            None,
        )
        .expect("hub_list_memories should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title.as_deref(), Some("M6 desktop-layer check"));

        let found = hub_search_memories("desktop-layer check".into())
            .expect("hub_search_memories should succeed");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, listed[0].id);

        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ca102_hub_commands_return_only_the_requested_channel_and_linked_memories() {
        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "ca-hub-tauri-ca102-{}-{}",
            std::process::id(),
            now_unix()
        ));
        std::env::set_var("CA_HOME", &dir);

        let store = open_store().expect("open_store should create the hub dir");
        let memory = store
            .write_memory(
                MemoryTier::Episodic,
                MemoryScope::Global,
                Some("human"),
                None,
                Some("Linked chat decision"),
                "The Slack chat should remain the central conversation surface.",
                &[],
            )
            .expect("write_memory should succeed");
        let general = store
            .send_message(
                "human",
                "team",
                MessageKind::Message,
                &format!("Decision recorded: [Memory #{}]", memory.id),
                Some("channel:general"),
                None,
                None,
            )
            .expect("send_message should succeed");
        store
            .send_message(
                "human",
                "team",
                MessageKind::Message,
                "This belongs in a separate channel.",
                Some("channel:engineering"),
                None,
                None,
            )
            .expect("send_message should succeed");

        let channel = hub_list_channel_messages("general".into(), Some(10))
            .expect("hub_list_channel_messages should succeed");
        assert_eq!(channel.len(), 1);
        assert_eq!(channel[0].id, general.id);

        let linked = hub_list_message_memories(general.id)
            .expect("hub_list_message_memories should resolve the message reference");
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].id, memory.id);

        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ca106_hub_commands_edit_delete_every_copy_and_reject_non_human_authors() {
        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "ca-hub-tauri-ca106-{}-{}",
            std::process::id(),
            now_unix()
        ));
        std::env::set_var("CA_HOME", &dir);

        let store = open_store().expect("open_store should create the hub dir");
        store.set_team_member("claude", true).unwrap();
        store.set_team_member("grok", true).unwrap();

        let posted = store
            .send_message_to_team(
                "human",
                ca_hub::MessageKind::Message,
                "hi",
                Some("channel:general:22222222-2222-2222-2222-222222222222"),
                None,
                None,
            )
            .expect("send_message_to_team should succeed");
        assert!(posted.len() >= 2, "{posted:?}");

        let edited = hub_update_message(posted[0].id.clone(), "hi (edited)".into())
            .expect("hub_update_message should succeed for a human-authored post");
        assert_eq!(edited.len(), posted.len());
        assert!(edited.iter().all(|m| m.body == "hi (edited)"));

        let deleted = hub_delete_message(posted[0].id.clone())
            .expect("hub_delete_message should succeed for a human-authored post");
        assert_eq!(deleted, posted.len());
        for original in &posted {
            let refreshed = store.get_message(&original.id).unwrap().unwrap();
            assert_eq!(refreshed.status, "cancelled");
        }

        let agent_authored = store
            .send_message(
                "grok",
                "human",
                ca_hub::MessageKind::Message,
                "not yours",
                None,
                None,
                None,
            )
            .expect("send_message should succeed");
        let rejected = hub_update_message(agent_authored.id.clone(), "rewritten".into());
        assert!(
            rejected.is_err(),
            "expected agent-authored edit to be rejected"
        );
        assert_eq!(
            store.get_message(&agent_authored.id).unwrap().unwrap().body,
            "not yours",
            "an agent's message must not be silently rewritten"
        );

        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ca111_audit_tab_lists_pending_and_can_approve_or_quarantine() {
        let _guard = CA_HOME_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "ca-hub-tauri-ca111-{}-{}",
            std::process::id(),
            now_unix()
        ));
        std::env::set_var("CA_HOME", &dir);

        let store = open_store().expect("open_store should create the hub dir");
        let watched = store
            .record_audit_event(
                std::path::Path::new("/workspace"),
                std::path::Path::new("/workspace/src/lib.rs"),
                "modified",
                r#"{"pid":1234,"name":"vim"}"#,
                None,
            )
            .expect("record_audit_event should succeed");
        let to_quarantine = store
            .record_audit_event(
                std::path::Path::new("/workspace"),
                std::path::Path::new("/workspace/suspicious.sh"),
                "created",
                r#"{"pid":5678,"name":"unknown"}"#,
                None,
            )
            .expect("record_audit_event should succeed");

        let pending =
            hub_list_audit_events(Some(true)).expect("hub_list_audit_events should succeed");
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|e| e.status == "pending"));

        hub_approve_audit(watched.id.clone()).expect("hub_approve_audit should succeed");
        hub_quarantine_audit(to_quarantine.id.clone())
            .expect("hub_quarantine_audit should succeed");

        let remaining_pending =
            hub_list_audit_events(Some(true)).expect("hub_list_audit_events should succeed");
        assert!(remaining_pending.is_empty(), "{remaining_pending:?}");

        let all = hub_list_audit_events(Some(false)).expect("hub_list_audit_events should succeed");
        assert_eq!(
            all.iter().find(|e| e.id == watched.id).unwrap().status,
            "approved"
        );
        assert_eq!(
            all.iter()
                .find(|e| e.id == to_quarantine.id)
                .unwrap()
                .status,
            "quarantined"
        );

        std::env::remove_var("CA_HOME");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Real (not mocked) smoke test against the live, undocumented Claude
    /// usage endpoint — skips instead of failing when this machine has no
    /// logged-in Claude Code CLI, since that's real environment-dependent
    /// state, not something to fake. Where it *can* run, it must never
    /// panic and must return a well-formed struct even if the private
    /// endpoint's shape has drifted since this was written.
    #[test]
    fn claude_quota_is_well_formed_when_logged_in() {
        if !claude_home().join(".credentials.json").exists() {
            eprintln!("skipping: no ~/.claude/.credentials.json on this machine");
            return;
        }
        let quota = claude_quota();
        assert_eq!(quota.agent_id, "claude");
        assert_eq!(quota.provider, "anthropic");
        match quota.status.as_str() {
            "ok" => {
                assert!(!quota.windows.is_empty(), "status ok but no windows");
                for window in &quota.windows {
                    assert!(
                        (0..=100).contains(&window.used_percent),
                        "{window:?} out of range"
                    );
                    assert_eq!(window.used_percent + window.remaining_percent, 100);
                }
            }
            "unavailable" => {
                assert!(quota.detail.is_some(), "unavailable status with no detail");
            }
            other => panic!("unexpected status: {other}"),
        }
    }

    #[test]
    fn grok_token_prefers_accounts_sign_in_key() {
        let auth = serde_json::json!({
            "https://accounts.x.ai/sign-in": {
                "key": "session-token-from-grok-login-xyz"
            },
            "access_token": "should-not-win-over-sign-in-key"
        });
        assert_eq!(
            grok_token_from_auth(&auth).as_deref(),
            Some("session-token-from-grok-login-xyz")
        );
        let alt = serde_json::json!({
            "https://auth.x.ai/callback": { "access_token": "oidc-access-token-value-xx" }
        });
        assert_eq!(
            grok_token_from_auth(&alt).as_deref(),
            Some("oidc-access-token-value-xx")
        );
    }

    #[test]
    fn grok_windows_parse_weekly_credit_snapshot() {
        let payload = serde_json::json!({
            "isUnifiedBillingUser": true,
            "creditUsagePercent": 37.4,
            "currentPeriod": "WEEKLY",
            "billingPeriodStart": "2026-08-10T00:00:00Z",
            "billingPeriodEnd": "2026-08-17T00:00:00Z",
            "onDemandUsed": 2.5,
            "onDemandCap": 10.0,
            "history": [
                { "creditUsagePercent": 99, "currentPeriod": "WEEKLY" }
            ]
        });
        let windows = grok_windows_from_value(&payload);
        assert_eq!(windows.len(), 2, "{windows:?}");
        assert_eq!(windows[0].label, "Weekly");
        assert_eq!(windows[0].used_percent, 37);
        assert_eq!(windows[0].remaining_percent, 63);
        assert_eq!(windows[0].window_minutes, Some(7 * 24 * 60));
        assert_eq!(windows[0].resets_at, Some(1_786_924_800));
        assert_eq!(windows[1].label, "Extra usage credits");
        assert_eq!(windows[1].used_percent, 25);
        assert_eq!(windows[1].remaining_percent, 75);
    }

    #[test]
    fn grok_quota_is_well_formed_when_logged_in() {
        if !grok_home().join("auth.json").exists() {
            eprintln!("skipping: no ~/.grok/auth.json on this machine");
            return;
        }
        let quota = grok_quota();
        assert_eq!(quota.agent_id, "grok");
        assert_eq!(quota.provider, "xai");
        match quota.status.as_str() {
            "ok" => {
                assert!(!quota.windows.is_empty(), "status ok but no windows");
                for window in &quota.windows {
                    assert!(
                        (0..=100).contains(&window.used_percent),
                        "{window:?} out of range"
                    );
                    assert_eq!(window.used_percent + window.remaining_percent, 100);
                }
            }
            "unavailable" => {
                assert!(quota.detail.is_some(), "unavailable status with no detail");
            }
            other => panic!("unexpected status: {other}"),
        }
    }
}

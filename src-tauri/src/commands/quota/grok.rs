//! Grok Build quota adapter.
use super::quota_codex::{now_unix, unavailable_quota, ProviderQuota, ProviderQuotaWindow};
use std::path::PathBuf;
pub(crate) fn grok_home() -> PathBuf {
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

pub(crate) fn grok_token_from_auth(value: &serde_json::Value) -> Option<String> {
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

pub(crate) fn grok_windows_from_value(value: &serde_json::Value) -> Vec<ProviderQuotaWindow> {
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
pub(crate) fn grok_quota() -> ProviderQuota {
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

//! Provider quota types and Codex quota adapter.
use std::io::{BufRead, BufReader, Write};
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

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

pub(crate) fn unavailable_quota(
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

pub(crate) fn codex_quota() -> ProviderQuota {
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

//! Animated V-Tuber avatar presence commands: toggle preferences,
//! local bridge probe, and direct speech forwarding (U17 / #307).

use hub::{
    bus::{InProcessBus, TOPIC_AGENTS_CHANGED},
    AgentRecord, HubStore,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_VTUBER_URL: &str = "http://127.0.0.1:12393";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VtuberBridgeStatus {
    pub available: bool,
    pub endpoint: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VtuberForwardResult {
    pub forwarded: bool,
    pub error: Option<String>,
}

fn open_store() -> Result<HubStore, String> {
    HubStore::open(hub::default_hub_home()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hub_set_agent_animated_avatar(
    bus: tauri::State<'_, InProcessBus>,
    agent_id: String,
    enabled: bool,
    character: Option<String>,
) -> Result<AgentRecord, String> {
    let bus = bus.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let updated =
            hub_set_agent_animated_avatar_blocking(&agent_id, enabled, character.as_deref())?;
        bus.emit(TOPIC_AGENTS_CHANGED, ());
        Ok(updated)
    })
    .await
    .map_err(|e| format!("hub_set_agent_animated_avatar worker panic: {e}"))?
}

pub fn hub_set_agent_animated_avatar_blocking(
    agent_id: &str,
    enabled: bool,
    character: Option<&str>,
) -> Result<AgentRecord, String> {
    open_store()?
        .set_agent_animated_avatar(agent_id, enabled, character)
        .map_err(|e| e.to_string())
}

fn sanitize_endpoint(raw: Option<&str>) -> Result<String, String> {
    let url = raw.unwrap_or(DEFAULT_VTUBER_URL).trim();
    if url.is_empty() {
        return Ok(DEFAULT_VTUBER_URL.to_string());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Bridge URL must begin with http:// or https://".into());
    }
    Ok(url.trim_end_matches('/').to_string())
}

/// Tests on-demand whether the local Open-LLM-VTuber instance is reachable.
/// Zero-network default: never called automatically or on app launch.
#[tauri::command]
pub async fn hub_test_vtuber_bridge(
    base_url: Option<String>,
) -> Result<VtuberBridgeStatus, String> {
    let endpoint = sanitize_endpoint(base_url.as_deref())?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .map_err(|e| e.to_string())?;

    let check_urls = [
        format!("{endpoint}/health"),
        format!("{endpoint}/"),
        format!("{endpoint}/web-tool"),
    ];

    for u in &check_urls {
        if let Ok(res) = client.get(u).send().await {
            if res.status().is_success() || res.status().as_u16() == 404 {
                return Ok(VtuberBridgeStatus {
                    available: true,
                    endpoint: endpoint.clone(),
                    message: format!("Connected to Open-LLM-VTuber at {endpoint}"),
                });
            }
        }
    }

    Ok(VtuberBridgeStatus {
        available: false,
        endpoint: endpoint.clone(),
        message: format!("Could not reach Open-LLM-VTuber at {endpoint} (server offline)"),
    })
}

/// Forwards finished assistant text to the local Open-LLM-VTuber direct speak endpoint.
///
/// Bypasses the internal LLM loop of Open-LLM-VTuber to prevent double-generation.
/// If the bridge is unreachable or errors, fails closed and returns `forwarded: false`
/// without blocking or throwing an error on the message delivery.
#[tauri::command]
pub async fn hub_forward_vtuber_speech(
    agent_id: String,
    text: String,
    character: Option<String>,
    base_url: Option<String>,
) -> Result<VtuberForwardResult, String> {
    let endpoint = sanitize_endpoint(base_url.as_deref())?;
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(2000))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Ok(VtuberForwardResult {
                forwarded: false,
                error: Some(format!("Client error: {e}")),
            });
        }
    };

    let payload = serde_json::json!({
        "text": text,
        "character": character,
        "agent_id": agent_id,
    });

    let targets = [
        format!("{endpoint}/speak"),
        format!("{endpoint}/tts"),
        format!("{endpoint}/web-tool/speak"),
    ];

    for target in targets {
        if let Ok(res) = client.post(&target).json(&payload).send().await {
            if res.status().is_success() {
                return Ok(VtuberForwardResult {
                    forwarded: true,
                    error: None,
                });
            }
        }
    }

    Ok(VtuberForwardResult {
        forwarded: false,
        error: Some(format!(
            "Open-LLM-VTuber at {endpoint} unreachable; static avatar active"
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_endpoint_defaults_and_validates() {
        assert_eq!(sanitize_endpoint(None).unwrap(), "http://127.0.0.1:12393");
        assert_eq!(
            sanitize_endpoint(Some("")).unwrap(),
            "http://127.0.0.1:12393"
        );
        assert_eq!(
            sanitize_endpoint(Some("http://localhost:8000/")).unwrap(),
            "http://localhost:8000"
        );
        assert!(sanitize_endpoint(Some("ftp://bad")).is_err());
    }

    #[test]
    fn hub_set_agent_animated_avatar_blocking_fails_on_nonexistent() {
        let res = hub_set_agent_animated_avatar_blocking("nonexistent_agent_xyz", true, None);
        assert!(res.is_err());
    }
}

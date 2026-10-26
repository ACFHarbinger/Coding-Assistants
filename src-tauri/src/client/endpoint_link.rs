//! Connect and disconnect an Orchestrate role to an already-running
//! OpenAI-compatible HTTP service (P8 / U9, issue #88).
//!
//! "Connected" means this role will send chat completions to the saved URL.
//! It does not mean a TCP probe succeeded. Health checks are a later slice.
//! Disconnect keeps the URL draft, stores no process id, and never signals
//! an external server.

use serde::{Deserialize, Serialize};

/// Saved roles that omit this field stay connected, so an existing endpoint
/// keeps its current HTTP path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EndpointLinkState {
    #[default]
    Connected,
    Disconnected,
}

/// Result of connect or disconnect. `owns_process` is always false: this
/// link never represents a child the app is allowed to kill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointLink {
    pub endpoint: String,
    pub state: EndpointLinkState,
    pub owns_process: bool,
}

pub const DISCONNECTED_ENDPOINT: &str =
    "OpenAI-compatible endpoint is disconnected. Connect it before running this role.";

pub const ENDPOINT_URL_ERROR: &str = "OpenAI-compatible endpoint must be an http or https URL with a host and no username or password.";

const MAX_ENDPOINT_LEN: usize = 2048;

/// Where a non-provider-specific endpoint goes before any process spawn.
pub enum AttachedEndpoint<'a> {
    Ready(&'a str),
    Blocked(&'static str),
    None,
}

pub fn validate_openai_compatible_endpoint(raw: &str) -> Result<String, String> {
    let endpoint = raw.trim();
    if endpoint.is_empty() || endpoint.len() > MAX_ENDPOINT_LEN {
        return Err(ENDPOINT_URL_ERROR.to_string());
    }
    if endpoint.chars().any(char::is_control) {
        return Err(ENDPOINT_URL_ERROR.to_string());
    }
    let parsed = tauri::Url::parse(endpoint).map_err(|_| ENDPOINT_URL_ERROR.to_string())?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(ENDPOINT_URL_ERROR.to_string());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ENDPOINT_URL_ERROR.to_string());
    }
    match parsed.host_str() {
        Some(host) if !host.trim().is_empty() => {}
        _ => return Err(ENDPOINT_URL_ERROR.to_string()),
    }
    Ok(endpoint.to_string())
}

/// Accept a local or remote OpenAI-compatible base URL. Localhost and
/// private addresses are allowed. This does not open a socket.
pub fn connect_endpoint(raw: &str) -> Result<EndpointLink, String> {
    Ok(EndpointLink {
        endpoint: validate_openai_compatible_endpoint(raw)?,
        state: EndpointLinkState::Connected,
        owns_process: false,
    })
}

/// Keep the draft address and mark the role disconnected. No process handle
/// is created or signaled.
pub fn disconnect_endpoint(endpoint: &str) -> EndpointLink {
    EndpointLink {
        endpoint: endpoint.trim().to_string(),
        state: EndpointLinkState::Disconnected,
        owns_process: false,
    }
}

pub fn attached_endpoint<'a>(
    endpoint: Option<&'a str>,
    state: EndpointLinkState,
) -> AttachedEndpoint<'a> {
    let Some(endpoint) = endpoint.map(str::trim).filter(|value| !value.is_empty()) else {
        return AttachedEndpoint::None;
    };
    if state == EndpointLinkState::Disconnected {
        return AttachedEndpoint::Blocked(DISCONNECTED_ENDPOINT);
    }
    if validate_openai_compatible_endpoint(endpoint).is_err() {
        return AttachedEndpoint::Blocked(ENDPOINT_URL_ERROR);
    }
    AttachedEndpoint::Ready(endpoint)
}

#[tauri::command]
pub fn endpoint_link_connect(endpoint: String) -> Result<EndpointLink, String> {
    connect_endpoint(&endpoint)
}

#[tauri::command]
pub fn endpoint_link_disconnect(endpoint: String) -> Result<EndpointLink, String> {
    Ok(disconnect_endpoint(&endpoint))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::llm::{LLMClient, ModelConfig};
    use hub::bus::InProcessBus;

    fn config(endpoint: Option<&str>, state: EndpointLinkState) -> ModelConfig {
        ModelConfig {
            provider: "ollama".into(),
            model: "llama3".into(),
            endpoint: endpoint.map(str::to_string),
            endpoint_state: state,
            ..ModelConfig::default()
        }
    }

    #[test]
    fn connect_allows_local_http_and_rejects_other_schemes() {
        let link = connect_endpoint("  http://127.0.0.1:1234/v1  ").unwrap();
        assert_eq!(link.endpoint, "http://127.0.0.1:1234/v1");
        assert_eq!(link.state, EndpointLinkState::Connected);
        assert!(!link.owns_process);
        assert!(connect_endpoint("http://[::1]:8080/v1").is_ok());
        assert!(connect_endpoint("https://example.com/v1").is_ok());
        for raw in [
            "file:///tmp/model",
            "javascript:alert(1)",
            "http://user:secret@127.0.0.1/v1",
            "http://",
            "not a url",
            "",
        ] {
            let error = connect_endpoint(raw).unwrap_err();
            assert_eq!(error, ENDPOINT_URL_ERROR);
            assert!(!error.contains("secret"));
            if !raw.is_empty() {
                assert!(!error.contains(raw));
            }
        }
    }

    #[test]
    fn disconnect_keeps_the_endpoint_and_owns_no_process() {
        let connected = connect_endpoint("http://127.0.0.1:1234/v1").unwrap();
        let disconnected = disconnect_endpoint(&connected.endpoint);
        assert_eq!(disconnected.endpoint, "http://127.0.0.1:1234/v1");
        assert_eq!(disconnected.state, EndpointLinkState::Disconnected);
        assert!(!disconnected.owns_process);
        assert_eq!(
            endpoint_link_disconnect(connected.endpoint).unwrap(),
            disconnected
        );
    }

    #[test]
    fn route_uses_http_only_while_connected() {
        match attached_endpoint(Some("http://127.0.0.1:9/v1"), EndpointLinkState::Connected) {
            AttachedEndpoint::Ready(url) => assert_eq!(url, "http://127.0.0.1:9/v1"),
            _ => panic!("connected endpoint should be ready"),
        }
        match attached_endpoint(
            Some("http://127.0.0.1:9/v1"),
            EndpointLinkState::Disconnected,
        ) {
            AttachedEndpoint::Blocked(message) => assert_eq!(message, DISCONNECTED_ENDPOINT),
            _ => panic!("disconnected endpoint must not fall through"),
        }
        assert!(matches!(
            attached_endpoint(None, EndpointLinkState::Disconnected),
            AttachedEndpoint::None
        ));
        assert!(matches!(
            attached_endpoint(Some("   "), EndpointLinkState::Connected),
            AttachedEndpoint::None
        ));
        assert!(matches!(
            attached_endpoint(Some("file:///tmp/x"), EndpointLinkState::Connected),
            AttachedEndpoint::Blocked(ENDPOINT_URL_ERROR)
        ));
    }

    #[test]
    fn missing_state_stays_connected_for_saved_roles() {
        let parsed: ModelConfig = serde_json::from_str(
            r#"{"provider":"ollama","model":"llama3","endpoint":"http://127.0.0.1:11434/v1"}"#,
        )
        .unwrap();
        assert_eq!(parsed.endpoint_state, EndpointLinkState::Connected);
        assert!(matches!(
            attached_endpoint(parsed.endpoint.as_deref(), parsed.endpoint_state),
            AttachedEndpoint::Ready("http://127.0.0.1:11434/v1")
        ));
    }

    #[tokio::test]
    async fn disconnected_role_errors_before_any_provider_spawn() {
        let client = LLMClient::new();
        let bus = InProcessBus::new();
        let error = client
            .chat_completion(
                &config(
                    Some("http://127.0.0.1:9/v1"),
                    EndpointLinkState::Disconnected,
                ),
                "hello",
                None,
                &bus,
                "Developer",
                None,
                None,
            )
            .await
            .unwrap_err();
        assert_eq!(error, DISCONNECTED_ENDPOINT);
    }
}

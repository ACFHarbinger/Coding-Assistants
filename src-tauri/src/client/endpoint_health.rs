//! Reachability probe for a connected OpenAI-compatible endpoint (#88).
//!
//! GET `{base}/models` with a short timeout. A key saved for this address
//! is sent as `Authorization: Bearer`. No key sends no credential.
//! A disconnected, blank, or non-http(s) address returns a result and does
//! not open a socket. This never starts or signals a process.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::client::endpoint_auth::http_api_key;
use crate::client::endpoint_link::{
    validate_openai_compatible_endpoint, EndpointLinkState, ENDPOINT_URL_ERROR,
};

const HEALTH_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EndpointHealthKind {
    Reachable,
    AuthenticationRequired,
    Unexpected,
    Unreachable,
    NotProbed,
}

/// Structured probe result. `message` is safe to show: it never includes the
/// URL, a response body, or a credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointHealth {
    pub kind: EndpointHealthKind,
    pub status: Option<u16>,
    pub message: String,
}

#[tauri::command]
pub async fn endpoint_link_health(
    endpoint: String,
    endpoint_state: EndpointLinkState,
) -> EndpointHealth {
    check_endpoint_health(&endpoint, endpoint_state).await
}

pub async fn check_endpoint_health(endpoint: &str, state: EndpointLinkState) -> EndpointHealth {
    check_endpoint_health_within(endpoint, state, HEALTH_TIMEOUT).await
}

pub async fn check_endpoint_health_within(
    endpoint: &str,
    state: EndpointLinkState,
    timeout: Duration,
) -> EndpointHealth {
    if state == EndpointLinkState::Disconnected {
        return not_probed("Disconnected. Reachability was not checked.");
    }
    let endpoint = match validate_openai_compatible_endpoint(endpoint) {
        Ok(endpoint) => endpoint,
        Err(_) => return not_probed(ENDPOINT_URL_ERROR),
    };
    let Some(url) = models_url(&endpoint) else {
        return not_probed(ENDPOINT_URL_ERROR);
    };
    let secret = http_api_key(false, &endpoint);
    let bearer = secret.as_ref().map(|value| value.expose());
    match send_models_get(&url, bearer, timeout).await {
        Ok(status) => classify_status(status),
        Err(ProbeFailure::Local) => unreachable_message("could not reach the endpoint"),
        Err(ProbeFailure::Transport(error)) => classify_transport(&error),
    }
}

fn not_probed(message: &str) -> EndpointHealth {
    EndpointHealth {
        kind: EndpointHealthKind::NotProbed,
        status: None,
        message: message.to_string(),
    }
}

fn unreachable_message(detail: &str) -> EndpointHealth {
    EndpointHealth {
        kind: EndpointHealthKind::Unreachable,
        status: None,
        message: format!("Unreachable. {detail}."),
    }
}

/// `{base}/models` with any query, fragment, or userinfo removed.
pub fn models_url(base: &str) -> Option<String> {
    let mut url = tauri::Url::parse(base.trim()).ok()?;
    let path = url.path().trim_end_matches('/').to_string();
    url.set_path(&format!("{path}/models"));
    url.set_query(None);
    url.set_fragment(None);
    url.set_username("").ok()?;
    url.set_password(None).ok()?;
    Some(url.to_string())
}

pub fn classify_status(status: u16) -> EndpointHealth {
    if (200..300).contains(&status) {
        return EndpointHealth {
            kind: EndpointHealthKind::Reachable,
            status: Some(status),
            message: "Reachable.".to_string(),
        };
    }
    if status == 401 || status == 403 {
        return EndpointHealth {
            kind: EndpointHealthKind::AuthenticationRequired,
            status: Some(status),
            message: "Reachable. Authentication required.".to_string(),
        };
    }
    EndpointHealth {
        kind: EndpointHealthKind::Unexpected,
        status: Some(status),
        message: format!("Reachable, but the server returned HTTP {status}."),
    }
}

/// Map a transport failure to a short static phrase. The reqwest display
/// string is not copied: it includes the request URL.
pub fn classify_transport(error: &reqwest::Error) -> EndpointHealth {
    if error.is_redirect() {
        return EndpointHealth {
            kind: EndpointHealthKind::Unexpected,
            status: None,
            message: "Reachable, but the server returned an HTTP redirect.".to_string(),
        };
    }
    classify_transport_detail(error.is_timeout(), &transport_chain(error))
}

/// `chain` is searched for a failure class and then discarded. It is not
/// copied into the result, because reqwest includes the request URL.
pub fn classify_transport_detail(timed_out: bool, chain: &str) -> EndpointHealth {
    let detail = if timed_out {
        "timed out"
    } else {
        let lower = chain.to_ascii_lowercase();
        if lower.contains("connection refused") || lower.contains("actively refused") {
            "connection refused"
        } else if lower.contains("dns")
            || lower.contains("lookup address")
            || lower.contains("name or service not known")
            || lower.contains("no such host")
            || lower.contains("nodename nor servname")
        {
            "DNS lookup failed"
        } else {
            "could not reach the endpoint"
        }
    };
    unreachable_message(detail)
}

fn transport_chain(error: &reqwest::Error) -> String {
    let mut parts = Vec::new();
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(source) = current {
        parts.push(source.to_string());
        current = source.source();
    }
    parts.join(" ")
}

enum ProbeFailure {
    Local,
    Transport(reqwest::Error),
}

async fn send_models_get(
    url: &str,
    bearer: Option<&str>,
    timeout: Duration,
) -> Result<u16, ProbeFailure> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(2))
        .build()
        .map_err(|_| ProbeFailure::Local)?;
    let request = client.get(url);
    let request = match bearer.map(str::trim).filter(|value| !value.is_empty()) {
        Some(bearer) => request.bearer_auth(bearer),
        None => request,
    };
    let response = request.send().await.map_err(ProbeFailure::Transport)?;
    Ok(response.status().as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    fn spawn_status(status: u16) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut buf = Vec::new();
            let mut tmp = [0_u8; 1024];
            loop {
                match stream.read(&mut tmp) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&tmp[..n]);
                        if buf.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&buf).into_owned());
            let body = format!(
                "HTTP/1.1 {status} STATUS\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(body.as_bytes());
        });
        (format!("http://{addr}/v1"), rx)
    }

    fn assert_models_get_without_credentials(request: &str) {
        let lower = request.to_ascii_lowercase();
        assert!(lower.starts_with("get /v1/models http/1.1"), "{request}");
        assert!(!lower.contains("authorization"));
        assert!(!lower.contains("api-key"));
        assert!(!lower.contains("bearer"));
    }

    #[test]
    fn models_url_appends_models_and_drops_query_secrets() {
        assert_eq!(
            models_url("http://127.0.0.1:1234/v1").as_deref(),
            Some("http://127.0.0.1:1234/v1/models")
        );
        assert_eq!(
            models_url("http://127.0.0.1:1234/v1/").as_deref(),
            Some("http://127.0.0.1:1234/v1/models")
        );
        let stripped = models_url("http://127.0.0.1:1234/v1?api_key=sekret").unwrap();
        assert_eq!(stripped, "http://127.0.0.1:1234/v1/models");
        assert!(!stripped.contains("sekret"));
    }

    #[test]
    fn status_classifier_separates_success_auth_and_unexpected() {
        assert_eq!(classify_status(200).kind, EndpointHealthKind::Reachable);
        assert_eq!(classify_status(204).kind, EndpointHealthKind::Reachable);
        assert_eq!(
            classify_status(401).kind,
            EndpointHealthKind::AuthenticationRequired
        );
        assert_eq!(
            classify_status(403).kind,
            EndpointHealthKind::AuthenticationRequired
        );
        let unexpected = classify_status(503);
        assert_eq!(unexpected.kind, EndpointHealthKind::Unexpected);
        assert_eq!(unexpected.status, Some(503));
        assert!(unexpected.message.contains("503"));
    }

    #[test]
    fn transport_classifier_uses_static_phrases_only() {
        let refused = classify_transport_detail(false, "connection refused (os error 111)");
        assert_eq!(refused.kind, EndpointHealthKind::Unreachable);
        assert!(refused.message.contains("connection refused"));
        assert!(classify_transport_detail(true, "http://secret.example/v1")
            .message
            .contains("timed out"));
        let dns = classify_transport_detail(false, "dns lookup failed: no such host");
        assert!(dns.message.contains("DNS lookup failed"));
        let leaked = classify_transport_detail(
            false,
            "error for url (http://user:secret@127.0.0.1/v1?api_key=sekret): connection refused",
        );
        assert!(!leaked.message.contains("secret"));
        assert!(!leaked.message.contains("sekret"));
        assert!(!leaked.message.contains("http"));
    }

    #[tokio::test]
    async fn disconnected_or_invalid_does_not_open_a_socket() {
        let hits = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let hits_thread = Arc::clone(&hits);
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                hits_thread.fetch_add(1, Ordering::SeqCst);
                let body = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(body.as_bytes());
            }
        });
        let url = format!("http://{addr}/v1");
        let disconnected = tokio::time::timeout(
            Duration::from_millis(500),
            check_endpoint_health(&url, EndpointLinkState::Disconnected),
        )
        .await
        .expect("disconnected probe must not wait on the network");
        assert_eq!(disconnected.kind, EndpointHealthKind::NotProbed);
        assert!(disconnected.message.contains("Disconnected"));

        let invalid =
            check_endpoint_health("file:///tmp/model", EndpointLinkState::Connected).await;
        assert_eq!(invalid.kind, EndpointHealthKind::NotProbed);
        assert_eq!(invalid.message, ENDPOINT_URL_ERROR);
        let blank = check_endpoint_health("  ", EndpointLinkState::Connected).await;
        assert_eq!(blank.kind, EndpointHealthKind::NotProbed);
        assert_eq!(hits.load(Ordering::SeqCst), 0);
        let _ = std::net::TcpStream::connect(addr);
    }

    #[tokio::test]
    async fn loopback_classifies_success_auth_and_unexpected() {
        for (status, kind) in [
            (200, EndpointHealthKind::Reachable),
            (401, EndpointHealthKind::AuthenticationRequired),
            (403, EndpointHealthKind::AuthenticationRequired),
            (404, EndpointHealthKind::Unexpected),
        ] {
            let (url, request) = spawn_status(status);
            let health = check_endpoint_health(&url, EndpointLinkState::Connected).await;
            assert_eq!(health.kind, kind, "{status}");
            assert_eq!(health.status, Some(status));
            let captured = request.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_models_get_without_credentials(&captured);
            if kind == EndpointHealthKind::Unexpected {
                assert!(health.message.contains(&status.to_string()));
            }
        }
    }

    #[tokio::test]
    async fn connection_refused_is_unreachable() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let url = format!("http://127.0.0.1:{port}/v1");
        let health = check_endpoint_health(&url, EndpointLinkState::Connected).await;
        assert_eq!(health.kind, EndpointHealthKind::Unreachable);
        assert!(
            health.message.contains("connection refused"),
            "{}",
            health.message
        );
        assert!(!health.message.contains(&url));
    }

    #[tokio::test]
    async fn stalled_peer_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf);
            let mut one = [0_u8; 1];
            let _ = stream.read(&mut one);
        });
        let url = format!("http://{addr}/v1");
        let health = tokio::time::timeout(
            Duration::from_secs(2),
            check_endpoint_health_within(
                &url,
                EndpointLinkState::Connected,
                Duration::from_millis(200),
            ),
        )
        .await
        .expect("health timeout must follow the caller timeout");
        assert_eq!(health.kind, EndpointHealthKind::Unreachable);
        assert!(health.message.contains("timed out"), "{}", health.message);
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn saved_key_is_sent_as_bearer_and_omitted_from_the_result() {
        let _lock = super::super::endpoint_auth::tests_env_lock();
        let dir = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("CA_HOME");
        let previous_backend = std::env::var_os("CA_SECRET_BACKEND");
        std::env::set_var("CA_HOME", dir.path());
        std::env::set_var("CA_SECRET_BACKEND", "file");
        let (url, request) = spawn_status(200);
        let key = "endpoint-bearer-token";
        super::super::endpoint_auth::save_endpoint_api_key(&url, key).unwrap();
        let health = check_endpoint_health(&url, EndpointLinkState::Connected).await;
        let captured = request.recv_timeout(Duration::from_secs(2)).unwrap();
        let lower = captured.to_ascii_lowercase();
        assert!(lower.contains("authorization: bearer endpoint-bearer-token"));
        assert_eq!(health.kind, EndpointHealthKind::Reachable);
        assert!(!health.message.contains(key));
        match previous_home {
            Some(value) => std::env::set_var("CA_HOME", value),
            None => std::env::remove_var("CA_HOME"),
        }
        match previous_backend {
            Some(value) => std::env::set_var("CA_SECRET_BACKEND", value),
            None => std::env::remove_var("CA_SECRET_BACKEND"),
        }
    }
}

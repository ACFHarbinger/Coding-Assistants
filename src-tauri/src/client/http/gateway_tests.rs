use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn serve_script(responses: Vec<Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for body in responses {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(&body);
            }
        }
    });
    format!("http://{address}/v1")
}

fn json_response(status: &str, json: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
        json.len()
    )
    .into_bytes()
}

const OK_JSON: &str = r#"{"id":"gen-1","object":"chat.completion","created":1,"model":"openai/gpt-4o-mini","choices":[{"index":0,"message":{"role":"assistant","content":"via openrouter"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":4,"total_tokens":7,"cost":0.0015}}"#;

#[test]
fn split_fallback_models_drops_empties_and_defaults() {
    assert_eq!(
        split_fallback_models("openai/gpt-4o, anthropic/claude-3.5-sonnet"),
        vec![
            "openai/gpt-4o".to_string(),
            "anthropic/claude-3.5-sonnet".to_string()
        ]
    );
    assert_eq!(
        split_fallback_models("  ,  "),
        vec![OPENROUTER_DEFAULT_MODEL.to_string()]
    );
    assert_eq!(
        split_fallback_models(""),
        vec![OPENROUTER_DEFAULT_MODEL.to_string()]
    );
}

#[test]
fn usage_from_openrouter_json_reads_tokens_and_cost() {
    let usage = serde_json::json!({
        "prompt_tokens": 10,
        "completion_tokens": 20,
        "total_tokens": 30,
        "cost": 0.0025
    });
    let (tokens, cost) = usage_from_openrouter_json(&usage).unwrap();
    assert_eq!(tokens.prompt_tokens, 10);
    assert_eq!(tokens.completion_tokens, 20);
    assert_eq!(tokens.total_tokens, 30);
    assert_eq!(cost, Some(0.0025));
    assert!(usage_from_openrouter_json(&serde_json::json!({"prompt_tokens": 1})).is_none());
}

#[test]
fn openrouter_provider_and_auth_are_presence_only() {
    assert!(is_openrouter_provider("openrouter"));
    assert!(is_openrouter_provider(" openrouter "));
    assert!(!is_openrouter_provider("openai"));
    assert!(!openrouter_is_authenticated(None));
    assert!(!openrouter_is_authenticated(Some("  ")));
    assert!(openrouter_is_authenticated(Some(
        "presence-flag-not-a-secret"
    )));
}

#[tokio::test]
async fn openrouter_chat_reads_content_usage_and_cost() {
    let base = serve_script(vec![json_response("200 OK", OK_JSON)]);
    let result = openrouter_chat_to(
        &openrouter_chat_url(&base),
        "presence-flag-not-a-secret",
        "openai/gpt-4o-mini",
        "; rm -rf / && echo pwned $(whoami)",
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    assert_eq!(result.text, "via openrouter");
    let usage = result.usage.unwrap();
    assert_eq!(usage.total_tokens, 7);
    assert_eq!(result.cost, Some(0.0015));
}

#[tokio::test]
async fn fallback_chain_skips_a_retryable_model() {
    let err = r#"{"error":{"message":"model overloaded","type":"server_error"}}"#;
    let base = serve_script(vec![
        json_response("503 Service Unavailable", err),
        json_response("200 OK", OK_JSON),
    ]);
    let models = vec![
        "openai/gpt-4o".to_string(),
        "openai/gpt-4o-mini".to_string(),
    ];
    let result = chat_with_fallback(
        &base,
        "presence-flag-not-a-secret",
        &models,
        "hi",
        Duration::from_secs(5),
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.text, "via openrouter");
}

#[tokio::test]
async fn fallback_chain_stops_on_cancel() {
    let token = Arc::new(AtomicBool::new(true));
    let error = chat_with_fallback(
        "http://127.0.0.1:1/v1",
        "presence-flag-not-a-secret",
        &["openai/gpt-4o".to_string()],
        "hi",
        Duration::from_secs(5),
        Some(token),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), "cancelled");
}

#[tokio::test]
async fn openrouter_offline_host_fails_fast() {
    let error = openrouter_chat_to(
        "http://127.0.0.1:1/chat/completions",
        "presence-flag-not-a-secret",
        "openai/gpt-4o-mini",
        "hi",
        Duration::from_secs(5),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), "transport");
}

/// Records the request body so we can prove shell metacharacters stayed data.
fn serve_and_capture() -> (String, Arc<Mutex<String>>) {
    let captured = Arc::new(Mutex::new(String::new()));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let captured_clone = captured.clone();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = vec![0u8; 8192];
            if let Ok(n) = stream.read(&mut buf) {
                *captured_clone.lock().unwrap() = String::from_utf8_lossy(&buf[..n]).into();
            }
            let _ = stream.write_all(&json_response("200 OK", OK_JSON));
        }
    });
    (format!("http://{address}/v1"), captured)
}

#[tokio::test]
async fn openrouter_request_keeps_prompt_as_json_data() {
    let dangerous = "; rm -rf / && echo pwned $(whoami)";
    let (base, captured) = serve_and_capture();
    let _ = openrouter_chat_to(
        &openrouter_chat_url(&base),
        "presence-flag-not-a-secret",
        "openai/gpt-4o-mini",
        dangerous,
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    let request = captured.lock().unwrap().clone();
    assert!(request.contains(dangerous), "prompt missing: {request}");
    assert!(request.contains("HTTP-Referer") || request.contains("http-referer"));
}

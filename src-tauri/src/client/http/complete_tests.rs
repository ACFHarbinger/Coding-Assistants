use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn serve_once(body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(&body);
        }
    });
    format!("http://{address}")
}

fn json_response(status: &str, json: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
        json.len()
    )
    .into_bytes()
}

fn sse_response(events: &[&str]) -> Vec<u8> {
    let mut body = String::new();
    for event in events {
        body.push_str("data: ");
        body.push_str(event);
        body.push_str("\n\n");
    }
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n{body}"
    )
    .into_bytes()
}

const OK_JSON: &str = r#"{"id":"chatcmpl-1","object":"chat.completion","created":1,"model":"gpt-4o","choices":[{"index":0,"message":{"role":"assistant","content":"working on P4"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":4,"total_tokens":7}}"#;

#[test]
fn normalize_api_base_adds_v1_and_strips_slash() {
    assert_eq!(
        normalize_api_base("https://api.openai.com/v1"),
        "https://api.openai.com/v1"
    );
    assert_eq!(
        normalize_api_base("https://api.openai.com/v1/"),
        "https://api.openai.com/v1"
    );
    assert_eq!(
        normalize_api_base("http://127.0.0.1:1234"),
        "http://127.0.0.1:1234/v1"
    );
    assert_eq!(normalize_api_base("  "), OPENAI_DEFAULT_BASE);
}

#[test]
fn token_usage_json_is_secret_free() {
    let usage = TokenUsage {
        prompt_tokens: 3,
        completion_tokens: 4,
        total_tokens: 7,
    };
    let json = usage.to_json();
    assert_eq!(json["prompt_tokens"], 3);
    assert_eq!(json["completion_tokens"], 4);
    assert_eq!(json["total_tokens"], 7);
}

#[tokio::test]
async fn chat_reads_content_and_usage() {
    let base = serve_once(json_response("200 OK", OK_JSON));
    let result = chat(
        &base,
        "presence-flag-not-a-secret",
        "gpt-4o",
        "hi",
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    assert_eq!(result.text, "working on P4");
    let usage = result.usage.unwrap();
    assert_eq!(usage.prompt_tokens, 3);
    assert_eq!(usage.completion_tokens, 4);
    assert_eq!(usage.total_tokens, 7);
}

#[tokio::test]
async fn chat_maps_api_error_without_retry() {
    let json = r#"{"error":{"message":"Incorrect API key provided","type":"invalid_request_error","param":null,"code":"invalid_api_key"}}"#;
    let base = serve_once(json_response("401 Unauthorized", json));
    let error = chat(
        &base,
        "presence-flag-not-a-secret",
        "gpt-4o",
        "hi",
        Duration::from_secs(5),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), "api");
    assert!(error.to_string().contains("Incorrect API key provided"));
    assert!(!error.to_string().contains("sk-"));
}

#[tokio::test]
async fn chat_offline_host_fails_fast_without_retry() {
    let error = chat(
        "http://127.0.0.1:1",
        "presence-flag-not-a-secret",
        "gpt-4o",
        "hi",
        Duration::from_secs(5),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), "transport");
}

#[tokio::test]
async fn chat_hung_host_hits_the_request_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            let _held = stream;
            thread::sleep(Duration::from_secs(30));
        }
    });
    let started = std::time::Instant::now();
    let error = chat(
        &format!("http://{address}"),
        "presence-flag-not-a-secret",
        "gpt-4o",
        "hi",
        Duration::from_millis(300),
    )
    .await
    .unwrap_err();
    assert!(
        started.elapsed() >= Duration::from_millis(250),
        "error came too fast to be the timeout: {error}"
    );
    assert_eq!(error.code(), "timeout");
}

#[tokio::test]
async fn chat_stream_concatenates_deltas_and_reads_usage() {
    let chunk1 = r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":"hel"},"finish_reason":null}]}"#;
    let chunk2 = r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"lo"},"finish_reason":"stop"}]}"#;
    let usage = r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}}"#;
    let base = serve_once(sse_response(&[chunk1, chunk2, usage, "[DONE]"]));
    let mut deltas = Vec::new();
    let result = chat_stream(
        &base,
        "presence-flag-not-a-secret",
        "gpt-4o",
        "; rm -rf / && echo pwned $(whoami)",
        Duration::from_secs(5),
        None,
        |delta| deltas.push(delta.to_string()),
    )
    .await
    .unwrap();
    assert_eq!(deltas, vec!["hel", "lo"]);
    assert_eq!(result.text, "hello");
    let usage = result.usage.unwrap();
    assert_eq!(usage.total_tokens, 5);
}

#[tokio::test]
async fn chat_stream_honours_cancel_token() {
    let token = Arc::new(AtomicBool::new(true));
    let error = chat_stream(
        "http://127.0.0.1:1",
        "presence-flag-not-a-secret",
        "gpt-4o",
        "hi",
        Duration::from_secs(5),
        Some(token),
        |_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), "cancelled");
}

//! Muse Spark chat completion over reqwest.
//!
//! The orchestration `AtomicBool` is the same cancel flag as the other HTTP
//! completions. A flag that is already set never opens a socket. A flag that
//! flips while the call is in flight drops the reqwest future, which aborts
//! the connection. The assistant body is published only after a finished call
//! that is still not cancelled.

use crate::agent::AgentEvent;
use crate::client::providers::{
    muse_chat_completions_url, muse_request_body, muse_response_text, muse_unavailable_request,
    MUSE_REQUEST_TIMEOUT_SECS,
};
use hub::bus::{InProcessBus, TOPIC_AGENT_EVENT};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const CANCEL_POLL: Duration = Duration::from_millis(50);
const CANCELLED: &str = "Task cancelled";

fn cancelled(token: Option<&Arc<AtomicBool>>) -> bool {
    token.is_some_and(|flag| flag.load(Ordering::SeqCst))
}

async fn until_cancelled(token: Arc<AtomicBool>) {
    loop {
        if token.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(CANCEL_POLL).await;
    }
}

/// Run `work` until it finishes or the cancel flag is set. Dropping `work`
/// drops the reqwest request or body future and aborts the socket.
async fn with_cancel<T>(
    token: Option<&Arc<AtomicBool>>,
    work: impl std::future::Future<Output = Result<T, reqwest::Error>>,
) -> Result<T, String> {
    if cancelled(token) {
        return Err(CANCELLED.to_string());
    }
    let Some(flag) = token.cloned() else {
        return work.await.map_err(muse_unavailable_request);
    };
    let watched = flag.clone();
    tokio::pin!(work);
    tokio::select! {
        biased;
        () = until_cancelled(watched) => Err(CANCELLED.to_string()),
        result = &mut work => {
            if cancelled(Some(&flag)) {
                Err(CANCELLED.to_string())
            } else {
                result.map_err(muse_unavailable_request)
            }
        }
    }
}

pub(crate) async fn muse_chat_request(
    api_key: &str,
    model: &str,
    prompt: &str,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    muse_chat_request_to(
        &muse_chat_completions_url(),
        api_key,
        model,
        prompt,
        Duration::from_secs(MUSE_REQUEST_TIMEOUT_SECS),
        token,
    )
    .await
}

pub(crate) async fn muse_chat_request_to(
    url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    timeout: Duration,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    if cancelled(token.as_ref()) {
        return Err(CANCELLED.to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(muse_unavailable_request)?;
    let response = with_cancel(
        token.as_ref(),
        client
            .post(url)
            .bearer_auth(api_key)
            .json(&muse_request_body(model, prompt))
            .send(),
    )
    .await?;
    if cancelled(token.as_ref()) {
        return Err(CANCELLED.to_string());
    }
    let status = response.status();
    let body = with_cancel(token.as_ref(), response.text()).await?;
    if cancelled(token.as_ref()) {
        return Err(CANCELLED.to_string());
    }
    if !status.is_success() {
        return Err(format!("Muse Model API returned {status}: {body}"));
    }
    muse_response_text(&body)
}

fn publish_response(bus: &InProcessBus, source: &str, output: &str) {
    bus.emit(
        TOPIC_AGENT_EVENT,
        AgentEvent {
            source: source.to_string(),
            event_type: "response".to_string(),
            content: output.to_string(),
        },
    );
}

pub(crate) async fn muse_chat_turn(
    api_key: &str,
    model: &str,
    prompt: &str,
    bus: &InProcessBus,
    source: &str,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    let output = muse_chat_request(api_key, model, prompt, token.clone()).await?;
    if cancelled(token.as_ref()) {
        return Err(CANCELLED.to_string());
    }
    publish_response(bus, source, &output);
    Ok(output)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn muse_chat_turn_to(
    url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    timeout: Duration,
    bus: &InProcessBus,
    source: &str,
    token: Option<Arc<AtomicBool>>,
) -> Result<String, String> {
    let output = muse_chat_request_to(url, api_key, model, prompt, timeout, token.clone()).await?;
    if cancelled(token.as_ref()) {
        return Err(CANCELLED.to_string());
    }
    publish_response(bus, source, &output);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub::EventBus;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    fn events(bus: &InProcessBus) -> hub::bus::BusReceiver {
        bus.subscribe()
    }

    #[tokio::test]
    async fn muse_offline_host_fails_fast_without_retry() {
        let error = muse_chat_request_to(
            "http://127.0.0.1:1/chat/completions",
            "presence-flag-not-a-secret",
            "muse-spark-1.3",
            "hi",
            Duration::from_secs(5),
            None,
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
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let _held = stream;
                thread::sleep(Duration::from_secs(30));
            }
        });
        let started = Instant::now();
        let error = muse_chat_request_to(
            &format!("http://{address}/chat/completions"),
            "presence-flag-not-a-secret",
            "muse-spark-1.3",
            "hi",
            Duration::from_millis(300),
            None,
        )
        .await
        .unwrap_err();
        assert!(
            started.elapsed() >= Duration::from_millis(250),
            "error came too fast to be the timeout: {error}"
        );
        assert!(
            error.contains("Model API request failed"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn preset_cancel_does_not_open_or_emit() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (accepted_tx, accepted_rx) = mpsc::channel();
        thread::spawn(move || {
            if listener.accept().is_ok() {
                let _ = accepted_tx.send(());
            }
        });
        let bus = InProcessBus::new();
        let rx = events(&bus);
        let started = Instant::now();
        let error = muse_chat_turn_to(
            &format!("http://{address}/chat/completions"),
            "presence-flag-not-a-secret",
            "muse-spark-1.3",
            "hi",
            Duration::from_secs(5),
            &bus,
            "Muse",
            Some(Arc::new(AtomicBool::new(true))),
        )
        .await
        .unwrap_err();
        assert_eq!(error, CANCELLED);
        assert!(
            started.elapsed() < Duration::from_millis(200),
            "cancelled call still waited: {error}"
        );
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(
            accepted_rx.try_recv().is_err(),
            "pre-set cancel opened a socket"
        );
        assert!(
            rx.try_recv().is_err(),
            "pre-set cancel published a response"
        );
    }

    #[tokio::test]
    async fn inflight_cancel_aborts_without_publishing_the_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let token = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&token);
        let (closed_tx, closed_rx) = tokio::sync::oneshot::channel();
        // The server thread sets the flag only after it has the request.
        // The client stays on this task so a blocking wait cannot stall it.
        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buf = [0u8; 1024];
            let mut collected = Vec::new();
            loop {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => {
                        collected.extend_from_slice(&buf[..n]);
                        if collected.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                }
            }
            flag.store(true, Ordering::SeqCst);
            loop {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        let _ = closed_tx.send(());
                        break;
                    }
                    Ok(_) => {}
                }
            }
        });
        let bus = InProcessBus::new();
        let rx = events(&bus);
        let started = Instant::now();
        let error = muse_chat_turn_to(
            &format!("http://{address}/chat/completions"),
            "presence-flag-not-a-secret",
            "muse-spark-1.3",
            "hi",
            Duration::from_secs(8),
            &bus,
            "Muse",
            Some(token),
        )
        .await
        .unwrap_err();
        assert_eq!(error, CANCELLED);
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "cancel did not abort the stalled call: {error}"
        );
        tokio::time::timeout(Duration::from_secs(2), closed_rx)
            .await
            .expect("dropping the reqwest future did not close the socket")
            .expect("server lost the close signal");
        assert!(rx.try_recv().is_err(), "aborted call published a body");
    }

    #[tokio::test]
    async fn finished_call_still_publishes_when_not_cancelled() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"spark"}}]}"#;
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        let bus = InProcessBus::new();
        let rx = events(&bus);
        let text = muse_chat_turn_to(
            &format!("http://{address}/chat/completions"),
            "presence-flag-not-a-secret",
            "muse-spark-1.3",
            "hi",
            Duration::from_secs(5),
            &bus,
            "Muse",
            None,
        )
        .await
        .unwrap();
        assert_eq!(text, "spark");
        let event = rx.try_recv().unwrap();
        assert_eq!(event.payload["event_type"], "response");
        assert_eq!(event.payload["content"], "spark");
        assert_eq!(event.payload["source"], "Muse");
    }
}

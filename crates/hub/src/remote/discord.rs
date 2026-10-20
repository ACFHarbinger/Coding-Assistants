//! Discord Bot REST transport. Outbound channel-history polling only — no
//! gateway/websocket, no listen port (U25 / #319). Mirrors the Telegram v1
//! remote-control contract: pairing allow-list, `/approve` `/reject`
//! `/send` `/wakes`, unknown senders silent, HubStore writes as `human`.
//!
//! Owner setup (once): create a bot in the Discord Developer Portal, enable
//! the **Message Content** intent, invite it to the guild with `View
//! Channel` + `Send Messages` + `Read Message History` on the bound
//! channel, and set `DISCORD_BOT_TOKEN` (P12 `tool.discord.bot_token`).
//! Commands are plain channel messages (`/start <code>`, `/wakes`, ...),
//! not registered Discord slash commands.

use super::command::{parse_inbound, Inbound};
use super::dispatch::{self, Reply};
use crate::secret;
use crate::store::HubStore;
use serde::Deserialize;
use std::fmt;
use std::time::Duration;

pub use super::discord_binding::BindingStore;

const API: &str = "https://discord.com/api/v10";
const TOKEN_KEY: &str = "DISCORD_BOT_TOKEN";
/// Discord rejects message content over 2000 chars; leave headroom.
const MAX_MESSAGE_CHARS: usize = 1900;
/// Backoff when a 429 body carries no usable `retry_after`.
const DEFAULT_RETRY: Duration = Duration::from_secs(5);

/// Discord sender identity in a guild channel. Snowflake ids are u64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordUser {
    pub user_id: u64,
    pub channel_id: u64,
    pub username: Option<String>,
}

/// Failure of one REST call. [`ApiError::RateLimited`] carries Discord's
/// `retry_after` so the caller backs off instead of hammering the API.
#[derive(Debug)]
pub enum ApiError {
    RateLimited(Duration),
    Failed(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimited(wait) => write!(f, "discord rate limited for {wait:?}"),
            Self::Failed(error) => f.write_str(error),
        }
    }
}

/// Resolve the bot token (vault wins, env fallback). Never log the value.
pub fn resolve_bot_token() -> Option<secret::SecretString> {
    secret::resolve(TOKEN_KEY)
}

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(deserialize_with = "snowflake")]
    id: u64,
    author: Author,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Author {
    #[serde(deserialize_with = "snowflake")]
    id: u64,
    username: Option<String>,
    #[serde(default)]
    bot: bool,
}

/// Snowflakes arrive as JSON strings; accept a bare number too.
fn snowflake<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(text) => text.parse().map_err(serde::de::Error::custom),
        serde_json::Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("snowflake out of range")),
        other => Err(serde::de::Error::custom(format!(
            "not a snowflake: {other}"
        ))),
    }
}

/// One polled batch: inbound `(sender, text)` pairs (oldest first) plus the
/// newest snowflake seen, for advancing the channel cursor.
pub type MessageBatch = (Vec<(DiscordUser, String)>, Option<u64>);

/// Inbound channel texts (oldest first) plus the newest snowflake seen.
/// Bot authors (including this bot's own replies) are skipped.
pub fn parse_messages(body: &str, channel_id: u64) -> Result<MessageBatch, String> {
    let messages: Vec<Message> =
        serde_json::from_str(body).map_err(|error| format!("discord json: {error}"))?;
    let max_seen = messages.iter().map(|message| message.id).max();
    let mut rows: Vec<(u64, DiscordUser, String)> = messages
        .into_iter()
        .filter(|message| !message.author.bot)
        .filter_map(|message| {
            let text = message.content?.trim().to_string();
            if text.is_empty() {
                return None;
            }
            Some((
                message.id,
                DiscordUser {
                    user_id: message.author.id,
                    channel_id,
                    username: message.author.username,
                },
                text,
            ))
        })
        .collect();
    // Discord returns newest-first; process oldest-first like getUpdates.
    rows.sort_by_key(|(id, _, _)| *id);
    Ok((
        rows.into_iter()
            .map(|(_, user, text)| (user, text))
            .collect(),
        max_seen,
    ))
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Extract Discord's `retry_after` (seconds, float) from a 429 body.
fn retry_after(response: &mut ureq::http::Response<ureq::Body>) -> Duration {
    let body = response
        .body_mut()
        .read_to_string()
        .unwrap_or_else(|_| String::new());
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| value.get("retry_after")?.as_f64())
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .map(Duration::from_secs_f64)
        .unwrap_or(DEFAULT_RETRY)
}

/// Poll one channel's history after `after` (a message snowflake).
pub fn fetch_messages(
    token: &str,
    channel_id: u64,
    after: Option<u64>,
) -> Result<MessageBatch, ApiError> {
    fetch_messages_from(API, token, channel_id, after)
}

fn fetch_messages_from(
    api_base: &str,
    token: &str,
    channel_id: u64,
    after: Option<u64>,
) -> Result<MessageBatch, ApiError> {
    let mut url = format!("{api_base}/channels/{channel_id}/messages?limit=100");
    if let Some(after) = after {
        url.push_str(&format!("&after={after}"));
    }
    let mut response = agent(Duration::from_secs(20))
        .get(&url)
        .header("Authorization", format!("Bot {token}"))
        .call()
        .map_err(|error| ApiError::Failed(format!("discord get messages: {error}")))?;
    let status = response.status().as_u16();
    if status == 429 {
        return Err(ApiError::RateLimited(retry_after(&mut response)));
    }
    if !(200..300).contains(&status) {
        return Err(ApiError::Failed(format!(
            "discord get messages: http {status}"
        )));
    }
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| ApiError::Failed(format!("discord get messages body: {error}")))?;
    parse_messages(&body, channel_id).map_err(ApiError::Failed)
}

/// Send a channel reply. Retries once after Discord's own `retry_after`
/// when the first attempt is rate limited.
pub fn send_message(token: &str, channel_id: u64, text: &str) -> Result<(), ApiError> {
    send_message_to(API, token, channel_id, text)
}

fn send_message_to(
    api_base: &str,
    token: &str,
    channel_id: u64,
    text: &str,
) -> Result<(), ApiError> {
    let url = format!("{api_base}/channels/{channel_id}/messages");
    let body = serde_json::json!({ "content": truncate(text) });
    let mut attempts = 0;
    loop {
        attempts += 1;
        let mut response = agent(Duration::from_secs(15))
            .post(&url)
            .header("Authorization", format!("Bot {token}"))
            .send_json(&body)
            .map_err(|error| ApiError::Failed(format!("discord create message: {error}")))?;
        let status = response.status().as_u16();
        if status == 429 && attempts == 1 {
            std::thread::sleep(retry_after(&mut response));
            continue;
        }
        if (200..300).contains(&status) {
            return Ok(());
        }
        if status == 429 {
            return Err(ApiError::RateLimited(retry_after(&mut response)));
        }
        return Err(ApiError::Failed(format!(
            "discord create message: http {status}"
        )));
    }
}

fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_MESSAGE_CHARS {
        return text.to_string();
    }
    let mut out: String = text.chars().take(MAX_MESSAGE_CHARS - 1).collect();
    out.push('…');
    out
}

/// Handle one inbound line. Unbound senders get silence except `/start`.
pub fn handle_inbound(
    hub: &HubStore,
    bindings: &mut BindingStore,
    from: &DiscordUser,
    text: &str,
) -> Reply {
    match parse_inbound(text) {
        Inbound::Start { code } => redeem(bindings, from, code.as_deref()),
        _ if !bindings.is_bound(from.user_id) => Reply::Silent,
        inbound => dispatch::apply_authenticated(hub, "discord", inbound),
    }
}

fn redeem(bindings: &mut BindingStore, from: &DiscordUser, code: Option<&str>) -> Reply {
    let Some(code) = code else {
        return Reply::Text("post /start <code> from `ca discord pair` in this channel".into());
    };
    match bindings.redeem(from.user_id, from.channel_id, from.username.clone(), code) {
        Ok(()) => Reply::Text("paired. /help for commands.".into()),
        Err(error) => Reply::Text(error.to_string()),
    }
}

/// Apply one channel batch (already oldest-first) and reply where needed.
pub fn process_channel_messages(
    hub: &HubStore,
    bindings: &mut BindingStore,
    token: &str,
    inbound: Vec<(DiscordUser, String)>,
) -> Result<(), String> {
    for (user, text) in inbound {
        match handle_inbound(hub, bindings, &user, &text) {
            Reply::Silent => {}
            Reply::Text(reply) => {
                send_message(token, user.channel_id, &reply).map_err(|error| error.to_string())?
            }
        }
    }
    Ok(())
}

/// Push newly seen pending wakes to every bound channel.
pub fn push_new_wakes(
    hub: &HubStore,
    bindings: &mut BindingStore,
    token: &str,
) -> Result<(), String> {
    if bindings.bound_users().is_empty() {
        return Ok(());
    }
    let wakes = hub
        .list_wakes(None, true)
        .map_err(|error| error.to_string())?;
    for wake in wakes {
        if bindings.was_wake_notified(&wake.id) {
            continue;
        }
        let text = format!(
            "wake {} for {} — /approve {} or /reject {}",
            wake.id, wake.target_agent, wake.id, wake.id
        );
        for user in bindings.bound_users() {
            send_message(token, user.channel_id, &text).map_err(|error| error.to_string())?;
        }
        bindings
            .mark_wake_notified(&wake.id)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::HubStore;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use tempfile::tempdir;

    const FIXTURE: &str = r#"[
        {"id": "902", "channel_id": "100", "type": 0,
         "author": {"id": "7", "username": "pk", "bot": false}, "content": "/wakes"},
        {"id": "901", "channel_id": "100", "type": 0,
         "author": {"id": "9", "username": "ca-bot", "bot": true}, "content": "paired."},
        {"id": "900", "channel_id": "100", "type": 0,
         "author": {"id": "7", "username": "pk", "bot": false}, "content": "  "},
        {"id": "899", "channel_id": "100", "type": 0,
         "author": {"id": "8", "username": "rk", "bot": false}, "content": "/start ABC123"}
    ]"#;

    #[test]
    fn parse_messages_skips_bots_blank_texts_and_orders_oldest_first() {
        let (inbound, max_seen) = parse_messages(FIXTURE, 100).unwrap();
        assert_eq!(max_seen, Some(902));
        assert_eq!(inbound.len(), 2);
        assert_eq!(inbound[0].0.user_id, 8);
        assert_eq!(inbound[0].1, "/start ABC123");
        assert_eq!(inbound[1].0.user_id, 7);
        assert_eq!(inbound[1].0.channel_id, 100);
        assert_eq!(inbound[1].0.username.as_deref(), Some("pk"));
        assert_eq!(inbound[1].1, "/wakes");
    }

    #[test]
    fn snowflake_accepts_a_bare_number() {
        let body = r#"[{"id": 42, "author": {"id": 7}, "content": "hi"}]"#;
        let (inbound, max_seen) = parse_messages(body, 100).unwrap();
        assert_eq!(max_seen, Some(42));
        assert_eq!(inbound[0].1, "hi");
    }

    #[test]
    fn truncate_caps_at_discord_limit() {
        let long = "x".repeat(3000);
        let cut = truncate(&long);
        assert_eq!(cut.chars().count(), MAX_MESSAGE_CHARS);
        assert!(cut.ends_with('…'));
        assert_eq!(truncate("short"), "short");
    }

    fn user(id: u64) -> DiscordUser {
        DiscordUser {
            user_id: id,
            channel_id: 100,
            username: Some("pk".into()),
        }
    }

    #[test]
    fn strangers_are_silent_and_pairing_then_approve_works() {
        let dir = tempdir().unwrap();
        let hub = HubStore::open(dir.path()).unwrap();
        let mut bindings = BindingStore::open(dir.path()).unwrap();
        assert_eq!(
            handle_inbound(&hub, &mut bindings, &user(1), "/wakes"),
            Reply::Silent
        );
        let code = bindings.issue_pairing(100).unwrap();
        assert!(matches!(
            handle_inbound(&hub, &mut bindings, &user(1), &format!("/start {code}")),
            Reply::Text(text) if text.starts_with("paired")
        ));
        let wake = hub
            .request_wake("claude", Some("need you"), None, true)
            .unwrap();
        let reply = handle_inbound(
            &hub,
            &mut bindings,
            &user(1),
            &format!("/approve {}", wake.id),
        );
        assert!(matches!(reply, Reply::Text(text) if text.contains("delivered")));
        assert!(hub.list_wakes(None, true).unwrap().is_empty());
    }

    // --- Loopback mock-server tests (no real Discord traffic) ---

    fn serve_once(status: u16, body: &str) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let body = body.to_string();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut raw = Vec::new();
            let mut chunk = [0u8; 2048];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&chunk[..n]);
                if request_complete(&raw) {
                    break;
                }
            }
            let reply = format!(
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&raw).to_string()
        });
        (base, handle)
    }

    fn request_complete(raw: &[u8]) -> bool {
        let text = String::from_utf8_lossy(raw);
        let Some((head, body)) = text.split_once("\r\n\r\n") else {
            return false;
        };
        let wanted = head
            .lines()
            .find_map(|line| line.split_once(':'))
            .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        body.len() >= wanted
    }

    #[test]
    fn fetch_and_send_hit_the_rest_endpoints_with_bot_auth() {
        let (base, get) = serve_once(200, FIXTURE);
        let (inbound, max_seen) = fetch_messages_from(&base, "secret", 100, Some(900)).unwrap();
        let request = get.join().unwrap();
        assert!(request.starts_with("GET /channels/100/messages?limit=100&after=900 "));
        assert!(request.contains("\r\nauthorization: Bot secret\r\n"));
        assert_eq!(max_seen, Some(902));
        assert_eq!(inbound.len(), 2);

        let (base, post) = serve_once(200, r#"{"id": "903"}"#);
        send_message_to(&base, "secret", 100, "hello").unwrap();
        let request = post.join().unwrap();
        assert!(request.starts_with("POST /channels/100/messages "));
        assert!(request.contains("\r\nauthorization: Bot secret\r\n"));
        assert!(request.contains("hello"), "request: {request}");
    }

    #[test]
    fn send_retries_once_after_a_429_then_succeeds() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            for (status, body) in [
                (429, r#"{"message":"rate limited","retry_after":0.01}"#),
                (200, r#"{"id":"903"}"#),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = [0u8; 4096];
                let n = stream.read(&mut raw).unwrap();
                assert!(n > 0);
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        send_message_to(&base, "secret", 100, "hello").unwrap();
        handle.join().unwrap();
    }
}

//! Discord Bot REST transport (U25 / #319): outbound channel-history polling,
//! pairing allow-list, `/approve` `/reject` `/send` `/wakes`, unknown senders silent.
//! Commands are plain channel messages, not registered Discord slash commands.

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

/// One polled batch: inbound `(message_id, sender, text)` tuples (oldest first) plus the
/// newest snowflake seen, for advancing the channel cursor.
pub type MessageBatch = (Vec<(u64, DiscordUser, String)>, Option<u64>);

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
    Ok((rows, max_seen))
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
    let body = response.body_mut().read_to_string().unwrap_or_default();
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
    let suffix = after.map(|a| format!("&after={a}")).unwrap_or_default();
    let url = format!("{api_base}/channels/{channel_id}/messages?limit=100{suffix}");
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
/// Reloads durable bindings first so external unbind/pair CLI invocations
/// take effect before authorization. Fails closed on reload errors.
pub fn handle_inbound(
    hub: &HubStore,
    bindings: &mut BindingStore,
    from: &DiscordUser,
    text: &str,
) -> Reply {
    if let Err(error) = bindings.reload() {
        eprintln!("discord binding reload failed: {error}");
        return Reply::Silent;
    }
    handle_inbound_with_reloaded(hub, bindings, from, text)
}

fn handle_inbound_with_reloaded(
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

/// Apply one channel batch (already oldest-first), persisting the cursor
/// before each reply so a failed outbound delivery cannot re-execute on restart.
pub fn process_channel_messages(
    hub: &HubStore,
    bindings: &mut BindingStore,
    token: &str,
    inbound: Vec<(u64, DiscordUser, String)>,
) -> Result<(), String> {
    process_channel_messages_to(API, hub, bindings, token, inbound)
}

pub(crate) fn process_channel_messages_to(
    api: &str,
    hub: &HubStore,
    bindings: &mut BindingStore,
    token: &str,
    inbound: Vec<(u64, DiscordUser, String)>,
) -> Result<(), String> {
    for (id, user, text) in inbound {
        bindings.reload().map_err(|error| error.to_string())?;
        let reply = handle_inbound_with_reloaded(hub, bindings, &user, &text);
        bindings
            .advance_cursor(user.channel_id, id)
            .map_err(|error| error.to_string())?;
        if let Reply::Text(reply) = reply {
            send_message_to(api, token, user.channel_id, &reply)
                .map_err(|error| error.to_string())?;
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
    bindings.reload().map_err(|error| error.to_string())?;
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
#[path = "discord_tests.rs"]
mod tests;

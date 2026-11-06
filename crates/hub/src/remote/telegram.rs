//! Telegram Bot API transport. Outbound long-poll only — no webhook (U25).

use super::dispatch::ChatUser;
use serde::Deserialize;
use std::time::Duration;

const API: &str = "https://api.telegram.org";

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    from: Option<User>,
    chat: Chat,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct User {
    id: i64,
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Chat {
    id: i64,
    #[serde(rename = "type")]
    kind: String,
}

/// Inbound private-chat texts plus the next `getUpdates` offset.
pub fn parse_updates(body: &str) -> Result<(i64, Vec<(ChatUser, String)>), String> {
    let parsed: ApiResponse<Vec<Update>> =
        serde_json::from_str(body).map_err(|error| format!("telegram json: {error}"))?;
    if !parsed.ok {
        return Err(parsed
            .description
            .unwrap_or_else(|| "telegram api error".into()));
    }
    let updates = parsed.result.unwrap_or_default();
    let next_offset = updates
        .iter()
        .map(|update| update.update_id + 1)
        .max()
        .unwrap_or(0);
    let inbound = updates
        .into_iter()
        .filter_map(|update| {
            let message = update.message?;
            if message.chat.kind != "private" {
                return None;
            }
            let from = message.from?;
            let text = message.text?;
            Some((
                ChatUser {
                    user_id: from.id,
                    chat_id: message.chat.id,
                    username: from.username,
                },
                text,
            ))
        })
        .collect();
    Ok((next_offset, inbound))
}

/// Long-poll `getUpdates`. `token` is used only to build the URL.
pub fn get_updates(token: &str, offset: i64) -> Result<(i64, Vec<(ChatUser, String)>), String> {
    let url = format!(
        "{API}/bot{token}/getUpdates?offset={offset}&timeout=30&allowed_updates=%5B%22message%22%5D"
    );
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(40)))
        .build()
        .into();
    let mut response = agent
        .get(&url)
        .call()
        .map_err(|error| format!("telegram getUpdates: {error}"))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("telegram getUpdates body: {error}"))?;
    parse_updates(&body)
}

/// Send a chat reply. `token` is used only to build the URL.
pub fn send_message(token: &str, chat_id: i64, text: &str) -> Result<(), String> {
    let url = format!("{API}/bot{token}/sendMessage");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut response = agent
        .post(&url)
        .send_json(serde_json::json!({ "chat_id": chat_id, "text": text }))
        .map_err(|error| format!("telegram sendMessage: {error}"))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("telegram sendMessage body: {error}"))?;
    let parsed: ApiResponse<serde_json::Value> = serde_json::from_str(&body)
        .map_err(|error| format!("telegram sendMessage json: {error}"))?;
    if parsed.ok {
        Ok(())
    } else {
        Err(parsed
            .description
            .unwrap_or_else(|| "telegram sendMessage failed".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_updates_keeps_private_text_and_advances_offset() {
        let body = r#"{
            "ok": true,
            "result": [
                {
                    "update_id": 40,
                    "message": {
                        "message_id": 1,
                        "from": {"id": 9, "username": "pk"},
                        "chat": {"id": 9, "type": "private"},
                        "text": "/wakes"
                    }
                },
                {
                    "update_id": 41,
                    "message": {
                        "message_id": 2,
                        "from": {"id": 9, "username": "pk"},
                        "chat": {"id": -100, "type": "group"},
                        "text": "/approve x"
                    }
                }
            ]
        }"#;
        let (offset, inbound) = parse_updates(body).unwrap();
        assert_eq!(offset, 42);
        assert_eq!(inbound.len(), 1);
        assert_eq!(inbound[0].0.user_id, 9);
        assert_eq!(inbound[0].1, "/wakes");
    }
}

//! Chat-platform remote client (U25 / #319). Telegram v1 only.
//!
//! The bot token is resolved through P12. Pairing state lives in
//! `telegram_binding.json` next to `hub.db`. Inbound slash commands map onto
//! existing HubStore writes; unknown senders are silent.

mod binding;
mod command;
mod dispatch;
mod telegram;

pub use binding::BindingStore;
pub use command::{parse_inbound, Inbound};
pub use dispatch::{handle_inbound, ChatUser, Reply};
pub use telegram::{get_updates, parse_updates, send_message};

use crate::secret;
use crate::store::HubStore;

const TOKEN_KEY: &str = "TELEGRAM_BOT_TOKEN";

/// Resolve the bot token (vault wins, env fallback). Never log the value.
pub fn resolve_bot_token() -> Option<secret::SecretString> {
    secret::resolve(TOKEN_KEY)
}

/// Apply one inbound batch and push newly seen pending wakes to bound chats.
pub fn process_updates(
    hub: &HubStore,
    bindings: &mut BindingStore,
    token: &str,
    inbound: Vec<(ChatUser, String)>,
) -> Result<(), String> {
    for (user, text) in inbound {
        match handle_inbound(hub, bindings, &user, &text) {
            Reply::Silent => {}
            Reply::Text(reply) => send_message(token, user.chat_id, &reply)?,
        }
    }
    push_new_wakes(hub, bindings, token)
}

fn push_new_wakes(hub: &HubStore, bindings: &mut BindingStore, token: &str) -> Result<(), String> {
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
            send_message(token, user.chat_id, &text)?;
        }
        bindings
            .mark_wake_notified(&wake.id)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

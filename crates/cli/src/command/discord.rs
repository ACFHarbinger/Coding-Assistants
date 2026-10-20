//! `ca discord` runner (U25 / #319).

use anyhow::Context;
use hub::remote::discord::{self, BindingStore};
use std::thread::sleep;
use std::time::Duration;

use crate::app::DiscordCommand;

/// Discord rate limits channel-history polls; keep cycles at 5s or slower.
const MIN_POLL_SECS: u64 = 5;

pub(crate) fn run(store: &hub::HubStore, action: DiscordCommand) -> anyhow::Result<()> {
    let mut bindings = BindingStore::open(store.data_dir())?;
    match action {
        DiscordCommand::Pair { channel } => {
            let code = bindings.issue_pairing(channel)?;
            println!("pairing code {code} (10 minutes). In channel {channel}, post: /start {code}");
        }
        DiscordCommand::Status => {
            let token_set = discord::resolve_bot_token().is_some();
            println!("token: {}", if token_set { "set" } else { "missing" });
            if bindings.bound_users().is_empty() {
                println!("bound: none");
            } else {
                for user in bindings.bound_users() {
                    println!(
                        "bound: {} channel {} {}",
                        user.user_id,
                        user.channel_id,
                        user.username.as_deref().unwrap_or("-")
                    );
                }
            }
        }
        DiscordCommand::Unbind { user_id } => {
            let removed = bindings.unbind(user_id)?;
            println!("unbound {removed}");
        }
        DiscordCommand::Run { interval_secs } => {
            run_loop(store, &mut bindings, interval_secs.max(MIN_POLL_SECS))?
        }
    }
    Ok(())
}

fn run_loop(
    store: &hub::HubStore,
    bindings: &mut BindingStore,
    interval_secs: u64,
) -> anyhow::Result<()> {
    let token = discord::resolve_bot_token().context("DISCORD_BOT_TOKEN is not set")?;
    let token = token.expose();
    if bindings.bound_users().is_empty() {
        eprintln!(
            "no bound users yet; run `ca discord pair --channel <id>` and post /start <code> there"
        );
    }
    let interval = Duration::from_secs(interval_secs);
    loop {
        for channel in bindings.poll_channels() {
            let after = bindings.cursor(channel);
            match discord::fetch_messages(token, channel, after) {
                Ok((inbound, max_seen)) => {
                    discord::process_channel_messages(store, bindings, token, inbound)
                        .map_err(|error| anyhow::anyhow!(error))?;
                    if let Some(max_seen) = max_seen {
                        bindings
                            .advance_cursor(channel, max_seen)
                            .map_err(|error| anyhow::anyhow!(error))?;
                    }
                }
                Err(discord::ApiError::RateLimited(wait)) => sleep(wait),
                Err(discord::ApiError::Failed(error)) => return Err(anyhow::anyhow!(error)),
            }
        }
        discord::push_new_wakes(store, bindings, token).map_err(|error| anyhow::anyhow!(error))?;
        sleep(interval);
    }
}

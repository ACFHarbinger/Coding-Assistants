//! `ca telegram` runner (U25 / #319).

use anyhow::Context;
use hub::remote::{self, BindingStore};

use crate::app::TelegramCommand;

pub(crate) fn run(store: &hub::HubStore, action: TelegramCommand) -> anyhow::Result<()> {
    let mut bindings = BindingStore::open(store.data_dir())?;
    match action {
        TelegramCommand::Pair => {
            let code = bindings.issue_pairing()?;
            println!("pairing code {code} (10 minutes). DM the bot: /start {code}");
        }
        TelegramCommand::Status => {
            let token_set = remote::resolve_bot_token().is_some();
            println!("token: {}", if token_set { "set" } else { "missing" });
            if bindings.bound_users().is_empty() {
                println!("bound: none");
            } else {
                for user in bindings.bound_users() {
                    println!(
                        "bound: {} chat {} {}",
                        user.user_id,
                        user.chat_id,
                        user.username.as_deref().unwrap_or("-")
                    );
                }
            }
        }
        TelegramCommand::Unbind { user_id } => {
            let removed = bindings.unbind(user_id)?;
            println!("unbound {removed}");
        }
        TelegramCommand::Run => run_loop(store, &mut bindings)?,
    }
    Ok(())
}

fn run_loop(store: &hub::HubStore, bindings: &mut BindingStore) -> anyhow::Result<()> {
    let token = remote::resolve_bot_token().context("TELEGRAM_BOT_TOKEN is not set")?;
    let token = token.expose();
    if bindings.bound_users().is_empty() {
        eprintln!("no bound users yet; run `ca telegram pair` and /start the code");
    }
    let mut offset = bindings.update_offset();
    loop {
        let (next, inbound) =
            remote::get_updates(token, offset).map_err(|error| anyhow::anyhow!(error))?;
        remote::process_updates(store, bindings, token, inbound)
            .map_err(|error| anyhow::anyhow!(error))?;
        if next > 0 {
            bindings
                .advance_update_offset(next)
                .map_err(|error| anyhow::anyhow!(error))?;
            offset = next;
        }
    }
}

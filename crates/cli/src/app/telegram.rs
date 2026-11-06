//! `ca telegram` subcommands (U25 / #319).

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum TelegramCommand {
    /// Print a one-time pairing code (valid 10 minutes).
    Pair,
    /// Show bound Telegram user ids and whether a bot token is set.
    Status,
    /// Remove one bound user, or every bound user if `--user-id` is omitted.
    Unbind {
        #[arg(long)]
        user_id: Option<i64>,
    },
    /// Long-poll Telegram and map commands onto the Hub (opt-in, outbound only).
    Run,
}

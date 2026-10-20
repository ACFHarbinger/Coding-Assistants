//! `ca discord` subcommands (U25 / #319).

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum DiscordCommand {
    /// Print a one-time pairing code (valid 10 minutes) for a guild channel.
    Pair {
        /// Snowflake id of the channel the bot watches for `/start <code>`.
        #[arg(long)]
        channel: u64,
    },
    /// Show bound Discord user ids and whether a bot token is set.
    Status,
    /// Remove one bound user, or every bound user if `--user-id` is omitted.
    Unbind {
        #[arg(long)]
        user_id: Option<u64>,
    },
    /// Poll bound Discord channels and map commands onto the Hub (opt-in,
    /// outbound REST only; needs the Message Content intent).
    Run {
        /// Seconds between poll cycles (clamped to >= 5; Discord rate limits).
        #[arg(long, default_value_t = 5)]
        interval_secs: u64,
    },
}

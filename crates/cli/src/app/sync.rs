//! `ca sync` subcommands (S4 / #94).

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum SyncCommand {
    /// Show account, policy counts, last verified base, and schema warnings.
    Preview,
    /// Owner-started upload run. Takes the Hub lock; encrypts a snapshot.
    Up,
    /// Owner-started download run. Takes the Hub lock; restores into staging.
    Down,
    /// Owner-started up then down. Takes the Hub lock; never replaces live hub.db.
    Sync,
    /// Release a held Hub sync lock.
    Cancel,
}

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum JournalCommand {
    Append {
        #[arg(long)]
        agent: String,
        entry: String,
    },
}

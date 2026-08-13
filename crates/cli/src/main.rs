mod agent;
mod app;
mod command;
mod harness;
mod helpers;
mod io;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    command::run(app::Cli::parse())
}

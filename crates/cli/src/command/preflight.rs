use crate::app::Command;
use std::path::PathBuf;

pub(super) fn run_if_requested(command: &Command, home: PathBuf) -> Option<anyhow::Result<()>> {
    match command {
        Command::Preflight {
            workspace,
            session,
            json,
        } => Some(crate::preflight::run(
            home,
            workspace.clone(),
            session.clone(),
            *json,
        )),
        _ => None,
    }
}

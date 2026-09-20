//! `ca tool` runner (P5 / #333).

use anyhow::Context;
use hub::os_tool::{self, OsToolRequest};
use hub::{HubStore, SettingsStore};
use std::path::{Path, PathBuf};

use crate::app::ToolCommand;

pub(crate) fn run(store: &HubStore, home: &Path, action: ToolCommand) -> anyhow::Result<()> {
    match action {
        ToolCommand::Run {
            cwd,
            workspace,
            program,
            args,
        } => {
            let cwd = abs_dir(cwd.unwrap_or(std::env::current_dir()?))?;
            let workspace = match workspace {
                Some(path) => abs_dir(path)?,
                None => cwd.clone(),
            };
            let workspace_str = workspace.to_string_lossy().into_owned();
            let strictness = SettingsStore::open(home)
                .effective(Some(workspace_str.as_str()))
                .orchestration
                .sandbox_strictness;
            let result = os_tool::submit(
                store,
                OsToolRequest {
                    program,
                    args,
                    cwd: cwd.to_string_lossy().into_owned(),
                    workspace: Some(workspace_str),
                },
                strictness,
                os_tool::DEFAULT_TIMEOUT,
            )
            .map_err(|error| anyhow::anyhow!(error))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if result.status == "denied" {
                anyhow::bail!("os tool denied");
            }
        }
        ToolCommand::Approve { id } => {
            let result = os_tool::approve(store, &id, os_tool::DEFAULT_TIMEOUT)
                .map_err(|error| anyhow::anyhow!(error))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        ToolCommand::Pending => {
            let pending: Vec<_> = store
                .list_audit_events(true)?
                .into_iter()
                .filter(|event| event.operation == "os_tool.proposed")
                .collect();
            println!("{}", serde_json::to_string_pretty(&pending)?);
        }
    }
    Ok(())
}

fn abs_dir(path: PathBuf) -> anyhow::Result<PathBuf> {
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    let path = path
        .canonicalize()
        .with_context(|| format!("directory not found: {}", path.display()))?;
    if !path.is_dir() {
        anyhow::bail!("not a directory: {}", path.display());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Cli, Command, ToolCommand};
    use clap::Parser;
    use hub::SandboxStrictness;

    #[test]
    fn clap_keeps_hyphen_args_as_data() {
        let cli = Cli::try_parse_from(["ca", "tool", "run", "echo", "-n", "p5-ok"]).unwrap();
        match cli.command {
            Command::Tool {
                action: ToolCommand::Run { program, args, .. },
            } => {
                assert_eq!(program, "echo");
                assert_eq!(args, vec!["-n", "p5-ok"]);
            }
            _ => panic!("expected ca tool run"),
        }
    }

    #[test]
    fn run_echo_is_audited() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path()).unwrap();
        let cwd = dir.path().to_string_lossy().into_owned();
        let result = os_tool::submit(
            &store,
            OsToolRequest {
                program: "echo".into(),
                args: vec!["cli-p5".into()],
                cwd: cwd.clone(),
                workspace: Some(cwd),
            },
            SandboxStrictness::Standard,
            os_tool::DEFAULT_TIMEOUT,
        )
        .unwrap();
        assert_eq!(result.status, "ran");
        assert!(result.stdout.contains("cli-p5"));
    }
}

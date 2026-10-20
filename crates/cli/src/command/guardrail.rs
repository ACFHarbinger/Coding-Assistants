//! `ca guardrail` runner (P16 / #360). Thin wrapper over
//! `HubStore::{add,list,set_enabled,remove}_guardrail`; screening itself
//! happens inside the hub on dispatch and on reply acceptance.

use anyhow::{bail, Context};
use hub::guardrails::{run_rules, GuardrailMode, GuardrailSpec, GuardrailStage};
use hub::HubStore;
use std::io::Read;

use crate::app::GuardrailCommand;

fn parse_stage(raw: &str) -> anyhow::Result<GuardrailStage> {
    GuardrailStage::parse(raw)
        .ok_or_else(|| anyhow::anyhow!("--stage must be input or output, got {raw:?}"))
}

pub(crate) fn run(store: &HubStore, action: GuardrailCommand) -> anyhow::Result<()> {
    match action {
        GuardrailCommand::List { workspace } => {
            let rules = store.list_guardrails(workspace.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&rules)?);
        }
        GuardrailCommand::Add {
            name,
            stage,
            mode,
            workspace,
            spec,
        } => {
            let stage = parse_stage(&stage)?;
            let mode = GuardrailMode::parse(&mode)
                .ok_or_else(|| anyhow::anyhow!("--mode must be blocking or parallel"))?;
            let spec: GuardrailSpec =
                serde_json::from_str(&spec).context("--spec is not a guardrail spec")?;
            let rule = store.add_guardrail(&name, workspace.as_deref(), stage, mode, spec)?;
            println!("{}", serde_json::to_string_pretty(&rule)?);
        }
        GuardrailCommand::Enable { id, on } => {
            let rule = store.set_guardrail_enabled(&id, on)?;
            println!("{}", serde_json::to_string_pretty(&rule)?);
        }
        GuardrailCommand::Remove { id } => {
            store.remove_guardrail(&id)?;
            println!("removed {id}");
        }
        GuardrailCommand::Check {
            stage,
            workspace,
            text,
        } => {
            let stage = parse_stage(&stage)?;
            let content = if text == "-" {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                text
            };
            let rules = store.effective_guardrails(stage, workspace.as_deref())?;
            let verdict = run_rules(&rules, &content);
            println!("{}", serde_json::to_string_pretty(&verdict)?);
            if verdict.halted() {
                bail!(
                    "tripwire: {}",
                    verdict.tripped().map(|t| t.detail.as_str()).unwrap_or("")
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::app::{Cli, Command, GuardrailCommand};
    use clap::Parser;

    #[test]
    fn enable_accepts_on_false() {
        let cli =
            Cli::try_parse_from(["ca", "guardrail", "enable", "rule-1", "--on", "false"]).unwrap();
        match cli.command {
            Command::Guardrail {
                action: GuardrailCommand::Enable { id, on },
            } => {
                assert_eq!(id, "rule-1");
                assert!(!on);
            }
            _ => panic!("expected ca guardrail enable"),
        }
    }

    #[test]
    fn enable_defaults_to_true() {
        let cli = Cli::try_parse_from(["ca", "guardrail", "enable", "rule-1"]).unwrap();
        match cli.command {
            Command::Guardrail {
                action: GuardrailCommand::Enable { on, .. },
            } => assert!(on),
            _ => panic!("expected ca guardrail enable"),
        }
    }
}

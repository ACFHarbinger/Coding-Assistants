//! C16 sub-orchestrator calls: a parent role invokes a P4 direct-HTTP
//! specialist in-process (`[[SUBORCH:Role]]`), budget-gated and audited.

use super::budget::BudgetContext;
use super::orchestrator::RoleConfig;
use crate::client::http::is_direct_http_provider;
use crate::client::llm::{LLMClient, ModelConfig};
use hub::bus::{InProcessBus, TOPIC_AGENT_EVENT};
use hub::HubStore;
use serde_json::json;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub const MAX_FANOUT: u8 = 4;
const MARKER: &str = "[[SUBORCH:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuborchInvocation {
    pub role: String,
    pub instruction: String,
}

pub fn parse(response: &str) -> Option<SuborchInvocation> {
    let pos = response.find(MARKER)?;
    let rest = &response[pos + MARKER.len()..];
    let end = rest.find("]]")?;
    let role = rest[..end].trim();
    if role.is_empty() {
        return None;
    }
    let instruction = rest[end + 2..].trim();
    Some(SuborchInvocation {
        role: role.to_string(),
        instruction: if instruction.is_empty() {
            "Classify or route the current work in one short answer.".into()
        } else {
            instruction.to_string()
        },
    })
}

fn audit(work_dir: &str, parent: &str, role: &str, config: Option<&ModelConfig>, status: &str) {
    let Ok(store) = HubStore::open(hub::default_hub_home()) else {
        return;
    };
    let process = json!({
        "kind": "suborch",
        "parent": parent,
        "role": role,
        "provider": config.map(|c| c.provider.as_str()).unwrap_or(""),
        "model": config.map(|c| c.model.as_str()).unwrap_or(""),
        "status": status,
    })
    .to_string();
    let root = Path::new(work_dir);
    let _ = store.record_audit_event(root, root, "suborch.call", &process, None);
}

fn system_note(text: impl Into<String>) -> String {
    format!("\n\nSystem: {}", text.into())
}

/// Run one SUBORCH call. Never spawns a CLI. Folds errors into a system note.
#[allow(clippy::too_many_arguments)]
pub async fn invoke(
    invocation: &SuborchInvocation,
    parent: &str,
    parent_response: &str,
    roles: &[RoleConfig],
    client: &LLMClient,
    work_dir: &str,
    bus: &InProcessBus,
    token: Arc<AtomicBool>,
    mcp_config_path: Option<&str>,
    budget: &BudgetContext,
    fanout: &mut u8,
) -> Result<String, String> {
    if *fanout >= MAX_FANOUT {
        audit(work_dir, parent, &invocation.role, None, "capped");
        return Ok(system_note(format!(
            "sub-orchestrator fan-out capped at {MAX_FANOUT} calls this turn."
        )));
    }
    let target = roles
        .iter()
        .find(|role| role.name.eq_ignore_ascii_case(&invocation.role));
    let Some(target) = target else {
        audit(work_dir, parent, &invocation.role, None, "unknown_role");
        let names: Vec<_> = roles.iter().map(|role| role.name.as_str()).collect();
        return Ok(system_note(format!(
            "unknown sub-orchestrator role {}. Available: {}.",
            invocation.role,
            names.join(", ")
        )));
    };
    if !is_direct_http_provider(&target.config.provider, target.config.endpoint.as_deref()) {
        audit(
            work_dir,
            parent,
            &target.name,
            Some(&target.config),
            "unavailable",
        );
        return Ok(system_note(format!(
            "sub-orchestrator {} unavailable: provider `{}` is not a P4 direct-HTTP model.",
            target.name, target.config.provider
        )));
    }
    if let Err(stop) = budget.deny_unless_allowed(&target.name) {
        audit(
            work_dir,
            parent,
            &target.name,
            Some(&target.config),
            "budget_stopped",
        );
        return Ok(system_note(stop));
    }
    *fanout = fanout.saturating_add(1);
    bus.emit(
        TOPIC_AGENT_EVENT,
        super::orchestrator::AgentEvent {
            source: parent.to_string(),
            event_type: "thought".to_string(),
            content: format!(
                "Sub-orchestrator {}: {}",
                target.name, invocation.instruction
            ),
        },
    );
    let prompt = format!(
        "You are specialist {}.\nAnswer in a short classification or routing note. Do not emit ASK_USER, ASK_AGENT, or SUBORCH markers.\n\nParent ({parent}) said:\n{parent_response}\n\nInstruction:\n{}",
        target.name, invocation.instruction
    );
    match client
        .chat_completion(
            &target.config,
            &prompt,
            Some(work_dir),
            bus,
            &target.name,
            mcp_config_path,
            Some(token),
        )
        .await
    {
        Ok(answer) => {
            audit(work_dir, parent, &target.name, Some(&target.config), "ok");
            budget.write_exhaustion_handoff(&target.name, "Completed a SUBORCH specialist call.");
            Ok(format!(
                "\n\nAgent: {parent_response}\n\nSub-orchestrator {}: {answer}",
                target.name
            ))
        }
        Err(error) => {
            let status = if error.to_ascii_lowercase().contains("unavailable") {
                "unavailable"
            } else {
                "error"
            };
            audit(work_dir, parent, &target.name, Some(&target.config), status);
            Ok(system_note(format!(
                "sub-orchestrator {} {status} ({error})",
                target.name
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(name: &str, provider: &str) -> RoleConfig {
        RoleConfig {
            name: name.into(),
            config: ModelConfig {
                provider: provider.into(),
                model: "gpt-4o-mini".into(),
                ..ModelConfig::default()
            },
        }
    }

    #[test]
    fn parse_reads_role_and_instruction() {
        let parsed = parse("plan it\n[[SUBORCH:Router]] classify as bug or feature").unwrap();
        assert_eq!(parsed.role, "Router");
        assert_eq!(parsed.instruction, "classify as bug or feature");
        assert!(parse("no marker").is_none());
        assert!(parse("[[SUBORCH:]] empty").is_none());
        assert!(parse("[[SUBORCH:Router").is_none());
    }

    #[test]
    fn parse_defaults_an_empty_instruction() {
        let parsed = parse("[[SUBORCH:Router]]").unwrap();
        assert!(parsed.instruction.contains("Classify"));
    }

    #[tokio::test]
    async fn cli_provider_is_unavailable_without_spawning() {
        let roles = vec![role("Router", "opencode")];
        let invocation = SuborchInvocation {
            role: "Router".into(),
            instruction: "route".into(),
        };
        let dir = tempfile::tempdir().unwrap();
        let budget = BudgetContext::open("c16");
        let bus = InProcessBus::new();
        let client = LLMClient::new();
        let mut fanout = 0;
        let note = invoke(
            &invocation,
            "Planner",
            "do work",
            &roles,
            &client,
            &dir.path().to_string_lossy(),
            &bus,
            Arc::new(AtomicBool::new(false)),
            None,
            &budget,
            &mut fanout,
        )
        .await
        .unwrap();
        assert!(note.contains("unavailable"), "{note}");
        assert!(note.contains("P4 direct-HTTP"), "{note}");
        assert_eq!(fanout, 0);
    }

    #[tokio::test]
    async fn unknown_role_and_fanout_cap_are_notes() {
        let roles = vec![role("Router", "openai")];
        let invocation = SuborchInvocation {
            role: "Missing".into(),
            instruction: "x".into(),
        };
        let dir = tempfile::tempdir().unwrap();
        let budget = BudgetContext::open("c16");
        let bus = InProcessBus::new();
        let client = LLMClient::new();
        let mut fanout = 0;
        let unknown = invoke(
            &invocation,
            "Planner",
            "do work",
            &roles,
            &client,
            &dir.path().to_string_lossy(),
            &bus,
            Arc::new(AtomicBool::new(false)),
            None,
            &budget,
            &mut fanout,
        )
        .await
        .unwrap();
        assert!(unknown.contains("unknown sub-orchestrator"), "{unknown}");

        fanout = MAX_FANOUT;
        let capped = invoke(
            &SuborchInvocation {
                role: "Router".into(),
                instruction: "x".into(),
            },
            "Planner",
            "do work",
            &roles,
            &client,
            &dir.path().to_string_lossy(),
            &bus,
            Arc::new(AtomicBool::new(false)),
            None,
            &budget,
            &mut fanout,
        )
        .await
        .unwrap();
        assert!(capped.contains("fan-out capped"), "{capped}");
    }
}

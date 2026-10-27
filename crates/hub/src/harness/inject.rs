//! Hub task/wake injection dispatch (the inject half of the old
//! harness/mod.rs, split for the 500-LoC cap, #158). Task-only
//! sends to a live harness route through its provider bridge;
//! otherwise the task stays durably queued and never spawns a
//! replacement process.

use crate::HubError;
use crate::{ProviderCallGate, SliceStamp, DEFAULT_PROVIDER_CALL_UNITS};

use super::spawn::spawn_explicit;
use super::{
    claude_spawn_args, codex_spawn_args, cursor_spawn_args, gemini_spawn_args, grok_spawn_args,
    muse_spawn_args, opencode_spawn_args, qwen_spawn_args, vibe_managed_spawn_args, HarnessId,
    HarnessInjectRequest, HarnessInjectResult,
};

/// C6 / #84 first slice: a Claude one-shot spawn is a provider call, so
/// the same pre-call budget decision the in-process path takes through
/// `BudgetContext::deny_unless_allowed` must run before the process
/// starts. `Ok(None)` = allowed (unmetered or reserved); `Ok(Some(..))` =
/// stop without spawning. Reserving consumes one unit per spawned run.
fn claude_spawn_gate(store: &crate::HubStore) -> Result<Option<HarnessInjectResult>, HubError> {
    let stamp = SliceStamp::registered(None, None, Some("claude".to_string()));
    match store.gate_provider_call(
        "claude",
        DEFAULT_PROVIDER_CALL_UNITS,
        &stamp,
        "Claude one-shot harness run",
        "Claude one-shot harness runs completed so far.",
        "Further Claude one-shot runs.",
        None,
    )? {
        ProviderCallGate::Unmetered | ProviderCallGate::Reserved { .. } => Ok(None),
        ProviderCallGate::Stopped { status, handoff } => Ok(Some(HarnessInjectResult {
            harness: "claude".into(),
            pid: None,
            status: "stopped".into(),
            detail: if handoff.is_some() {
                format!(
                    "claude reached its budget ({}/{} units); handoff written; no process was spawned",
                    status.spent_units, status.limit_units
                )
            } else {
                format!(
                    "claude is budget-paused ({}/{} units); resume_agent required; no process was spawned",
                    status.spent_units, status.limit_units
                )
            },
        })),
    }
}

/// Same pre-spawn decision as [`claude_spawn_gate`], for a Codex one-shot
/// (`HarnessId::Chat`). The budget agent id is `codex`, not the harness
/// name `chat`. Task-only delivery returns before this runs.
fn codex_spawn_gate(store: &crate::HubStore) -> Result<Option<HarnessInjectResult>, HubError> {
    let stamp = SliceStamp::registered(None, None, Some("codex".to_string()));
    match store.gate_provider_call(
        "codex",
        DEFAULT_PROVIDER_CALL_UNITS,
        &stamp,
        "Codex one-shot harness run",
        "Codex one-shot harness runs completed so far.",
        "Further Codex one-shot runs.",
        None,
    )? {
        ProviderCallGate::Unmetered | ProviderCallGate::Reserved { .. } => Ok(None),
        ProviderCallGate::Stopped { status, handoff } => Ok(Some(HarnessInjectResult {
            harness: "chat".into(),
            pid: None,
            status: "stopped".into(),
            detail: if handoff.is_some() {
                format!(
                    "Codex reached its budget ({}/{} units); handoff written; no process was spawned",
                    status.spent_units, status.limit_units
                )
            } else {
                format!(
                    "Codex is budget-paused ({}/{} units); resume_agent required; no process was spawned",
                    status.spent_units, status.limit_units
                )
            },
        })),
    }
}

pub fn inject_harness(request: &HarnessInjectRequest) -> Result<HarnessInjectResult, HubError> {
    inject_harness_inner(None, request)
}

/// Same as [`inject_harness`], but Grok task delivery uses a registered
/// active session through the leader ACP bridge when a store is provided.
pub fn inject_harness_with_store(
    store: &crate::HubStore,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    inject_harness_inner(Some(store), request)
}

fn inject_harness_inner(
    store: Option<&crate::HubStore>,
    request: &HarnessInjectRequest,
) -> Result<HarnessInjectResult, HubError> {
    let harness = HarnessId::parse(&request.harness)?;
    if request.body.trim().is_empty() {
        return Err(HubError::Invalid("inject body must not be empty".into()));
    }
    let rewritten;
    let request = if request.is_task {
        rewritten = HarnessInjectRequest {
            body: crate::store::goal_delivery_body(&request.body),
            ..request.clone()
        };
        &rewritten
    } else {
        request
    };
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid("workspace path must be absolute".into()));
    }
    if request.is_task && !request.is_wake {
        if harness == HarnessId::Grok {
            if let Some(store) = store {
                return crate::bridge::grok::deliver_grok_task(store, request);
            }
        }
        if harness == HarnessId::Muse {
            if let Some(store) = store {
                return crate::bridge::muse::deliver_muse_task(store, request);
            }
        }
        if harness == HarnessId::Gemini {
            if let Some(store) = store {
                return crate::bridge::gemini::deliver_gemini_task(store, request);
            }
        }
        if harness == HarnessId::Claude {
            if let Some(store) = store {
                // A connected Claude Code Channel session (C14.3) drains
                // task/wake-tagged messages from its Hub inbox on its own
                // poll loop, so the durable send *is* the delivery — no
                // socket write needed. Only fall through to the C12
                // discovery bridge (which always reports `unavailable`
                // for a live interactive session because `cc-socks` is
                // undocumented) when no Channel session is connected.
                if crate::bridge::channels::claude::is_channel_session_live(&request.workspace)
                    .unwrap_or(false)
                {
                    return Ok(HarnessInjectResult {
                        harness: harness.as_str().into(),
                        pid: None,
                        status: "queued".into(),
                        detail: "A connected Claude Code Channel session for this workspace will \
                                 pick this up from its Hub inbox on its next poll; no manual \
                                 delivery needed"
                            .into(),
                    });
                }
                return crate::bridge::claude::deliver_claude_task(store, request);
            }
        }
        if harness == HarnessId::Chat {
            if let Some(store) = store {
                return crate::bridge::channels::chat::deliver_codex_task(store, request);
            }
        }
        if harness == HarnessId::Cursor {
            if let Some(store) = store {
                return crate::bridge::cursor::deliver_cursor_task(store, request);
            }
        }
        if harness == HarnessId::Vibe {
            if let Some(store) = store {
                return crate::bridge::vibe::deliver_vibe_task(store, request);
            }
        }
        if harness == HarnessId::Qwen {
            if let Some(store) = store {
                return crate::bridge::qwen::deliver_qwen_task(store, request);
            }
        }
        if harness == HarnessId::Kimi {
            if let Some(store) = store {
                return crate::bridge::kimi::deliver_kimi_task(store, request);
            }
        }
        if harness == HarnessId::Hermes {
            if let Some(store) = store {
                return crate::bridge::hermes::deliver_hermes_task(store, request);
            }
        }
        return Ok(HarnessInjectResult {
            harness: harness.as_str().into(),
            pid: None,
            status: "queued".into(),
            detail: "task is recorded in the session inbox; it awaits the target's active harness adapter".into(),
        });
    }

    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid("workspace path must be absolute".into()));
    }
    // C6 / #84 first slice: budget-stop a Claude spawn before the
    // provider call starts. Applies to every spawn this function can
    // make (wake and plain sends); task-only delivery returned earlier
    // and must never spawn. Without a store (bare `inject_harness`) there
    // is no budget to consult, matching the caller-driven CLI contract.
    if harness == HarnessId::Claude {
        if let Some(store) = store {
            if let Some(stopped) = claude_spawn_gate(store)? {
                return Ok(stopped);
            }
        }
    }
    if harness == HarnessId::Chat {
        if let Some(store) = store {
            if let Some(stopped) = codex_spawn_gate(store)? {
                return Ok(stopped);
            }
        }
    }
    let prompt = if request.is_task {
        request.body.clone()
    } else if request.is_wake {
        format!("[WAKE] {}", request.body)
    } else {
        request.body.clone()
    };

    let model = request.model.as_deref().or(match harness {
        HarnessId::OpenCode => Some(crate::harness::DEFAULT_OPENCODE_MODEL),
        HarnessId::DeepSeek => Some(crate::harness::DEFAULT_DEEPSEEK_MODEL),
        _ => None,
    });
    let effort = request.effort.as_deref();

    let args = match harness {
        HarnessId::Grok => grok_spawn_args(&request.workspace, &prompt, model, effort)?,
        HarnessId::Chat => codex_spawn_args(&request.workspace, &prompt, model, effort)?,
        HarnessId::Claude => claude_spawn_args(&request.workspace, &prompt, model, effort)?,
        HarnessId::Gemini => gemini_spawn_args(&request.workspace, &prompt, model, effort)?,
        HarnessId::OpenCode => opencode_spawn_args(&request.workspace, &prompt, model, effort)?,
        HarnessId::DeepSeek => opencode_spawn_args(
            &request.workspace,
            &prompt,
            Some(model.unwrap_or(crate::harness::DEFAULT_DEEPSEEK_MODEL)),
            effort,
        )?,
        HarnessId::Vibe => vibe_managed_spawn_args(
            &request.workspace,
            &prompt,
            request.session_id.as_deref(),
            model,
            effort,
        )?,
        // Muse wakes are one-shot `muse exec` runs (no `--session-id`);
        // task-only delivery routes through `deliver_muse_task` above, which
        // re-enters the managed session headlessly and arms capture.
        HarnessId::Muse => muse_spawn_args(&request.workspace, &prompt, model, effort)?,
        HarnessId::Cursor => cursor_spawn_args(&request.workspace, &prompt, model, effort)?,
        HarnessId::Qwen => qwen_spawn_args(&request.workspace, &prompt, model, effort)?,
        HarnessId::Kimi => super::kimi_managed_spawn_args(
            &request.workspace,
            &prompt,
            request.session_id.as_deref(),
            model,
            effort,
        )?,
        HarnessId::Hermes => super::hermes_managed_spawn_args(
            &request.workspace,
            &prompt,
            request.session_id.as_deref(),
            model,
            effort,
        )?,
    };

    let started = spawn_explicit(
        if harness == HarnessId::Cursor {
            crate::harness::cursor_executable()
        } else if harness == HarnessId::Kimi {
            crate::harness::kimi_executable()
        } else {
            harness.executable()
        },
        &request.workspace,
        &args,
    )?;
    Ok(HarnessInjectResult {
        harness: started.harness,
        pid: started.pid,
        status: if started.status == "started" {
            "spawned".into()
        } else {
            started.status
        },
        detail: started.detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn task_injection_queues_without_spawning_a_new_harness() {
        let result = inject_harness(&HarnessInjectRequest {
            harness: "grok".into(),
            workspace: PathBuf::from("/tmp/workspace-for-task-queue"),
            session_id: Some("session-1".into()),
            message_id: Some("message-1".into()),
            body: "review this".into(),
            is_task: true,
            is_wake: false,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(result.status, "queued");
        assert_eq!(result.pid, None);
    }

    #[test]
    fn chat_task_without_a_persisted_thread_is_unavailable_not_spawned() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::HubStore::open(dir.path()).unwrap();
        let result = crate::inject_harness_with_store(
            &store,
            &HarnessInjectRequest {
                harness: "chat".into(),
                workspace: PathBuf::from("/tmp/workspace-for-codex-queue"),
                session_id: Some("session-1".into()),
                message_id: Some("message-1".into()),
                body: "review this".into(),
                is_task: true,
                is_wake: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.pid, None);
        assert!(!result.detail.contains("spawned"));
    }

    #[test]
    fn claude_wake_spawn_stops_before_the_call_when_the_budget_would_be_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::HubStore::open(dir.path()).unwrap();
        // Half a unit: one reserved provider call cannot fit, so the
        // gate pauses the agent and stops before any spawn.
        store.set_agent_budget("claude", 0.5).unwrap();
        let result = crate::inject_harness_with_store(
            &store,
            &HarnessInjectRequest {
                harness: "claude".into(),
                workspace: PathBuf::from("/tmp/workspace-claude-budget"),
                body: "wake up".into(),
                is_task: false,
                is_wake: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.status, "stopped");
        assert_eq!(result.pid, None);
        assert!(result.detail.contains("handoff"), "{}", result.detail);
        let status = store.get_budget("claude").unwrap().unwrap();
        assert!(status.paused);
    }

    #[test]
    fn claude_spawn_gate_allows_an_unmetered_agent() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::HubStore::open(dir.path()).unwrap();
        // No budget row for claude: the gate must allow (None) without
        // reserving anything, and without touching the spawn path.
        assert!(claude_spawn_gate(&store).unwrap().is_none());
    }

    #[test]
    fn codex_wake_spawn_stops_before_the_call_when_the_budget_would_be_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::HubStore::open(dir.path()).unwrap();
        store.set_agent_budget("codex", 0.5).unwrap();
        let result = crate::inject_harness_with_store(
            &store,
            &HarnessInjectRequest {
                harness: "chat".into(),
                workspace: PathBuf::from("/tmp/workspace-codex-budget"),
                body: "wake up".into(),
                is_task: false,
                is_wake: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.harness, "chat");
        assert_eq!(result.status, "stopped");
        assert_eq!(result.pid, None);
        assert!(result.detail.contains("Codex"), "{}", result.detail);
        assert!(result.detail.contains("handoff"), "{}", result.detail);
        let status = store.get_budget("codex").unwrap().unwrap();
        assert!(status.paused);
    }

    #[test]
    fn codex_spawn_gate_allows_an_unmetered_agent() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::HubStore::open(dir.path()).unwrap();
        assert!(codex_spawn_gate(&store).unwrap().is_none());
    }

    #[test]
    fn codex_task_delivery_is_not_spawn_gated() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::HubStore::open(dir.path()).unwrap();
        store.set_agent_budget("codex", 0.5).unwrap();
        let result = crate::inject_harness_with_store(
            &store,
            &HarnessInjectRequest {
                harness: "chat".into(),
                workspace: PathBuf::from("/tmp/workspace-codex-task-not-gated"),
                session_id: Some("session-1".into()),
                message_id: Some("message-1".into()),
                body: "review this".into(),
                is_task: true,
                is_wake: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.pid, None);
        let status = store.get_budget("codex").unwrap().unwrap();
        assert!(!status.paused);
    }
}

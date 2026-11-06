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

/// C6 / #84: a one-shot spawn is a provider call, so the same pre-call
/// budget decision the in-process path takes must run before the process
/// starts. `Ok(None)` = allowed (unmetered or reserved); `Ok(Some)` =
/// `status: stopped`, no pid, no process. Reserving consumes one unit.
/// Budget keys follow roster identities: Chat is `codex`, Vibe is `mistral`.
fn one_shot_spawn_gate(
    store: &crate::HubStore,
    harness: HarnessId,
) -> Result<Option<HarnessInjectResult>, HubError> {
    let (agent_id, shown, prose) = one_shot_budget_names(harness);
    let stamp = SliceStamp::registered(None, None, Some(agent_id.to_string()));
    match store.gate_provider_call(
        agent_id,
        DEFAULT_PROVIDER_CALL_UNITS,
        &stamp,
        &format!("{prose} one-shot harness run"),
        &format!("{prose} one-shot harness runs completed so far."),
        &format!("Further {prose} one-shot runs."),
        None,
    )? {
        ProviderCallGate::Unmetered | ProviderCallGate::Reserved { .. } => Ok(None),
        ProviderCallGate::Stopped { status, handoff } => {
            let detail = if handoff.is_some() {
                format!(
                    "{shown} reached its budget ({}/{} units); handoff written; no process was spawned",
                    status.spent_units, status.limit_units
                )
            } else {
                format!(
                    "{shown} is budget-paused ({}/{} units); resume_agent required; no process was spawned",
                    status.spent_units, status.limit_units
                )
            };
            Ok(Some(HarnessInjectResult {
                harness: harness.as_str().into(),
                pid: None,
                status: "stopped".into(),
                detail,
            }))
        }
    }
}

/// `(budget agent id, stopped-detail subject, handoff prose name)`.
/// Claude and Codex keep their existing subject wording. Every other
/// harness names itself with [`HarnessId::as_str`] in the same sentence.
fn one_shot_budget_names(harness: HarnessId) -> (&'static str, &'static str, &'static str) {
    match harness {
        HarnessId::Claude => ("claude", "claude", "Claude"),
        HarnessId::Chat => ("codex", "Codex", "Codex"),
        HarnessId::Vibe => ("mistral", "vibe", "Vibe"),
        other => {
            let name = other.as_str();
            (name, name, name)
        }
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

/// Hub tasks are delivered as `/goal [GOAL] …`. A Kimi slash command has to
/// stay the first token, so a set `kimi_mode` or a body that already starts
/// with `/` skips that wrapper. `deliver_kimi_task` then applies `/swarm` or
/// `/goal` itself. Wakes keep the Hub marker.
fn task_delivery_body(harness: HarnessId, request: &HarnessInjectRequest) -> String {
    let slash = request.body.trim().starts_with('/');
    let kimi_command =
        harness == HarnessId::Kimi && !request.is_wake && (request.kimi_mode.is_some() || slash);
    if kimi_command {
        request.body.clone()
    } else {
        crate::store::goal_delivery_body(&request.body)
    }
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
            body: task_delivery_body(harness, request),
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
    // C6 / #84: budget-stop every one-shot spawn before the process
    // starts (wake and plain sends). Task-only delivery returned above.
    // Bare `inject_harness` has no store and stays ungated.
    if let Some(store) = store {
        if let Some(stopped) = one_shot_spawn_gate(store, harness)? {
            return Ok(stopped);
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
#[path = "inject_tests.rs"]
mod tests;

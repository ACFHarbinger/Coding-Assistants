//! Hub task/wake injection dispatch (the inject half of the old
//! harness/mod.rs, split for the 500-LoC cap, #158). Task-only
//! sends to a live harness route through its provider bridge;
//! otherwise the task stays durably queued and never spawns a
//! replacement process.

use crate::HubError;

use super::spawn::spawn_explicit;
use super::{
    claude_spawn_args, codex_spawn_args, gemini_spawn_args, grok_spawn_args, opencode_spawn_args,
    vibe_spawn_args, HarnessId, HarnessInjectRequest, HarnessInjectResult,
};

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
    if !request.workspace.is_absolute() {
        return Err(HubError::Invalid("workspace path must be absolute".into()));
    }
    if request.is_task && !request.is_wake {
        if harness == HarnessId::Grok {
            if let Some(store) = store {
                return crate::bridge::grok::deliver_grok_task(store, request);
            }
        }
        if harness == HarnessId::Gemini {
            if let Some(store) = store {
                return crate::bridge::gemini::deliver_gemini_task(store, request);
            }
        }
        if harness == HarnessId::Claude {
            if let Some(store) = store {
                return crate::bridge::claude::deliver_claude_task(store, request);
            }
        }
        if harness == HarnessId::Chat {
            if let Some(store) = store {
                return crate::bridge::channels::chat::deliver_codex_task(store, request);
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
    let prompt = if request.is_task && request.is_wake {
        format!("[TASK] [WAKE] {}", request.body)
    } else if request.is_task {
        format!("[TASK] {}", request.body)
    } else if request.is_wake {
        format!("[WAKE] {}", request.body)
    } else {
        request.body.clone()
    };

    let args = match harness {
        HarnessId::Grok => grok_spawn_args(&request.workspace, &prompt)?,
        HarnessId::Chat => codex_spawn_args(&request.workspace, &prompt)?,
        HarnessId::Claude => claude_spawn_args(&request.workspace, &prompt)?,
        HarnessId::Gemini => gemini_spawn_args(&request.workspace, &prompt)?,
        HarnessId::OpenCode => opencode_spawn_args(&request.workspace, &prompt)?,
        HarnessId::Vibe => vibe_spawn_args(&request.workspace, &prompt)?,
    };

    let started = spawn_explicit(harness.executable(), &request.workspace, &args)?;
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
            },
        )
        .unwrap();
        assert_eq!(result.status, "unavailable");
        assert_eq!(result.pid, None);
        assert!(!result.detail.contains("spawned"));
    }
}

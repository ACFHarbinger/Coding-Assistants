//! Provider command selection and child ownership for the generic start API.

use crate::HubError;
use std::ffi::OsString;
use std::process::Child;

use super::spawn::{spawn_explicit, spawn_explicit_owned};
use super::{
    claude_spawn_args, codex_spawn_args, gemini_managed_spawn_args, grok_spawn_args,
    opencode_spawn_args, vibe_spawn_args, HarnessId, HarnessStartRequest, HarnessStartResult,
    DEFAULT_DEEPSEEK_MODEL, DEFAULT_OPENCODE_MODEL,
};

fn harness_command(
    request: &HarnessStartRequest,
) -> Result<(&'static str, Vec<OsString>), HubError> {
    let harness = HarnessId::parse(&request.harness)?;
    let model = request.model.as_deref().or(match harness {
        HarnessId::OpenCode => Some(DEFAULT_OPENCODE_MODEL),
        HarnessId::DeepSeek => Some(DEFAULT_DEEPSEEK_MODEL),
        _ => None,
    });
    let effort = request.effort.as_deref();
    let args = match harness {
        HarnessId::Grok => grok_spawn_args(&request.workspace, &request.prompt, model, effort)?,
        HarnessId::Chat => codex_spawn_args(&request.workspace, &request.prompt, model, effort)?,
        HarnessId::Claude => claude_spawn_args(&request.workspace, &request.prompt, model, effort)?,
        HarnessId::Gemini => gemini_managed_spawn_args(
            &request.workspace,
            &request.prompt,
            request.session_id.as_deref(),
            model,
            effort,
        )?,
        HarnessId::OpenCode => {
            opencode_spawn_args(&request.workspace, &request.prompt, model, effort)?
        }
        HarnessId::DeepSeek => opencode_spawn_args(
            &request.workspace,
            &request.prompt,
            Some(model.unwrap_or(DEFAULT_DEEPSEEK_MODEL)),
            effort,
        )?,
        HarnessId::Vibe => vibe_spawn_args(&request.workspace, &request.prompt, model, effort)?,
    };
    Ok((harness.executable(), args))
}

pub(crate) fn start_harness_owned(
    request: &HarnessStartRequest,
) -> Result<(HarnessStartResult, Child), HubError> {
    let (program, args) = harness_command(request)?;
    let child = spawn_explicit_owned(program, &request.workspace, &args)
        .map_err(|error| HubError::Invalid(format!("{program} unavailable: {error}")))?;
    let result = HarnessStartResult {
        harness: program.into(),
        pid: Some(child.id()),
        status: "started".into(),
        detail: format!("spawned {program} pid {}", child.id()),
    };
    Ok((result, child))
}

pub fn start_harness(request: &HarnessStartRequest) -> Result<HarnessStartResult, HubError> {
    let (program, args) = harness_command(request)?;
    spawn_explicit(program, &request.workspace, &args)
}
